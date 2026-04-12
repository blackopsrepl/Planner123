use anyhow::Result;

use crate::{
    db,
    google::{
        events_api::{self, GoogleApiError},
        types::{
            google_event_to_local, is_cancelled, is_recurring_exception, remote_updated_at,
            GoogleEvent,
        },
    },
    models::{Calendar, Event},
    sync::state::{self, EventSyncState, SyncState, GOOGLE_PROVIDER},
};

#[derive(Debug, Default, Clone, Copy)]
pub struct PullReport {
    pub added: usize,
    pub updated: usize,
}

pub async fn pull_calendar(
    client: &crate::google::auth::GoogleClient,
    calendar: &Calendar,
) -> Result<PullReport> {
    let google_calendar_id = calendar
        .google_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("calendar '{}' has no google_id", calendar.name))?;
    let access_token = client.refresh_access_token().await?;
    let sync_token = {
        let conn = db::open()?;
        db::get_sync_token(&conn, &calendar.id)?
    };

    match pull_with_token(
        &access_token,
        google_calendar_id,
        calendar,
        sync_token.as_deref(),
    )
    .await
    {
        Ok(report) => Ok(report),
        Err(error) if sync_token.is_some() && error.is_sync_token_expired() => {
            let conn = db::open()?;
            db::delete_sync_token(&conn, &calendar.id)?;
            pull_with_token(&access_token, google_calendar_id, calendar, None)
                .await
                .map_err(Into::into)
        }
        Err(error) => Err(error.into()),
    }
}

pub fn merge_remote_event(
    calendar_id: &str,
    remote_event: &GoogleEvent,
    existing: Option<&Event>,
) -> Result<Event> {
    let mut merged = google_event_to_local(calendar_id, remote_event)?;
    if let Some(existing) = existing {
        merged.id = existing.id.clone();
        merged.project_id = existing.project_id.clone();
        merged.reminder_minutes = existing.reminder_minutes;
        merged.created_at = existing.created_at.clone();
    }
    merged.deleted_at = None;
    Ok(merged)
}

async fn pull_with_token(
    access_token: &str,
    google_calendar_id: &str,
    calendar: &Calendar,
    sync_token: Option<&str>,
) -> Result<PullReport, GoogleApiError> {
    let mut report = PullReport::default();
    let mut page_token: Option<String> = None;
    let next_sync_token = loop {
        let page = events_api::list_events(
            access_token,
            google_calendar_id,
            sync_token,
            page_token.as_deref(),
        )
        .await?;
        for remote_event in &page.items {
            let existed_before = remote_event
                .id
                .as_deref()
                .map(|google_id| {
                    let conn = db::open()?;
                    db::get_event_by_google_id(&conn, &calendar.id, google_id, true)
                        .map(|event| event.is_some())
                })
                .transpose()
                .map_err(|error| GoogleApiError {
                    status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    message: error.to_string(),
                })?
                .unwrap_or(false);
            apply_remote_event(calendar, remote_event).map_err(|error| GoogleApiError {
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                message: error.to_string(),
            })?;
            if !is_cancelled(remote_event) && !is_recurring_exception(remote_event) {
                if existed_before {
                    report.updated += 1;
                } else {
                    report.added += 1;
                }
            }
        }
        page_token = page.next_page_token;
        if page_token.is_none() {
            break page.next_sync_token;
        }
    };

    if let Some(next_sync_token) = next_sync_token {
        let conn = db::open().map_err(|error| GoogleApiError {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        })?;
        db::upsert_sync_token(&conn, &calendar.id, &next_sync_token).map_err(|error| {
            GoogleApiError {
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                message: error.to_string(),
            }
        })?;
    }

    Ok(report)
}

fn apply_remote_event(calendar: &Calendar, remote_event: &GoogleEvent) -> Result<()> {
    if is_recurring_exception(remote_event) {
        return Ok(());
    }

    let conn = db::open()?;
    let google_id = match remote_event.id.as_deref() {
        Some(google_id) => google_id,
        None => return Ok(()),
    };
    let existing = db::get_event_by_google_id(&conn, &calendar.id, google_id, true)?;

    if is_cancelled(remote_event) {
        if let Some(existing) = existing {
            let sync_state = state::load_event_sync_state(&conn, &existing.id)?;
            if matches!(
                sync_state.as_ref().map(|state| &state.sync_state),
                Some(SyncState::PendingCreate | SyncState::PendingUpdate | SyncState::Conflicted)
            ) {
                return Ok(());
            }
            db::soft_delete_event(&conn, &existing.id)?;
            state::delete_outbox_entry(&conn, GOOGLE_PROVIDER, &existing.id)?;
            if let Some(sync_state) = sync_state {
                state::upsert_event_sync_state(
                    &conn,
                    &EventSyncState {
                        event_id: existing.id,
                        sync_state: SyncState::Clean,
                        last_synced_at: Some(now_timestamp()),
                        last_push_attempt_at: sync_state.last_push_attempt_at,
                        last_remote_modified_at: remote_updated_at(remote_event),
                        last_sync_error_code: None,
                        last_sync_error_message: None,
                        remote_payload: Some(serde_json::to_string(remote_event)?),
                        created_at: sync_state.created_at,
                        updated_at: String::new(),
                    },
                )?;
            }
        }
        return Ok(());
    }

    let sync_state = existing
        .as_ref()
        .map(|existing| state::load_event_sync_state(&conn, &existing.id))
        .transpose()?
        .flatten();
    if matches!(
        sync_state.as_ref().map(|state| &state.sync_state),
        Some(
            SyncState::PendingCreate
                | SyncState::PendingUpdate
                | SyncState::PendingDelete
                | SyncState::Conflicted
        )
    ) {
        return Ok(());
    }

    let merged = merge_remote_event(&calendar.id, remote_event, existing.as_ref())?;
    if let Some(existing) = existing.as_ref() {
        db::update_event_from_sync(&conn, &merged)?;
        state::delete_outbox_entry(&conn, GOOGLE_PROVIDER, &existing.id)?;
    } else {
        db::insert_event(&conn, &merged)?;
    }

    state::upsert_event_sync_state(
        &conn,
        &EventSyncState {
            event_id: merged.id.clone(),
            sync_state: SyncState::Clean,
            last_synced_at: Some(now_timestamp()),
            last_push_attempt_at: sync_state
                .as_ref()
                .and_then(|state| state.last_push_attempt_at.clone()),
            last_remote_modified_at: remote_updated_at(remote_event),
            last_sync_error_code: None,
            last_sync_error_message: None,
            remote_payload: Some(serde_json::to_string(remote_event)?),
            created_at: sync_state.map(|state| state.created_at).unwrap_or_default(),
            updated_at: String::new(),
        },
    )?;
    Ok(())
}

fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
