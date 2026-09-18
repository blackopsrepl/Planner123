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
        .env("PLANNER123_TEST_GOOGLE_SYNC", override_json)
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
        .env("PLANNER123_TEST_KEYRING_SERVICE", unique_keyring_service())
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
        .env("PLANNER123_TEST_GOOGLE_DISCOVERY", discovery)
        .args(["google", "calendars", "discover"])
        .output()
        .unwrap();
    assert!(discovered.status.success());
    let discovered_json = read_json(&discovered.stdout);
    assert!(discovered_json["data"][0]["writable"].as_bool().unwrap());
    assert_eq!(discovered_json["data"][1]["access_role"], "reader");
    assert!(!discovered_json["data"][1]["imported"].as_bool().unwrap());

    let imported = cli_command(&temp)
        .env("PLANNER123_TEST_GOOGLE_DISCOVERY", discovery)
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
