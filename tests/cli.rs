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

#[test]
fn unknown_flag_returns_json_error() {
    let temp = TempDir::new().unwrap();
    let output = cli_command(&temp)
        .args(["events", "list", "--bogus"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let err = read_json(&output.stderr);
    assert_eq!(err["status"], "error");
    assert_eq!(err["code"], "invalid_arguments");
}

#[test]
fn invalid_timestamp_returns_json_error() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);
    let output = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "Planning",
            "--start-at",
            "bad-timestamp",
            "--end-at",
            "2026-03-30 10:00:00",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let err = read_json(&output.stderr);
    assert_eq!(err["status"], "error");
    assert_eq!(err["code"], "invalid_arguments");
}

#[test]
fn missing_resource_returns_not_found_json_error() {
    let temp = TempDir::new().unwrap();
    let output = cli_command(&temp)
        .args(["events", "get", "missing-event"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let err = read_json(&output.stderr);
    assert_eq!(err["status"], "error");
    assert_eq!(err["code"], "not_found");
}

#[test]
fn invalid_enum_returns_json_error() {
    let temp = TempDir::new().unwrap();
    let output = cli_command(&temp)
        .args([
            "dependencies",
            "create",
            "--from-event-id",
            "a",
            "--to-event-id",
            "b",
            "--dependency-type",
            "invalid",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let err = read_json(&output.stderr);
    assert_eq!(err["status"], "error");
    assert_eq!(err["code"], "invalid_arguments");
}

#[test]
fn agent_wrapper_script_exists() {
    assert!(Path::new("scripts/solverforge-calendar-cli").exists());
}

#[test]
fn event_crud_works_against_isolated_db() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);

    let created = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "Planning",
            "--start-at",
            "2026-03-30 09:00:00",
            "--end-at",
            "2026-03-30 10:00:00",
        ])
        .output()
        .unwrap();
    assert!(created.status.success());
    let created_json = read_json(&created.stdout);
    let event_id = created_json["data"]["id"].as_str().unwrap().to_string();

    let updated = cli_command(&temp)
        .args([
            "events",
            "update",
            &event_id,
            "--location",
            "HQ",
            "--reminder-minutes",
            "30",
        ])
        .output()
        .unwrap();
    assert!(updated.status.success());
    let updated_json = read_json(&updated.stdout);
    assert_eq!(updated_json["data"]["location"], "HQ");
    assert_eq!(updated_json["data"]["reminder_minutes"], 30);

    let listed = cli_command(&temp)
        .args(["events", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed_json = read_json(&listed.stdout);
    assert_eq!(listed_json["data"].as_array().unwrap().len(), 1);
}

#[test]
fn calendar_crud_and_source_validation_work() {
    let temp = TempDir::new().unwrap();

    let invalid_local = cli_command(&temp)
        .args([
            "calendars",
            "create",
            "--name",
            "Local",
            "--color",
            "#123456",
            "--google-id",
            "google-local",
        ])
        .output()
        .unwrap();
    assert_eq!(invalid_local.status.code(), Some(1));
    let invalid_local_json = read_json(&invalid_local.stderr);
    assert_eq!(invalid_local_json["code"], "validation_error");

    let created = cli_command(&temp)
        .args([
            "calendars",
            "create",
            "--name",
            "Work",
            "--color",
            "#50f872",
            "--source",
            "google",
            "--google-id",
            "work@example.com",
        ])
        .output()
        .unwrap();
    assert!(created.status.success());
    let created_json = read_json(&created.stdout);
    let calendar_id = created_json["data"]["id"].as_str().unwrap().to_string();

    let fetched = cli_command(&temp)
        .args(["calendars", "get", &calendar_id])
        .output()
        .unwrap();
    assert!(fetched.status.success());
    let fetched_json = read_json(&fetched.stdout);
    assert_eq!(fetched_json["data"]["google_id"], "work@example.com");

    let updated = cli_command(&temp)
        .args([
            "calendars",
            "update",
            &calendar_id,
            "--source",
            "local",
            "--name",
            "Personal",
        ])
        .output()
        .unwrap();
    assert!(updated.status.success());
    let updated_json = read_json(&updated.stdout);
    assert_eq!(updated_json["data"]["source"], "local");
    assert!(updated_json["data"]["google_id"].is_null());
    assert_eq!(updated_json["data"]["name"], "Personal");
}

#[test]
fn duplicate_google_calendar_returns_conflict_json_error() {
    let temp = TempDir::new().unwrap();

    let first = cli_command(&temp)
        .args([
            "calendars",
            "create",
            "--name",
            "Work",
            "--color",
            "#50f872",
            "--source",
            "google",
            "--google-id",
            "work@example.com",
        ])
        .output()
        .unwrap();
    assert!(first.status.success());

    let duplicate = cli_command(&temp)
        .args([
            "calendars",
            "create",
            "--name",
            "Work Again",
            "--color",
            "#ffaa00",
            "--source",
            "google",
            "--google-id",
            "work@example.com",
        ])
        .output()
        .unwrap();

    assert_eq!(duplicate.status.code(), Some(1));
    let duplicate_json = read_json(&duplicate.stderr);
    assert_eq!(duplicate_json["status"], "error");
    assert_eq!(duplicate_json["code"], "conflict");
}

#[test]
fn project_crud_works() {
    let temp = TempDir::new().unwrap();

    let created = cli_command(&temp)
        .args([
            "projects",
            "create",
            "--name",
            "Launch",
            "--color",
            "#ffaa00",
            "--description",
            "Initial launch",
        ])
        .output()
        .unwrap();
    assert!(created.status.success());
    let created_json = read_json(&created.stdout);
    let project_id = created_json["data"]["id"].as_str().unwrap().to_string();

    let updated = cli_command(&temp)
        .args([
            "projects",
            "update",
            &project_id,
            "--description",
            "",
            "--name",
            "Launch v2",
        ])
        .output()
        .unwrap();
    assert!(updated.status.success());
    let updated_json = read_json(&updated.stdout);
    assert_eq!(updated_json["data"]["name"], "Launch v2");
    assert!(updated_json["data"]["description"].is_null());

    let listed = cli_command(&temp)
        .args(["projects", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed_json = read_json(&listed.stdout);
    assert_eq!(listed_json["data"].as_array().unwrap().len(), 1);
}

#[test]
fn events_support_range_lists_and_clear_conflicts() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);

    let project = cli_command(&temp)
        .args([
            "projects", "create", "--name", "Launch", "--color", "#ffaa00",
        ])
        .output()
        .unwrap();
    assert!(project.status.success());
    let project_json = read_json(&project.stdout);
    let project_id = project_json["data"]["id"].as_str().unwrap().to_string();

    let first = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--project-id",
            &project_id,
            "--title",
            "Planning",
            "--description",
            "Discuss plan",
            "--start-at",
            "2026-03-30 09:00:00",
            "--end-at",
            "2026-03-30 10:00:00",
        ])
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_json = read_json(&first.stdout);
    let event_id = first_json["data"]["id"].as_str().unwrap().to_string();

    let second = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "Retro",
            "--start-at",
            "2026-03-31 09:00:00",
            "--end-at",
            "2026-03-31 10:00:00",
        ])
        .output()
        .unwrap();
    assert!(second.status.success());

    let ranged = cli_command(&temp)
        .args([
            "events",
            "list",
            "--from",
            "2026-03-30 00:00:00",
            "--to",
            "2026-03-30 23:59:59",
        ])
        .output()
        .unwrap();
    assert!(ranged.status.success());
    let ranged_json = read_json(&ranged.stdout);
    assert_eq!(ranged_json["data"].as_array().unwrap().len(), 1);

    let cleared = cli_command(&temp)
        .args([
            "events",
            "update",
            &event_id,
            "--clear-description",
            "--clear-project-id",
        ])
        .output()
        .unwrap();
    assert!(cleared.status.success());
    let cleared_json = read_json(&cleared.stdout);
    assert!(cleared_json["data"]["description"].is_null());
    assert!(cleared_json["data"]["project_id"].is_null());

    let conflicting = cli_command(&temp)
        .args([
            "events",
            "update",
            &event_id,
            "--description",
            "new",
            "--clear-description",
        ])
        .output()
        .unwrap();
    assert_eq!(conflicting.status.code(), Some(1));
    let conflicting_json = read_json(&conflicting.stderr);
    assert_eq!(conflicting_json["code"], "validation_error");
}

