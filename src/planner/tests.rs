use super::*;
use chrono::{Datelike, Timelike};
use tempfile::TempDir;

fn connection() -> (TempDir, Connection, String) {
    let temp = TempDir::new().unwrap();
    let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
    let calendar_id = db::load_calendars(&conn).unwrap()[0].id.clone();
    (temp, conn, calendar_id)
}

fn task(calendar_id: String, title: &str) -> CreateTaskInput {
    CreateTaskInput {
        title: title.into(),
        duration_minutes: 60,
        target_calendar_id: calendar_id,
        project_id: None,
        priority: TaskPriority::Normal,
        cognitive_load: CognitiveLoad::Medium,
        earliest_at: None,
        deadline_kind: DeadlineKind::None,
        deadline_at: None,
    }
}

fn configure_utc_workweek(conn: &Connection) {
    let mut availability = BTreeMap::new();
    for day in ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] {
        availability.insert(
            day.into(),
            vec![TimeWindow {
                start: "08:00".into(),
                end: "18:00".into(),
            }],
        );
    }
    update_settings(
        conn,
        SettingsUpdate {
            timezone: Some("UTC".into()),
            availability: Some(Availability(availability)),
            ..Default::default()
        },
    )
    .unwrap();
}

mod configuration;
mod dependencies;
mod end_to_end;
mod lifecycle;
mod scheduling;
