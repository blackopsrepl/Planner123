use anyhow::{bail, Result};

use crate::{
    db,
    google::{
        events_api::{self, GoogleApiError},
        types::{google_create_id, local_event_insert_body, local_event_patch_body},
    },
    models::Calendar,
    sync::state::{self, OutboxOperation, GOOGLE_PROVIDER},
};

use super::{
    finalize_delete, persist_conflict, persist_push_error, persist_remote_success, wrap_local_error,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct PushReport {
    pub creates: usize,
    pub updates: usize,
    pub deletes: usize,
    pub conflicts: usize,
}

pub async fn push_calendar(
    client: &crate::google::auth::GoogleClient,
    calendar: &Calendar,
) -> Result<PushReport> {
    let google_calendar_id = calendar
        .google_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("calendar '{}' has no google_id", calendar.name))?;
    let access_token = client.refresh_access_token().await?;
    let outbox_entries = {
        let conn = db::open()?;
        state::load_outbox_entries(&conn, Some(GOOGLE_PROVIDER))?
            .into_iter()
            .filter(|entry| entry.calendar_id == calendar.id)
            .collect::<Vec<_>>()
    };

    let mut report = PushReport::default();
    for entry in outbox_entries {
        match process_entry(&access_token, google_calendar_id, calendar, &entry).await {
            Ok(OutboxOperation::Create) => report.creates += 1,
            Ok(OutboxOperation::Update) => report.updates += 1,
            Ok(OutboxOperation::Delete) => report.deletes += 1,
            Err(error) if error.is_precondition_failed() => {
                persist_conflict(&access_token, google_calendar_id, calendar, &entry).await?;
                report.conflicts += 1;
            }
            Err(error) if error.is_not_found() && entry.operation == OutboxOperation::Delete => {
                let conn = db::open()?;
                finalize_delete(&conn, &entry.event_id)?;
                report.deletes += 1;
            }
            Err(error) => {
                let conn = db::open()?;
                persist_push_error(&conn, &entry.event_id, &error)?;
                bail!(error);
            }
        }
    }

    Ok(report)
}

async fn process_entry(
    access_token: &str,
    google_calendar_id: &str,
    calendar: &Calendar,
    entry: &state::OutboxEntry,
) -> Result<OutboxOperation, GoogleApiError> {
    let Some(event) = ({
        let conn = db::open().map_err(wrap_local_error)?;
        db::get_event_including_deleted(&conn, &entry.event_id).map_err(wrap_local_error)?
    }) else {
        let conn = db::open().map_err(wrap_local_error)?;
        state::delete_outbox_entry(&conn, GOOGLE_PROVIDER, &entry.event_id)
            .map_err(wrap_local_error)?;
        return Ok(entry.operation.clone());
    };

    match entry.operation {
        OutboxOperation::Create => {
            let remote = match events_api::insert_event(
                access_token,
                google_calendar_id,
                &local_event_insert_body(&event).map_err(wrap_local_error)?,
            )
            .await
            {
                Ok(remote) => remote,
                Err(error) if error.status.as_u16() == 409 => {
                    events_api::get_event(
                        access_token,
                        google_calendar_id,
                        &google_create_id(&event),
                    )
                    .await?
                }
                Err(error) => return Err(error),
            };
            let conn = db::open().map_err(wrap_local_error)?;
            persist_remote_success(&conn, calendar, &event, &remote).map_err(wrap_local_error)?;
            Ok(OutboxOperation::Create)
        }
        OutboxOperation::Update => {
            let google_event_id = event.google_id.as_deref().ok_or_else(|| GoogleApiError {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: "cannot update a Google event without google_id".to_string(),
            })?;
            let remote = events_api::patch_event(
                access_token,
                google_calendar_id,
                google_event_id,
                &local_event_patch_body(&event).map_err(wrap_local_error)?,
                event.google_etag.as_deref(),
            )
            .await?;
            let conn = db::open().map_err(wrap_local_error)?;
            persist_remote_success(&conn, calendar, &event, &remote).map_err(wrap_local_error)?;
            Ok(OutboxOperation::Update)
        }
        OutboxOperation::Delete => {
            let google_event_id = event.google_id.as_deref().ok_or_else(|| GoogleApiError {
                status: reqwest::StatusCode::BAD_REQUEST,
                message: "cannot delete a Google event without google_id".to_string(),
            })?;
            events_api::delete_event(
                access_token,
                google_calendar_id,
                google_event_id,
                event.google_etag.as_deref(),
            )
            .await?;
            let conn = db::open().map_err(wrap_local_error)?;
            finalize_delete(&conn, &event.id).map_err(wrap_local_error)?;
            Ok(OutboxOperation::Delete)
        }
    }
}