#[test]
fn ical_import_creates_events_and_reports_skipped_shapes() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);
    let ics_path = temp.path().join("import.ics");
    std::fs::write(
        &ics_path,
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Planning\r\nDESCRIPTION:Discuss\\nPlan\r\nLOCATION:HQ\r\nDTSTART;TZID=Europe/Rome:20260412T090000\r\nDTEND;TZID=Europe/Rome:20260412T100000\r\nRRULE:FREQ=WEEKLY;COUNT=2\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20260420\r\nDTEND;VALUE=DATE:20260422\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Exception\r\nRECURRENCE-ID:20260412T090000Z\r\nDTSTART:20260412T090000Z\r\nDTEND:20260412T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
    )
    .unwrap();

    let imported = cli_command(&temp)
        .args([
            "ical",
            "import",
            "--calendar-id",
            &calendar_id,
            "--path",
            ics_path.to_str().unwrap(),
            "--timezone",
            "Europe/Rome",
        ])
        .output()
        .unwrap();
    assert!(imported.status.success());
    let imported_json = read_json(&imported.stdout);
    assert_eq!(imported_json["data"]["imported"], 2);
    assert_eq!(imported_json["data"]["skipped"], 1);
    assert!(imported_json["data"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning.as_str().unwrap().contains("RECURRENCE-ID")));

    let listed = cli_command(&temp)
        .args(["events", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed_json = read_json(&listed.stdout);
    assert_eq!(listed_json["data"].as_array().unwrap().len(), 2);
}

#[test]
fn dependency_crud_and_validation_work() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);

    let first = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "A",
            "--start-at",
            "2026-03-30 09:00:00",
            "--end-at",
            "2026-03-30 10:00:00",
        ])
        .output()
        .unwrap();
    let first_json = read_json(&first.stdout);
    let event_a = first_json["data"]["id"].as_str().unwrap().to_string();

    let second = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "B",
            "--start-at",
            "2026-03-30 11:00:00",
            "--end-at",
            "2026-03-30 12:00:00",
        ])
        .output()
        .unwrap();
    let second_json = read_json(&second.stdout);
    let event_b = second_json["data"]["id"].as_str().unwrap().to_string();

    let created = cli_command(&temp)
        .args([
            "dependencies",
            "create",
            "--from-event-id",
            &event_a,
            "--to-event-id",
            &event_b,
            "--dependency-type",
            "related",
        ])
        .output()
        .unwrap();
    assert!(created.status.success());
    let created_json = read_json(&created.stdout);
    let dependency_id = created_json["data"]["id"].as_str().unwrap().to_string();

    let duplicate = cli_command(&temp)
        .args([
            "dependencies",
            "create",
            "--from-event-id",
            &event_a,
            "--to-event-id",
            &event_b,
            "--dependency-type",
            "related",
        ])
        .output()
        .unwrap();
    assert_eq!(duplicate.status.code(), Some(1));
    let duplicate_json = read_json(&duplicate.stderr);
    assert_eq!(duplicate_json["code"], "conflict");

    let updated = cli_command(&temp)
        .args([
            "dependencies",
            "update",
            &dependency_id,
            "--dependency-type",
            "blocks",
        ])
        .output()
        .unwrap();
    assert!(updated.status.success());
    let updated_json = read_json(&updated.stdout);
    assert_eq!(updated_json["data"]["dependency_type"], "blocks");

    let listed = cli_command(&temp)
        .args(["dependencies", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed_json = read_json(&listed.stdout);
    assert_eq!(listed_json["data"].as_array().unwrap().len(), 1);

    let deleted = cli_command(&temp)
        .args(["dependencies", "delete", &dependency_id])
        .output()
        .unwrap();
    assert!(deleted.status.success());

    let empty = cli_command(&temp)
        .args(["dependencies", "list"])
        .output()
        .unwrap();
    assert!(empty.status.success());
    let empty_json = read_json(&empty.stdout);
    assert!(empty_json["data"].as_array().unwrap().is_empty());
}

#[test]
fn calendar_delete_requires_explicit_cascade_flag() {
    let temp = TempDir::new().unwrap();

    let created_calendar = cli_command(&temp)
        .args([
            "calendars",
            "create",
            "--name",
            "Work",
            "--color",
            "#50f872",
        ])
        .output()
        .unwrap();
    assert!(created_calendar.status.success());
    let calendar_json = read_json(&created_calendar.stdout);
    let calendar_id = calendar_json["data"]["id"].as_str().unwrap().to_string();

    let created_event = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "Standup",
            "--start-at",
            "2026-03-30 09:00:00",
            "--end-at",
            "2026-03-30 09:30:00",
        ])
        .output()
        .unwrap();
    assert!(created_event.status.success());

    let blocked_delete = cli_command(&temp)
        .args(["calendars", "delete", &calendar_id])
        .output()
        .unwrap();
    assert_eq!(blocked_delete.status.code(), Some(1));
    let blocked_json = read_json(&blocked_delete.stderr);
    assert_eq!(blocked_json["code"], "conflict");

    let allowed_delete = cli_command(&temp)
        .args(["calendars", "delete", &calendar_id, "--cascade-events"])
        .output()
        .unwrap();
    assert!(allowed_delete.status.success());

    let listed = cli_command(&temp)
        .args(["events", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed_json = read_json(&listed.stdout);
    assert!(listed_json["data"].as_array().unwrap().is_empty());
}

#[test]
fn calendar_delete_rejects_last_active_calendar_via_binary() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);

    let blocked_delete = cli_command(&temp)
        .args(["calendars", "delete", &calendar_id])
        .output()
        .unwrap();
    assert_eq!(blocked_delete.status.code(), Some(1));
    let blocked_json = read_json(&blocked_delete.stderr);
    assert_eq!(blocked_json["code"], "conflict");
}

