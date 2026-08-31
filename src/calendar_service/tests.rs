mod tests {
    use tempfile::TempDir;

    use super::*;

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");
        let conn = db::open_at(&db_path).unwrap();
        (temp, conn)
    }

    #[test]
    fn create_google_calendar_requires_google_id() {
        let (_temp, conn) = open_test_db();
        let err = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: None,
                visible: true,
                position: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            CalendarServiceError::Validation("google calendars require a google_id".to_string())
        );
    }

    #[test]
    fn import_google_calendar_rejects_duplicate_google_id() {
        let (_temp, conn) = open_test_db();
        let discovered = DiscoveredGoogleCalendar {
            google_id: "work@example.com".to_string(),
            name: "Work".to_string(),
            color: "#50f872".to_string(),
            primary: false,
            access_role: Some("writer".to_string()),
            writable: true,
        };

        import_google_calendar(&conn, &discovered, None).unwrap();
        let err = import_google_calendar(&conn, &discovered, None).unwrap_err();
        assert_eq!(
            err,
            CalendarServiceError::Conflict(
                "google calendar 'work@example.com' is already imported".to_string()
            )
        );
    }

    #[test]
    fn import_google_calendar_persists_access_role_and_writability() {
        let (_temp, conn) = open_test_db();
        let imported = import_google_calendar(
            &conn,
            &DiscoveredGoogleCalendar {
                google_id: "readonly@example.com".to_string(),
                name: "Readonly".to_string(),
                color: "#123456".to_string(),
                primary: false,
                access_role: Some("reader".to_string()),
                writable: false,
            },
            None,
        )
        .unwrap();

        let sync_state = crate::sync::state::load_calendar_sync_state(&conn, &imported.id)
            .unwrap()
            .unwrap();
        assert_eq!(sync_state.google_access_role.as_deref(), Some("reader"));
        assert!(!sync_state.writable);
    }

    #[test]
    fn update_calendar_to_local_clears_google_id() {
        let (_temp, conn) = open_test_db();
        let created = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: Some(1),
            },
        )
        .unwrap();

        let updated = update_calendar(
            &conn,
            UpdateCalendarInput {
                id: created.id,
                name: Some("Personal".to_string()),
                color: None,
                source: Some(CalendarSource::Local),
                google_id: None,
                visible: None,
                position: None,
            },
        )
        .unwrap();

        assert_eq!(updated.source, CalendarSource::Local);
        assert_eq!(updated.google_id, None);
        assert_eq!(updated.name, "Personal");
    }

    #[test]
    fn update_calendar_to_local_detaches_event_sync_state() {
        let (_temp, conn) = open_test_db();
        let created = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: Some(1),
            },
        )
        .unwrap();
        let now = timestamp_now();
        db::insert_event(
            &conn,
            &crate::models::Event {
                id: uuid::Uuid::new_v4().to_string(),
                calendar_id: created.id.clone(),
                project_id: None,
                title: "Imported".to_string(),
                description: None,
                location: None,
                start_at: "2026-04-06 10:00:00".to_string(),
                end_at: "2026-04-06 11:00:00".to_string(),
                all_day: false,
                rrule: None,
                google_id: Some("google-event-1".to_string()),
                google_etag: Some("\"etag-1\"".to_string()),
                reminder_minutes: None,
                timezone: "UTC".to_string(),
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            },
        )
        .unwrap();
        db::upsert_sync_token(&conn, &created.id, "sync-token-1").unwrap();

        let updated = update_calendar(
            &conn,
            UpdateCalendarInput {
                id: created.id.clone(),
                name: None,
                color: None,
                source: Some(CalendarSource::Local),
                google_id: None,
                visible: None,
                position: None,
            },
        )
        .unwrap();

        let event = db::load_events(&conn)
            .unwrap()
            .into_iter()
            .find(|event| event.calendar_id == created.id)
            .unwrap();
        assert_eq!(updated.source, CalendarSource::Local);
        assert_eq!(updated.google_id, None);
        assert_eq!(event.google_id, None);
        assert_eq!(event.google_etag, None);
        assert_eq!(db::get_sync_token(&conn, &created.id).unwrap(), None);
    }

    #[test]
    fn update_calendar_rejects_google_id_changes_for_existing_google_calendar() {
        let (_temp, conn) = open_test_db();
        let created = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: Some(1),
            },
        )
        .unwrap();

        let err = update_calendar(
            &conn,
            UpdateCalendarInput {
                id: created.id,
                name: None,
                color: None,
                source: Some(CalendarSource::Google),
                google_id: Some(Some("personal@example.com".to_string())),
                visible: None,
                position: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            CalendarServiceError::Validation(
                "changing google_id on an existing google calendar is not supported".to_string()
            )
        );
    }

    #[test]
    fn filter_unimported_google_calendars_excludes_existing_rows() {
        let (_temp, conn) = open_test_db();
        import_google_calendar(
            &conn,
            &DiscoveredGoogleCalendar {
                google_id: "work@example.com".to_string(),
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                primary: false,
                access_role: Some("writer".to_string()),
                writable: true,
            },
            None,
        )
        .unwrap();

        let filtered = filter_unimported_google_calendars(
            &conn,
            vec![
                DiscoveredGoogleCalendar {
                    google_id: "work@example.com".to_string(),
                    name: "Work".to_string(),
                    color: "#50f872".to_string(),
                    primary: false,
                    access_role: Some("writer".to_string()),
                    writable: true,
                },
                DiscoveredGoogleCalendar {
                    google_id: "personal@example.com".to_string(),
                    name: "Personal".to_string(),
                    color: "#ffaa00".to_string(),
                    primary: true,
                    access_role: Some("reader".to_string()),
                    writable: false,
                },
            ],
        )
        .unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].google_id, "personal@example.com");
    }
}
