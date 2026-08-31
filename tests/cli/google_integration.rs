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