#[test]
fn google_sync_reports_missing_calendar_filter() {
    let temp = TempDir::new().unwrap();
    let output = cli_command(&temp)
        .args(["google", "sync", "--calendar-id", "missing-calendar"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let err = read_json(&output.stderr);
    assert_eq!(err["code"], "not_found");
}

#[test]
fn google_sync_runs_through_binary_with_test_override() {
    let temp = TempDir::new().unwrap();
    let calendar_id = create_google_calendar(&temp, "WorkGoogle");
    let override_json = format!(
        "{{\"by_calendar_id\":{{\"{}\":{{\"added\":4,\"updated\":2}}}}}}",
        calendar_id
    );

    let output = cli_command(&temp)
        .env("SOLVERFORGE_CALENDAR_TEST_GOOGLE_SYNC", override_json)
        .args(["google", "sync", "--calendar-id", &calendar_id])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json = read_json(&output.stdout);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["calendars_synced"], 1);
    assert_eq!(json["data"]["events_added"], 4);
    assert_eq!(json["data"]["events_updated"], 2);
}

#[test]
fn google_auth_status_reports_disconnected_with_isolated_keyring() {
    let temp = TempDir::new().unwrap();
    let output = cli_command(&temp)
        .env(
            "SOLVERFORGE_CALENDAR_TEST_KEYRING_SERVICE",
            unique_keyring_service(),
        )
        .args(["google", "auth", "status"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json = read_json(&output.stdout);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["state"], "disconnected");
    assert!(!json["data"]["has_refresh_token"].as_bool().unwrap());
}

#[test]
fn google_calendar_discovery_and_import_use_test_override() {
    let temp = TempDir::new().unwrap();
    let discovery = r##"[
      {
        "google_id": "writer@example.com",
        "name": "Writer",
        "color": "#123456",
        "primary": true,
        "access_role": "writer",
        "writable": true
      },
      {
        "google_id": "reader@example.com",
        "name": "Reader",
        "color": "#654321",
        "primary": false,
        "access_role": "reader",
        "writable": false
      }
    ]"##;

    let discovered = cli_command(&temp)
        .env("SOLVERFORGE_CALENDAR_TEST_GOOGLE_DISCOVERY", discovery)
        .args(["google", "calendars", "discover"])
        .output()
        .unwrap();
    assert!(discovered.status.success());
    let discovered_json = read_json(&discovered.stdout);
    assert!(discovered_json["data"][0]["writable"].as_bool().unwrap());
    assert_eq!(discovered_json["data"][1]["access_role"], "reader");
    assert!(!discovered_json["data"][1]["imported"].as_bool().unwrap());

    let imported = cli_command(&temp)
        .env("SOLVERFORGE_CALENDAR_TEST_GOOGLE_DISCOVERY", discovery)
        .args([
            "google",
            "calendars",
            "import",
            "--google-id",
            "reader@example.com",
        ])
        .output()
        .unwrap();
    assert!(imported.status.success());
    let imported_json = read_json(&imported.stdout);
    assert_eq!(
        imported_json["data"]["calendar"]["google_id"],
        "reader@example.com"
    );
    assert_eq!(
        imported_json["data"]["sync_state"]["google_access_role"],
        "reader"
    );
    assert!(!imported_json["data"]["sync_state"]["writable"]
        .as_bool()
        .unwrap());
}

#[test]
fn google_sync_status_reports_pending_outbox() {
    let temp = TempDir::new().unwrap();
    let calendar_id = create_google_calendar(&temp, "StatusGoogle");

    let created = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "Planning",
            "--start-at",
            "2026-04-12 09:00:00",
            "--end-at",
            "2026-04-12 10:00:00",
        ])
        .output()
        .unwrap();
    assert!(created.status.success());

    let status = cli_command(&temp)
        .args(["google", "sync-status", "--calendar-id", &calendar_id])
        .output()
        .unwrap();
    assert!(status.status.success());
    let json = read_json(&status.stdout);
    assert_eq!(json["data"][0]["pending_outbox"], 1);
}

