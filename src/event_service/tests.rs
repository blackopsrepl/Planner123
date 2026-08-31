mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{delete_event, save_event, EventServiceError};
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
    fn save_event_keeps_wall_clock_time_with_timezone() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let event = Event::new(
            calendar_id,
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "Europe/Rome",
        );

        let saved = save_event(&conn, event, true).unwrap();
        assert_eq!(saved.timezone, "Europe/Rome");
        assert_eq!(saved.start_at, "2026-04-12 09:00:00");
        assert_eq!(saved.end_at, "2026-04-12 10:00:00");
    }

    #[test]
    fn save_event_normalizes_all_day_bounds() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let mut event = Event::new(
            calendar_id,
            "Offsite",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.all_day = true;

        let saved = save_event(&conn, event, true).unwrap();
        assert_eq!(saved.start_at, "2026-04-12 00:00:00");
        assert_eq!(saved.end_at, "2026-04-12 23:59:59");
    }

    #[test]
    fn save_event_requires_unprefixed_rrule_content() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let mut event = Event::new(
            calendar_id,
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.rrule = Some("RRULE:FREQ=WEEKLY".to_string());

        let err = save_event(&conn, event, true).unwrap_err();
        assert!(
            matches!(err, EventServiceError::Validation(message) if message.contains("RRULE:"))
        );
    }

    #[test]
    fn delete_event_requires_existing_row() {
        let (_temp, conn) = open_test_db();
        let err = delete_event(&conn, "missing").unwrap_err();
        assert_eq!(
            err,
            EventServiceError::NotFound {
                resource: "event",
                id: "missing".to_string()
            }
        );
    }
}
