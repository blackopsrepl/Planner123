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