#[test]
fn google_conflicts_can_be_listed_and_resolved_keep_local() {
    let temp = TempDir::new().unwrap();
    let calendar_id = create_google_calendar(&temp, "ConflictGoogle");
    let conn = db::open_at(db_path_for(&temp)).unwrap();

    let event = event_service::save_event(
        &conn,
        Event::new(
            calendar_id.clone(),
            "Local title",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        ),
        true,
    )
    .unwrap();

    state::upsert_event_sync_state(
        &conn,
        &EventSyncState {
            event_id: event.id.clone(),
            sync_state: SyncState::Conflicted,
            last_synced_at: None,
            last_push_attempt_at: None,
            last_remote_modified_at: None,
            last_sync_error_code: Some("412".to_string()),
            last_sync_error_message: Some("remote event changed before update".to_string()),
            remote_payload: None,
            created_at: String::new(),
            updated_at: String::new(),
        },
    )
    .unwrap();

    let conflict_id = Uuid::new_v4().to_string();
    state::insert_conflict(
        &conn,
        &SyncConflict {
            id: conflict_id.clone(),
            event_id: event.id.clone(),
            calendar_id: calendar_id.clone(),
            local_snapshot: serde_json::to_string(&event).unwrap(),
            remote_snapshot: serde_json::to_string(&GoogleEvent {
                id: Some("remote-1".to_string()),
                etag: Some("\"etag-2\"".to_string()),
                status: Some("confirmed".to_string()),
                summary: Some("Remote title".to_string()),
                description: None,
                location: None,
                start: Some(GoogleEventTime {
                    date_time: Some("2026-04-12T09:00:00Z".to_string()),
                    date: None,
                    time_zone: Some("UTC".to_string()),
                }),
                end: Some(GoogleEventTime {
                    date_time: Some("2026-04-12T10:00:00Z".to_string()),
                    date: None,
                    time_zone: Some("UTC".to_string()),
                }),
                recurrence: None,
                updated: Some("2026-04-12T08:00:00Z".to_string()),
                recurring_event_id: None,
                original_start_time: None,
            })
            .unwrap(),
            remote_etag: Some("\"etag-2\"".to_string()),
            detected_at: String::new(),
            resolution_status: ConflictResolutionStatus::Pending,
            resolution_strategy: None,
            resolved_at: None,
        },
    )
    .unwrap();

    let listed = cli_command(&temp)
        .args(["google", "conflicts", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed_json = read_json(&listed.stdout);
    assert_eq!(listed_json["data"][0]["id"], conflict_id);

    let resolved = cli_command(&temp)
        .args([
            "google",
            "conflicts",
            "resolve",
            &conflict_id,
            "--strategy",
            "keep-local",
        ])
        .output()
        .unwrap();
    assert!(resolved.status.success());
    let resolved_json = read_json(&resolved.stdout);
    assert_eq!(resolved_json["data"]["resolution_status"], "resolved");
    assert_eq!(resolved_json["data"]["resolution_strategy"], "keep_local");

    let outbox = state::get_outbox_entry(&conn, state::GOOGLE_PROVIDER, &event.id)
        .unwrap()
        .unwrap();
    assert_eq!(outbox.operation.to_string(), "update");
    let refreshed = db::get_event_including_deleted(&conn, &event.id)
        .unwrap()
        .unwrap();
    assert_eq!(refreshed.google_id.as_deref(), Some("remote-1"));
    assert_eq!(refreshed.google_etag.as_deref(), Some("\"etag-2\""));
}
