use std::collections::HashMap;

use super::*;
use tempfile::TempDir;

fn open_test_db() -> (TempDir, Connection) {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");
    let conn = db::open_at(&db_path).unwrap();
    (temp, conn)
}

fn seed_calendar(conn: &Connection, name: &str) -> models::Calendar {
    let now = timestamp_now();
    let calendar = models::Calendar {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        color: "#82FB9C".to_string(),
        source: models::CalendarSource::Local,
        google_id: None,
        visible: true,
        position: 0,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    db::insert_calendar(conn, &calendar).unwrap();
    calendar
}

fn seed_project(conn: &Connection, name: &str) -> models::Project {
    let now = timestamp_now();
    let project = models::Project {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        color: "#ffaa00".to_string(),
        description: None,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    db::insert_project(conn, &project).unwrap();
    project
}

fn seed_event(
    conn: &Connection,
    calendar_id: &str,
    project_id: Option<String>,
    title: &str,
) -> models::Event {
    let now = timestamp_now();
    let event = models::Event {
        id: Uuid::new_v4().to_string(),
        calendar_id: calendar_id.to_string(),
        project_id,
        title: title.to_string(),
        description: None,
        location: None,
        start_at: "2026-03-30 09:00:00".to_string(),
        end_at: "2026-03-30 10:00:00".to_string(),
        all_day: false,
        rrule: None,
        google_id: None,
        google_etag: None,
        reminder_minutes: None,
        timezone: "UTC".to_string(),
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    db::insert_event(conn, &event).unwrap();
    event
}

struct FakeGoogleSyncBackend {
    default_result: Result<(usize, usize), CliError>,
    results_by_calendar_id: HashMap<String, Result<(usize, usize), CliError>>,
}

impl FakeGoogleSyncBackend {
    fn with_default(result: Result<(usize, usize), CliError>) -> Self {
        Self {
            default_result: result,
            results_by_calendar_id: HashMap::new(),
        }
    }

    fn with_calendar_result(
        mut self,
        calendar_id: &str,
        result: Result<(usize, usize), CliError>,
    ) -> Self {
        self.results_by_calendar_id
            .insert(calendar_id.to_string(), result);
        self
    }
}

impl GoogleSyncBackend for FakeGoogleSyncBackend {
    fn sync_calendar(
        &self,
        _runtime: &tokio::runtime::Runtime,
        _conn: &Connection,
        calendar: &models::Calendar,
    ) -> Result<google::sync::SyncCalendarReport, CliError> {
        let (added, updated) = self
            .results_by_calendar_id
            .get(&calendar.id)
            .cloned()
            .unwrap_or_else(|| self.default_result.clone())?;
        Ok(google::sync::SyncCalendarReport {
            events_added: added,
            events_updated: updated,
            ..Default::default()
        })
    }
}

mod behavior;
mod google_sync;
