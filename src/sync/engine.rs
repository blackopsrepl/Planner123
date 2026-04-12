use anyhow::Result;

use crate::{
    db,
    models::Calendar,
    sync::{
        pull, push,
        state::{self, CalendarSyncState},
    },
};

#[derive(Debug, Default, Clone, Copy)]
pub struct SyncCalendarReport {
    pub events_added: usize,
    pub events_updated: usize,
    pub pushed_creates: usize,
    pub pushed_updates: usize,
    pub pushed_deletes: usize,
    pub conflicts_detected: usize,
}

pub async fn sync_calendar(
    client: &crate::google::auth::GoogleClient,
    calendar: &Calendar,
) -> Result<SyncCalendarReport> {
    let push_report = push::push_calendar(client, calendar).await?;
    let pull_report = pull::pull_calendar(client, calendar).await?;
    let conn = db::open()?;
    mark_calendar_sync_success(&conn, calendar)?;
    Ok(SyncCalendarReport {
        events_added: pull_report.added,
        events_updated: pull_report.updated,
        pushed_creates: push_report.creates,
        pushed_updates: push_report.updates,
        pushed_deletes: push_report.deletes,
        conflicts_detected: push_report.conflicts,
    })
}

pub fn sync_status(conn: &rusqlite::Connection, calendar: &Calendar) -> Result<serde_json::Value> {
    let sync_state = state::load_calendar_sync_state(conn, &calendar.id)?;
    let pending_outbox = state::load_outbox_entries(conn, Some(state::GOOGLE_PROVIDER))?
        .into_iter()
        .filter(|entry| entry.calendar_id == calendar.id)
        .count();
    let pending_conflicts = state::load_conflicts(conn, true)?
        .into_iter()
        .filter(|conflict| conflict.calendar_id == calendar.id)
        .count();

    Ok(serde_json::json!({
        "calendar_id": calendar.id,
        "calendar_name": calendar.name,
        "google_id": calendar.google_id,
        "last_synced_at": sync_state.as_ref().and_then(|state| state.last_synced_at.clone()),
        "last_sync_error_code": sync_state.as_ref().and_then(|state| state.last_sync_error_code.clone()),
        "last_sync_error_message": sync_state.as_ref().and_then(|state| state.last_sync_error_message.clone()),
        "pending_outbox": pending_outbox,
        "pending_conflicts": pending_conflicts,
    }))
}

fn mark_calendar_sync_success(conn: &rusqlite::Connection, calendar: &Calendar) -> Result<()> {
    let existing = state::load_calendar_sync_state(conn, &calendar.id)?;
    state::upsert_calendar_sync_state(
        conn,
        &CalendarSyncState {
            calendar_id: calendar.id.clone(),
            google_access_role: existing
                .as_ref()
                .and_then(|state| state.google_access_role.clone()),
            writable: existing
                .as_ref()
                .map(|state| state.writable)
                .unwrap_or(true),
            last_synced_at: Some(now_timestamp()),
            last_sync_error_code: None,
            last_sync_error_message: None,
            created_at: existing.map(|state| state.created_at).unwrap_or_default(),
            updated_at: String::new(),
        },
    )?;
    Ok(())
}

pub fn mark_calendar_sync_error(
    conn: &rusqlite::Connection,
    calendar: &Calendar,
    message: &str,
) -> Result<()> {
    let existing = state::load_calendar_sync_state(conn, &calendar.id)?;
    state::upsert_calendar_sync_state(
        conn,
        &CalendarSyncState {
            calendar_id: calendar.id.clone(),
            google_access_role: existing
                .as_ref()
                .and_then(|state| state.google_access_role.clone()),
            writable: existing
                .as_ref()
                .map(|state| state.writable)
                .unwrap_or(true),
            last_synced_at: existing
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_sync_error_code: Some("sync_failed".to_string()),
            last_sync_error_message: Some(message.to_string()),
            created_at: existing.map(|state| state.created_at).unwrap_or_default(),
            updated_at: String::new(),
        },
    )?;
    Ok(())
}

fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
