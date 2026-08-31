use super::*;

#[test]
fn google_sync_without_google_calendars_returns_conflict() {
    let (_temp, conn) = open_test_db();
    let cli = Cli {
        command: Command::Google {
            action: GoogleCommand::Sync(GoogleSyncArgs { calendar_id: None }),
        },
    };

    let backend = FakeGoogleSyncBackend::with_default(Ok((0, 0)));
    let err = execute_with_backend(&conn, cli, &backend).unwrap_err();
    assert_eq!(err.code, "conflict");
    assert_eq!(err.message, "no google calendars found to sync");
}

#[test]
fn google_sync_reports_missing_credentials() {
    let (_temp, conn) = open_test_db();
    let now = timestamp_now();
    let google_calendar = models::Calendar {
        id: Uuid::new_v4().to_string(),
        name: "Google Work".to_string(),
        color: "#00aaff".to_string(),
        source: models::CalendarSource::Google,
        google_id: Some("google-work".to_string()),
        visible: true,
        position: 1,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    db::insert_calendar(&conn, &google_calendar).unwrap();

    let cli = Cli {
        command: Command::Google {
            action: GoogleCommand::Sync(GoogleSyncArgs { calendar_id: None }),
        },
    };
    let backend = FakeGoogleSyncBackend::with_default(Err(CliError::external(
        "google credentials are not configured in keyring",
    )));
    let err = execute_with_backend(&conn, cli, &backend).unwrap_err();
    assert_eq!(err.code, "external_error");
}

#[test]
fn google_sync_filters_to_selected_calendar() {
    let (_temp, conn) = open_test_db();
    let now = timestamp_now();
    let google_calendar = models::Calendar {
        id: Uuid::new_v4().to_string(),
        name: "Google Work".to_string(),
        color: "#00aaff".to_string(),
        source: models::CalendarSource::Google,
        google_id: Some("google-work".to_string()),
        visible: true,
        position: 1,
        created_at: now.clone(),
        updated_at: now.clone(),
        deleted_at: None,
    };
    let other_google_calendar = models::Calendar {
        id: Uuid::new_v4().to_string(),
        name: "Google Personal".to_string(),
        color: "#ffaa00".to_string(),
        source: models::CalendarSource::Google,
        google_id: Some("google-personal".to_string()),
        visible: true,
        position: 2,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    db::insert_calendar(&conn, &google_calendar).unwrap();
    db::insert_calendar(&conn, &other_google_calendar).unwrap();

    let cli = Cli {
        command: Command::Google {
            action: GoogleCommand::Sync(GoogleSyncArgs {
                calendar_id: Some(google_calendar.id.clone()),
            }),
        },
    };
    let backend = FakeGoogleSyncBackend::with_default(Ok((7, 8)))
        .with_calendar_result(&google_calendar.id, Ok((2, 3)));
    let value = execute_with_backend(&conn, cli, &backend).unwrap();
    assert_eq!(value["status"], "ok");
    assert_eq!(value["data"]["calendars_synced"], 1);
    assert_eq!(value["data"]["events_added"], 2);
    assert_eq!(value["data"]["events_updated"], 3);
}
