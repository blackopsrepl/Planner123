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
