mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{export_events, import_from_str};
    use crate::{db, models::Event};

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
        (temp, conn)
    }

    fn first_calendar_id(conn: &Connection) -> String {
        db::load_calendars(conn).unwrap()[0].id.clone()
    }

    #[test]
    fn export_round_trips_rrule() {
        let event = Event {
            id: "event-1".to_string(),
            calendar_id: "calendar-1".to_string(),
            project_id: None,
            title: "Planning".to_string(),
            description: Some("Discuss".to_string()),
            location: Some("HQ".to_string()),
            start_at: "2026-04-12 09:00:00".to_string(),
            end_at: "2026-04-12 10:00:00".to_string(),
            all_day: false,
            rrule: Some("FREQ=WEEKLY;COUNT=4".to_string()),
            google_id: None,
            google_etag: None,
            reminder_minutes: None,
            timezone: "Europe/Rome".to_string(),
            created_at: "2026-04-12 08:00:00".to_string(),
            updated_at: "2026-04-12 08:00:00".to_string(),
            deleted_at: None,
        };

        let exported = export_events(&[event], "Demo");
        assert!(exported.contains("RRULE:FREQ=WEEKLY;COUNT=4"));
    }

    #[test]
    fn import_preserves_timed_and_all_day_fields() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let report = import_from_str(
            &conn,
            &calendar_id,
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Planning\r\nDESCRIPTION:Discuss\\nPlan\r\nLOCATION:HQ\r\nDTSTART;TZID=Europe/Rome:20260412T090000\r\nDTEND;TZID=Europe/Rome:20260412T103000\r\nRRULE:FREQ=WEEKLY;COUNT=2\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20260420\r\nDTEND;VALUE=DATE:20260422\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            "Europe/Rome",
        )
        .unwrap();

        assert_eq!(report.imported, 2);
        assert_eq!(report.skipped, 0);

        let events = db::load_events(&conn).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].timezone, "Europe/Rome");
        assert_eq!(events[0].rrule.as_deref(), Some("FREQ=WEEKLY;COUNT=2"));
        assert_eq!(
            crate::google::types::local_event_insert_body(&events[0]).unwrap()["recurrence"],
            serde_json::json!(["RRULE:FREQ=WEEKLY;COUNT=2"])
        );
        assert!(events[1].all_day);
        assert_eq!(events[1].start_at, "2026-04-20 00:00:00");
        assert_eq!(events[1].end_at, "2026-04-21 23:59:59");
    }

    #[test]
    fn import_reports_unsupported_event_shapes() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let report = import_from_str(
            &conn,
            &calendar_id,
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Exception\r\nRECURRENCE-ID:20260412T090000Z\r\nDTSTART:20260412T090000Z\r\nDTEND:20260412T100000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:With Alarm\r\nDTSTART:20260412T110000Z\r\nDTEND:20260412T120000Z\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            "UTC",
        )
        .unwrap();

        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped, 1);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("RECURRENCE-ID")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("VALARM")));
    }
}
