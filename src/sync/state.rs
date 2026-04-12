use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Calendar, CalendarSource, Event};

pub const GOOGLE_PROVIDER: &str = "google";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Clean,
    PendingCreate,
    PendingUpdate,
    PendingDelete,
    Conflicted,
    Error,
}

impl std::fmt::Display for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "clean"),
            Self::PendingCreate => write!(f, "pending_create"),
            Self::PendingUpdate => write!(f, "pending_update"),
            Self::PendingDelete => write!(f, "pending_delete"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl SyncState {
    fn from_db(value: &str) -> Self {
        match value {
            "pending_create" => Self::PendingCreate,
            "pending_update" => Self::PendingUpdate,
            "pending_delete" => Self::PendingDelete,
            "conflicted" => Self::Conflicted,
            "error" => Self::Error,
            _ => Self::Clean,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxOperation {
    Create,
    Update,
    Delete,
}

impl std::fmt::Display for OutboxOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create => write!(f, "create"),
            Self::Update => write!(f, "update"),
            Self::Delete => write!(f, "delete"),
        }
    }
}

impl OutboxOperation {
    fn from_db(value: &str) -> Self {
        match value {
            "create" => Self::Create,
            "delete" => Self::Delete,
            _ => Self::Update,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStatus {
    Pending,
    Resolved,
}

impl std::fmt::Display for ConflictResolutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Resolved => write!(f, "resolved"),
        }
    }
}

impl ConflictResolutionStatus {
    fn from_db(value: &str) -> Self {
        match value {
            "resolved" => Self::Resolved,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    KeepLocal,
    KeepRemote,
}

impl std::fmt::Display for ConflictResolutionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepLocal => write!(f, "keep_local"),
            Self::KeepRemote => write!(f, "keep_remote"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSyncState {
    pub calendar_id: String,
    pub google_access_role: Option<String>,
    pub writable: bool,
    pub last_synced_at: Option<String>,
    pub last_sync_error_code: Option<String>,
    pub last_sync_error_message: Option<String>,
    pub pending_outbox: usize,
    pub pending_conflicts: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSyncState {
    pub event_id: String,
    pub sync_state: SyncState,
    pub last_synced_at: Option<String>,
    pub last_push_attempt_at: Option<String>,
    pub last_remote_modified_at: Option<String>,
    pub last_sync_error_code: Option<String>,
    pub last_sync_error_message: Option<String>,
    pub remote_payload: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: String,
    pub provider: String,
    pub calendar_id: String,
    pub event_id: String,
    pub operation: OutboxOperation,
    pub enqueued_at: String,
    pub attempt_count: i64,
    pub last_attempt_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: String,
    pub event_id: String,
    pub calendar_id: String,
    pub local_snapshot: String,
    pub remote_snapshot: String,
    pub remote_etag: Option<String>,
    pub detected_at: String,
    pub resolution_status: ConflictResolutionStatus,
    pub resolution_strategy: Option<ConflictResolutionStrategy>,
    pub resolved_at: Option<String>,
}

pub fn load_calendar_sync_state(
    conn: &Connection,
    calendar_id: &str,
) -> Result<Option<CalendarSyncState>> {
    conn.query_row(
        "SELECT css.calendar_id, css.google_access_role, css.writable, css.last_synced_at,
                css.last_sync_error_code, css.last_sync_error_message,
                (SELECT COUNT(*) FROM sync_outbox
                 WHERE provider = 'google' AND calendar_id = css.calendar_id),
                (SELECT COUNT(*) FROM sync_conflicts
                 WHERE calendar_id = css.calendar_id
                   AND resolution_status = 'pending'),
                css.created_at, css.updated_at
         FROM calendar_sync_state css
         WHERE css.calendar_id = ?1",
        [calendar_id],
        |row| {
            Ok(CalendarSyncState {
                calendar_id: row.get(0)?,
                google_access_role: row.get(1)?,
                writable: row.get::<_, i64>(2)? != 0,
                last_synced_at: row.get(3)?,
                last_sync_error_code: row.get(4)?,
                last_sync_error_message: row.get(5)?,
                pending_outbox: row.get::<_, i64>(6)? as usize,
                pending_conflicts: row.get::<_, i64>(7)? as usize,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn load_calendar_sync_states(conn: &Connection) -> Result<Vec<CalendarSyncState>> {
    let mut stmt = conn.prepare(
        "SELECT css.calendar_id, css.google_access_role, css.writable, css.last_synced_at,
                css.last_sync_error_code, css.last_sync_error_message,
                (SELECT COUNT(*) FROM sync_outbox
                 WHERE provider = 'google' AND calendar_id = css.calendar_id),
                (SELECT COUNT(*) FROM sync_conflicts
                 WHERE calendar_id = css.calendar_id
                   AND resolution_status = 'pending'),
                css.created_at, css.updated_at
         FROM calendar_sync_state css
         ORDER BY css.calendar_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CalendarSyncState {
            calendar_id: row.get(0)?,
            google_access_role: row.get(1)?,
            writable: row.get::<_, i64>(2)? != 0,
            last_synced_at: row.get(3)?,
            last_sync_error_code: row.get(4)?,
            last_sync_error_message: row.get(5)?,
            pending_outbox: row.get::<_, i64>(6)? as usize,
            pending_conflicts: row.get::<_, i64>(7)? as usize,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn upsert_calendar_sync_state(conn: &Connection, state: &CalendarSyncState) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO calendar_sync_state
             (calendar_id, google_access_role, writable, last_synced_at,
              last_sync_error_code, last_sync_error_message, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(calendar_id) DO UPDATE SET
             google_access_role = excluded.google_access_role,
             writable = excluded.writable,
             last_synced_at = excluded.last_synced_at,
             last_sync_error_code = excluded.last_sync_error_code,
             last_sync_error_message = excluded.last_sync_error_message,
             updated_at = excluded.updated_at",
        params![
            state.calendar_id,
            state.google_access_role,
            state.writable as i64,
            state.last_synced_at,
            state.last_sync_error_code,
            state.last_sync_error_message,
            if state.created_at.is_empty() {
                now.clone()
            } else {
                state.created_at.clone()
            },
            now,
        ],
    )?;
    Ok(())
}

pub fn load_event_sync_state(conn: &Connection, event_id: &str) -> Result<Option<EventSyncState>> {
    conn.query_row(
        "SELECT event_id, sync_state, last_synced_at, last_push_attempt_at,
                last_remote_modified_at, last_sync_error_code, last_sync_error_message,
                remote_payload, created_at, updated_at
         FROM event_sync_state
         WHERE event_id = ?1",
        [event_id],
        |row| {
            let sync_state: String = row.get(1)?;
            Ok(EventSyncState {
                event_id: row.get(0)?,
                sync_state: SyncState::from_db(&sync_state),
                last_synced_at: row.get(2)?,
                last_push_attempt_at: row.get(3)?,
                last_remote_modified_at: row.get(4)?,
                last_sync_error_code: row.get(5)?,
                last_sync_error_message: row.get(6)?,
                remote_payload: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn upsert_event_sync_state(conn: &Connection, state: &EventSyncState) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO event_sync_state
             (event_id, sync_state, last_synced_at, last_push_attempt_at,
              last_remote_modified_at, last_sync_error_code, last_sync_error_message,
              remote_payload, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(event_id) DO UPDATE SET
             sync_state = excluded.sync_state,
             last_synced_at = excluded.last_synced_at,
             last_push_attempt_at = excluded.last_push_attempt_at,
             last_remote_modified_at = excluded.last_remote_modified_at,
             last_sync_error_code = excluded.last_sync_error_code,
             last_sync_error_message = excluded.last_sync_error_message,
             remote_payload = excluded.remote_payload,
             updated_at = excluded.updated_at",
        params![
            state.event_id,
            state.sync_state.to_string(),
            state.last_synced_at,
            state.last_push_attempt_at,
            state.last_remote_modified_at,
            state.last_sync_error_code,
            state.last_sync_error_message,
            state.remote_payload,
            if state.created_at.is_empty() {
                now.clone()
            } else {
                state.created_at.clone()
            },
            now,
        ],
    )?;
    Ok(())
}

pub fn delete_event_sync_state(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM event_sync_state WHERE event_id = ?1",
        [event_id],
    )?;
    Ok(())
}

pub fn load_outbox_entries(conn: &Connection, provider: Option<&str>) -> Result<Vec<OutboxEntry>> {
    let sql = if provider.is_some() {
        "SELECT id, provider, calendar_id, event_id, operation, enqueued_at,
                attempt_count, last_attempt_at, last_error_code, last_error_message
         FROM sync_outbox
         WHERE provider = ?1
         ORDER BY enqueued_at, id"
    } else {
        "SELECT id, provider, calendar_id, event_id, operation, enqueued_at,
                attempt_count, last_attempt_at, last_error_code, last_error_message
         FROM sync_outbox
         ORDER BY enqueued_at, id"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(provider) = provider {
        stmt.query_map([provider], map_outbox_row)?
    } else {
        stmt.query_map([], map_outbox_row)?
    };
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn get_outbox_entry(
    conn: &Connection,
    provider: &str,
    event_id: &str,
) -> Result<Option<OutboxEntry>> {
    conn.query_row(
        "SELECT id, provider, calendar_id, event_id, operation, enqueued_at,
                attempt_count, last_attempt_at, last_error_code, last_error_message
         FROM sync_outbox
         WHERE provider = ?1 AND event_id = ?2",
        params![provider, event_id],
        map_outbox_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn upsert_outbox_entry(conn: &Connection, entry: &OutboxEntry) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO sync_outbox
             (id, provider, calendar_id, event_id, operation, enqueued_at,
              attempt_count, last_attempt_at, last_error_code, last_error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(provider, event_id) DO UPDATE SET
             calendar_id = excluded.calendar_id,
             operation = excluded.operation,
             enqueued_at = excluded.enqueued_at,
             last_error_code = NULL,
             last_error_message = NULL",
        params![
            if entry.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                entry.id.clone()
            },
            entry.provider,
            entry.calendar_id,
            entry.event_id,
            entry.operation.to_string(),
            if entry.enqueued_at.is_empty() {
                now
            } else {
                entry.enqueued_at.clone()
            },
            entry.attempt_count,
            entry.last_attempt_at,
            entry.last_error_code,
            entry.last_error_message,
        ],
    )?;
    Ok(())
}

pub fn delete_outbox_entry(conn: &Connection, provider: &str, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM sync_outbox WHERE provider = ?1 AND event_id = ?2",
        params![provider, event_id],
    )?;
    Ok(())
}

pub fn load_conflicts(conn: &Connection, pending_only: bool) -> Result<Vec<SyncConflict>> {
    let sql = if pending_only {
        "SELECT id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
                detected_at, resolution_status, resolution_strategy, resolved_at
         FROM sync_conflicts
         WHERE resolution_status = 'pending'
         ORDER BY detected_at DESC, id DESC"
    } else {
        "SELECT id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
                detected_at, resolution_status, resolution_strategy, resolved_at
         FROM sync_conflicts
         ORDER BY detected_at DESC, id DESC"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], map_conflict_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn get_conflict(conn: &Connection, conflict_id: &str) -> Result<Option<SyncConflict>> {
    conn.query_row(
        "SELECT id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
                detected_at, resolution_status, resolution_strategy, resolved_at
         FROM sync_conflicts
         WHERE id = ?1",
        [conflict_id],
        map_conflict_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn insert_conflict(conn: &Connection, conflict: &SyncConflict) -> Result<()> {
    conn.execute(
        "INSERT INTO sync_conflicts
             (id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
              detected_at, resolution_status, resolution_strategy, resolved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            if conflict.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                conflict.id.clone()
            },
            conflict.event_id,
            conflict.calendar_id,
            conflict.local_snapshot,
            conflict.remote_snapshot,
            conflict.remote_etag,
            if conflict.detected_at.is_empty() {
                now_timestamp()
            } else {
                conflict.detected_at.clone()
            },
            conflict.resolution_status.to_string(),
            conflict
                .resolution_strategy
                .as_ref()
                .map(std::string::ToString::to_string),
            conflict.resolved_at,
        ],
    )?;
    Ok(())
}

pub fn resolve_conflict(
    conn: &Connection,
    conflict_id: &str,
    strategy: ConflictResolutionStrategy,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_conflicts
         SET resolution_status = 'resolved',
             resolution_strategy = ?2,
             resolved_at = ?3
         WHERE id = ?1",
        params![conflict_id, strategy.to_string(), now_timestamp()],
    )?;
    Ok(())
}

pub fn delete_conflicts_for_event(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute("DELETE FROM sync_conflicts WHERE event_id = ?1", [event_id])?;
    Ok(())
}

pub fn clear_calendar_sync_state(conn: &Connection, calendar_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM calendar_sync_state WHERE calendar_id = ?1",
        [calendar_id],
    )?;
    Ok(())
}

pub fn clear_event_sync_tracking(conn: &Connection, event_id: &str) -> Result<()> {
    delete_outbox_entry(conn, GOOGLE_PROVIDER, event_id)?;
    delete_event_sync_state(conn, event_id)?;
    delete_conflicts_for_event(conn, event_id)?;
    Ok(())
}

pub fn google_calendar_writable(conn: &Connection, calendar: &Calendar) -> Result<bool> {
    if calendar.source != CalendarSource::Google {
        return Ok(true);
    }
    Ok(load_calendar_sync_state(conn, &calendar.id)?
        .map(|state| state.writable)
        .unwrap_or(true))
}

pub fn record_local_event_save(
    conn: &Connection,
    calendar: &Calendar,
    previous_event: Option<&Event>,
    event: &Event,
) -> Result<()> {
    if calendar.source != CalendarSource::Google {
        if event.google_id.is_none() {
            clear_event_sync_tracking(conn, &event.id)?;
        }
        return Ok(());
    }

    if !google_calendar_writable(conn, calendar)? {
        bail!("calendar '{}' is read-only", calendar.name);
    }

    if let Some(previous_event) = previous_event {
        if previous_event.calendar_id != event.calendar_id && previous_event.google_id.is_some() {
            bail!("moving synced Google events between calendars is not supported yet");
        }
    }

    let existing_outbox = get_outbox_entry(conn, GOOGLE_PROVIDER, &event.id)?;
    let operation = match existing_outbox.as_ref().map(|entry| &entry.operation) {
        Some(OutboxOperation::Create) => OutboxOperation::Create,
        _ if event.google_id.is_none() => OutboxOperation::Create,
        _ => OutboxOperation::Update,
    };
    let sync_state = match operation {
        OutboxOperation::Create => SyncState::PendingCreate,
        OutboxOperation::Update => SyncState::PendingUpdate,
        OutboxOperation::Delete => SyncState::PendingDelete,
    };

    let existing_state = load_event_sync_state(conn, &event.id)?;
    upsert_event_sync_state(
        conn,
        &EventSyncState {
            event_id: event.id.clone(),
            sync_state,
            last_synced_at: existing_state
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_push_attempt_at: existing_state
                .as_ref()
                .and_then(|state| state.last_push_attempt_at.clone()),
            last_remote_modified_at: existing_state
                .as_ref()
                .and_then(|state| state.last_remote_modified_at.clone()),
            last_sync_error_code: None,
            last_sync_error_message: None,
            remote_payload: existing_state.and_then(|state| state.remote_payload),
            created_at: String::new(),
            updated_at: String::new(),
        },
    )?;
    upsert_outbox_entry(
        conn,
        &OutboxEntry {
            id: String::new(),
            provider: GOOGLE_PROVIDER.to_string(),
            calendar_id: calendar.id.clone(),
            event_id: event.id.clone(),
            operation,
            enqueued_at: String::new(),
            attempt_count: 0,
            last_attempt_at: None,
            last_error_code: None,
            last_error_message: None,
        },
    )?;
    Ok(())
}

pub fn record_local_event_delete(
    conn: &Connection,
    calendar: &Calendar,
    event: &Event,
) -> Result<()> {
    if calendar.source != CalendarSource::Google {
        clear_event_sync_tracking(conn, &event.id)?;
        return Ok(());
    }

    if !google_calendar_writable(conn, calendar)? {
        bail!("calendar '{}' is read-only", calendar.name);
    }

    if event.google_id.is_none() {
        clear_event_sync_tracking(conn, &event.id)?;
        return Ok(());
    }

    let existing_state = load_event_sync_state(conn, &event.id)?;
    upsert_event_sync_state(
        conn,
        &EventSyncState {
            event_id: event.id.clone(),
            sync_state: SyncState::PendingDelete,
            last_synced_at: existing_state
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_push_attempt_at: existing_state
                .as_ref()
                .and_then(|state| state.last_push_attempt_at.clone()),
            last_remote_modified_at: existing_state
                .as_ref()
                .and_then(|state| state.last_remote_modified_at.clone()),
            last_sync_error_code: None,
            last_sync_error_message: None,
            remote_payload: existing_state.and_then(|state| state.remote_payload),
            created_at: String::new(),
            updated_at: String::new(),
        },
    )?;
    upsert_outbox_entry(
        conn,
        &OutboxEntry {
            id: String::new(),
            provider: GOOGLE_PROVIDER.to_string(),
            calendar_id: calendar.id.clone(),
            event_id: event.id.clone(),
            operation: OutboxOperation::Delete,
            enqueued_at: String::new(),
            attempt_count: 0,
            last_attempt_at: None,
            last_error_code: None,
            last_error_message: None,
        },
    )?;
    Ok(())
}

fn map_outbox_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OutboxEntry> {
    let operation: String = row.get(4)?;
    Ok(OutboxEntry {
        id: row.get(0)?,
        provider: row.get(1)?,
        calendar_id: row.get(2)?,
        event_id: row.get(3)?,
        operation: OutboxOperation::from_db(&operation),
        enqueued_at: row.get(5)?,
        attempt_count: row.get(6)?,
        last_attempt_at: row.get(7)?,
        last_error_code: row.get(8)?,
        last_error_message: row.get(9)?,
    })
}

fn map_conflict_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncConflict> {
    let resolution_status: String = row.get(7)?;
    let resolution_strategy: Option<String> = row.get(8)?;
    Ok(SyncConflict {
        id: row.get(0)?,
        event_id: row.get(1)?,
        calendar_id: row.get(2)?,
        local_snapshot: row.get(3)?,
        remote_snapshot: row.get(4)?,
        remote_etag: row.get(5)?,
        detected_at: row.get(6)?,
        resolution_status: ConflictResolutionStatus::from_db(&resolution_status),
        resolution_strategy: resolution_strategy.as_deref().map(|value| match value {
            "keep_remote" => ConflictResolutionStrategy::KeepRemote,
            _ => ConflictResolutionStrategy::KeepLocal,
        }),
        resolved_at: row.get(9)?,
    })
}

fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{
        get_outbox_entry, insert_conflict, load_calendar_sync_state, load_event_sync_state,
        upsert_calendar_sync_state, CalendarSyncState, ConflictResolutionStatus, OutboxOperation,
        SyncConflict, SyncState, GOOGLE_PROVIDER,
    };
    use crate::{
        db,
        models::{CalendarSource, Event},
    };

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
        (temp, conn)
    }

    #[test]
    fn repeated_google_updates_coalesce_to_one_create_operation() {
        let (_temp, conn) = open_test_db();
        let calendar = crate::calendar_service::create_calendar(
            &conn,
            crate::calendar_service::CreateCalendarInput {
                name: "Google".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("primary@example.com".to_string()),
                visible: true,
                position: None,
            },
        )
        .unwrap();

        let event = crate::event_service::save_event(
            &conn,
            Event::new(
                calendar.id.clone(),
                "Planning",
                "2026-04-12 09:00:00",
                "2026-04-12 10:00:00",
                "UTC",
            ),
            true,
        )
        .unwrap();

        let outbox = get_outbox_entry(&conn, GOOGLE_PROVIDER, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(outbox.operation, OutboxOperation::Create);

        let sync_state = load_event_sync_state(&conn, &event.id).unwrap().unwrap();
        assert_eq!(sync_state.sync_state, SyncState::PendingCreate);
    }

    #[test]
    fn read_only_google_calendar_blocks_local_changes() {
        let (_temp, conn) = open_test_db();
        let calendar = crate::calendar_service::create_calendar(
            &conn,
            crate::calendar_service::CreateCalendarInput {
                name: "Read Only".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("readonly@example.com".to_string()),
                visible: true,
                position: None,
            },
        )
        .unwrap();
        upsert_calendar_sync_state(
            &conn,
            &CalendarSyncState {
                calendar_id: calendar.id.clone(),
                google_access_role: Some("reader".to_string()),
                writable: false,
                last_synced_at: None,
                last_sync_error_code: None,
                last_sync_error_message: None,
                pending_outbox: 0,
                pending_conflicts: 0,
                created_at: String::new(),
                updated_at: String::new(),
            },
        )
        .unwrap();

        let err = crate::event_service::save_event(
            &conn,
            Event::new(
                calendar.id.clone(),
                "Blocked",
                "2026-04-12 09:00:00",
                "2026-04-12 10:00:00",
                "UTC",
            ),
            true,
        )
        .unwrap_err();
        assert!(err.to_string().contains("read-only"));
    }

    #[test]
    fn delete_supersedes_pending_create_when_no_remote_event_exists() {
        let (_temp, conn) = open_test_db();
        let calendar = crate::calendar_service::create_calendar(
            &conn,
            crate::calendar_service::CreateCalendarInput {
                name: "Google".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("primary@example.com".to_string()),
                visible: true,
                position: None,
            },
        )
        .unwrap();

        let event = crate::event_service::save_event(
            &conn,
            Event::new(
                calendar.id.clone(),
                "Draft",
                "2026-04-12 09:00:00",
                "2026-04-12 10:00:00",
                "UTC",
            ),
            true,
        )
        .unwrap();
        crate::event_service::delete_event(&conn, &event.id).unwrap();

        assert!(get_outbox_entry(&conn, GOOGLE_PROVIDER, &event.id)
            .unwrap()
            .is_none());
        assert!(load_event_sync_state(&conn, &event.id).unwrap().is_none());
    }

    #[test]
    fn calendar_sync_state_reports_pending_counts() {
        let (_temp, conn) = open_test_db();
        let calendar = crate::calendar_service::create_calendar(
            &conn,
            crate::calendar_service::CreateCalendarInput {
                name: "Google".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("primary@example.com".to_string()),
                visible: true,
                position: None,
            },
        )
        .unwrap();
        upsert_calendar_sync_state(
            &conn,
            &CalendarSyncState {
                calendar_id: calendar.id.clone(),
                google_access_role: Some("owner".to_string()),
                writable: true,
                last_synced_at: None,
                last_sync_error_code: None,
                last_sync_error_message: None,
                pending_outbox: 0,
                pending_conflicts: 0,
                created_at: String::new(),
                updated_at: String::new(),
            },
        )
        .unwrap();
        let event = crate::event_service::save_event(
            &conn,
            Event::new(
                calendar.id.clone(),
                "Planning",
                "2026-04-12 09:00:00",
                "2026-04-12 10:00:00",
                "UTC",
            ),
            true,
        )
        .unwrap();
        insert_conflict(
            &conn,
            &SyncConflict {
                id: String::new(),
                event_id: event.id,
                calendar_id: calendar.id.clone(),
                local_snapshot: "{}".to_string(),
                remote_snapshot: "{}".to_string(),
                remote_etag: Some("\"etag\"".to_string()),
                detected_at: String::new(),
                resolution_status: ConflictResolutionStatus::Pending,
                resolution_strategy: None,
                resolved_at: None,
            },
        )
        .unwrap();

        let state = load_calendar_sync_state(&conn, &calendar.id)
            .unwrap()
            .unwrap();
        assert_eq!(state.pending_outbox, 1);
        assert_eq!(state.pending_conflicts, 1);
    }
}
