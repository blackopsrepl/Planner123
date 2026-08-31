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
