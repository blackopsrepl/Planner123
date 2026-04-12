use anyhow::{bail, Result};

use crate::{
    db,
    google::{
        events_api::{self, GoogleApiError},
        types::{local_event_insert_body, local_event_patch_body, remote_updated_at, GoogleEvent},
    },
    models::Calendar,
    sync::{
        pull,
        state::{
            self, ConflictResolutionStatus, EventSyncState, OutboxOperation, SyncConflict,
            SyncState, GOOGLE_PROVIDER,
        },
    },
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
            let remote = events_api::insert_event(
                access_token,
                google_calendar_id,
                &local_event_insert_body(&event).map_err(wrap_local_error)?,
            )
            .await?;
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

fn persist_remote_success(
    conn: &rusqlite::Connection,
    calendar: &Calendar,
    local_event: &crate::models::Event,
    remote_event: &GoogleEvent,
) -> Result<()> {
    let merged = pull::merge_remote_event(&calendar.id, remote_event, Some(local_event))?;
    db::update_event_from_sync(conn, &merged)?;
    state::delete_outbox_entry(conn, GOOGLE_PROVIDER, &merged.id)?;
    state::upsert_event_sync_state(
        conn,
        &EventSyncState {
            event_id: merged.id.clone(),
            sync_state: SyncState::Clean,
            last_synced_at: Some(now_timestamp()),
            last_push_attempt_at: Some(now_timestamp()),
            last_remote_modified_at: remote_updated_at(remote_event),
            last_sync_error_code: None,
            last_sync_error_message: None,
            remote_payload: Some(serde_json::to_string(remote_event)?),
            created_at: state::load_event_sync_state(conn, &merged.id)?
                .map(|state| state.created_at)
                .unwrap_or_default(),
            updated_at: String::new(),
        },
    )?;
    Ok(())
}

async fn persist_conflict(
    access_token: &str,
    google_calendar_id: &str,
    calendar: &Calendar,
    entry: &state::OutboxEntry,
) -> Result<()> {
    let conn = db::open()?;
    let event = db::get_event_including_deleted(&conn, &entry.event_id)?.ok_or_else(|| {
        anyhow::anyhow!(
            "event '{}' disappeared before conflict handling",
            entry.event_id
        )
    })?;
    let remote = events_api::get_event(
        access_token,
        google_calendar_id,
        event
            .google_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("event '{}' has no google_id", event.id))?,
    )
    .await?;
    state::delete_outbox_entry(&conn, GOOGLE_PROVIDER, &event.id)?;
    state::delete_conflicts_for_event(&conn, &event.id)?;
    state::insert_conflict(
        &conn,
        &SyncConflict {
            id: String::new(),
            event_id: event.id.clone(),
            calendar_id: calendar.id.clone(),
            local_snapshot: serde_json::to_string(&event)?,
            remote_snapshot: serde_json::to_string(&remote)?,
            remote_etag: remote.etag.clone(),
            detected_at: String::new(),
            resolution_status: ConflictResolutionStatus::Pending,
            resolution_strategy: None,
            resolved_at: None,
        },
    )?;
    state::upsert_event_sync_state(
        &conn,
        &EventSyncState {
            event_id: event.id.clone(),
            sync_state: SyncState::Conflicted,
            last_synced_at: state::load_event_sync_state(&conn, &event.id)?
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_push_attempt_at: Some(now_timestamp()),
            last_remote_modified_at: remote_updated_at(&remote),
            last_sync_error_code: Some("412".to_string()),
            last_sync_error_message: Some("remote event changed before update".to_string()),
            remote_payload: Some(serde_json::to_string(&remote)?),
            created_at: state::load_event_sync_state(&conn, &event.id)?
                .map(|state| state.created_at)
                .unwrap_or_default(),
            updated_at: String::new(),
        },
    )?;
    Ok(())
}

fn finalize_delete(conn: &rusqlite::Connection, event_id: &str) -> Result<()> {
    state::delete_outbox_entry(conn, GOOGLE_PROVIDER, event_id)?;
    if let Some(existing_state) = state::load_event_sync_state(conn, event_id)? {
        state::upsert_event_sync_state(
            conn,
            &EventSyncState {
                event_id: event_id.to_string(),
                sync_state: SyncState::Clean,
                last_synced_at: Some(now_timestamp()),
                last_push_attempt_at: Some(now_timestamp()),
                last_remote_modified_at: existing_state.last_remote_modified_at,
                last_sync_error_code: None,
                last_sync_error_message: None,
                remote_payload: existing_state.remote_payload,
                created_at: existing_state.created_at,
                updated_at: String::new(),
            },
        )?;
    }
    Ok(())
}

fn persist_push_error(
    conn: &rusqlite::Connection,
    event_id: &str,
    error: &GoogleApiError,
) -> Result<()> {
    let existing_state = state::load_event_sync_state(conn, event_id)?;
    state::upsert_event_sync_state(
        conn,
        &EventSyncState {
            event_id: event_id.to_string(),
            sync_state: SyncState::Error,
            last_synced_at: existing_state
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_push_attempt_at: Some(now_timestamp()),
            last_remote_modified_at: existing_state
                .as_ref()
                .and_then(|state| state.last_remote_modified_at.clone()),
            last_sync_error_code: Some(error.status.as_u16().to_string()),
            last_sync_error_message: Some(error.message.clone()),
            remote_payload: existing_state.and_then(|state| state.remote_payload),
            created_at: state::load_event_sync_state(conn, event_id)?
                .map(|state| state.created_at)
                .unwrap_or_default(),
            updated_at: String::new(),
        },
    )?;
    Ok(())
}

fn wrap_local_error(error: impl std::fmt::Display) -> GoogleApiError {
    GoogleApiError {
        status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        message: error.to_string(),
    }
}

fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
