use assert_cmd::Command;
use serde_json::Value;
use solverforge_calendar::{
    db, event_service,
    google::types::{GoogleEvent, GoogleEventTime},
    models::Event,
    sync::state::{self, ConflictResolutionStatus, EventSyncState, SyncConflict, SyncState},
};
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

fn cli_command(temp: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("solverforge-calendar-cli").unwrap();
    cmd.env("XDG_DATA_HOME", temp.path());
    cmd
}

fn read_json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

fn first_calendar_id(temp: &TempDir) -> String {
    let calendars = cli_command(temp)
        .args(["calendars", "list"])
        .output()
        .unwrap();
    assert!(calendars.status.success());
    let calendars_json = read_json(&calendars.stdout);
    calendars_json["data"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn create_google_calendar(temp: &TempDir, name: &str) -> String {
    let created = cli_command(temp)
        .args([
            "calendars",
            "create",
            "--name",
            name,
            "--color",
            "#50f872",
            "--source",
            "google",
            "--google-id",
            &format!("{}@example.com", name.to_lowercase()),
        ])
        .output()
        .unwrap();
    assert!(created.status.success());
    let created_json = read_json(&created.stdout);
    created_json["data"]["id"].as_str().unwrap().to_string()
}

fn unique_keyring_service() -> String {
    format!("solverforge-calendar-test-{}", Uuid::new_v4().simple())
}

fn db_path_for(temp: &TempDir) -> std::path::PathBuf {
    temp.path().join("solverforge").join("calendar.db")
}

include!("cli/planner_and_errors.rs");
include!("cli/calendar_and_projects.rs");
include!("cli/events_and_ical.rs");
include!("cli/dependencies.rs");
include!("cli/google_sync.rs");
include!("cli/google_integration.rs");
