mod tests {
    use super::{
        google_event_to_local, is_recurring_exception, local_event_insert_body,
        local_event_patch_body, GoogleEvent,
    };
    use crate::models::Event;

    #[test]
    fn maps_google_event_datetime_into_wall_clock_timezone() {
        let event = serde_json::from_value::<GoogleEvent>(serde_json::json!({
            "id": "abc",
            "etag": "\"etag\"",
            "summary": "Planning",
            "start": {
                "dateTime": "2026-04-12T09:00:00+02:00",
                "timeZone": "Europe/Rome"
            },
            "end": {
                "dateTime": "2026-04-12T10:00:00+02:00",
                "timeZone": "Europe/Rome"
            }
        }))
        .unwrap();

        let local = google_event_to_local("calendar-1", &event).unwrap();
        assert_eq!(local.start_at, "2026-04-12 09:00:00");
        assert_eq!(local.end_at, "2026-04-12 10:00:00");
        assert_eq!(local.timezone, "Europe/Rome");
    }

    #[test]
    fn skips_recurring_exceptions_for_first_pass_support() {
        let event = serde_json::from_value::<GoogleEvent>(serde_json::json!({
            "id": "abc",
            "recurringEventId": "master",
            "originalStartTime": {
                "dateTime": "2026-04-12T09:00:00+02:00"
            }
        }))
        .unwrap();

        assert!(is_recurring_exception(&event));
    }

    #[test]
    fn builds_google_insert_body_for_all_day_event() {
        let mut event = Event::new(
            "calendar-1",
            "Offsite",
            "2026-04-12 00:00:00",
            "2026-04-12 23:59:59",
            "UTC",
        );
        event.all_day = true;

        let body = local_event_insert_body(&event).unwrap();
        assert_eq!(body["start"]["date"], "2026-04-12");
        assert_eq!(body["end"]["date"], "2026-04-13");
    }

    #[test]
    fn translates_canonical_rrule_content_for_google_create_and_update() {
        let mut event = Event::new(
            "calendar-1",
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.rrule = Some("FREQ=WEEKLY;COUNT=2".to_string());

        assert_eq!(
            local_event_insert_body(&event).unwrap()["recurrence"],
            serde_json::json!(["RRULE:FREQ=WEEKLY;COUNT=2"])
        );
        assert_eq!(
            local_event_patch_body(&event).unwrap()["recurrence"],
            serde_json::json!(["RRULE:FREQ=WEEKLY;COUNT=2"])
        );
    }

    #[test]
    fn imports_google_rrule_as_canonical_rule_content() {
        let event = serde_json::from_value::<GoogleEvent>(serde_json::json!({
            "id": "abc",
            "start": { "dateTime": "2026-04-12T09:00:00Z" },
            "end": { "dateTime": "2026-04-12T10:00:00Z" },
            "recurrence": ["RRULE:FREQ=WEEKLY;COUNT=2"]
        }))
        .unwrap();

        assert_eq!(
            google_event_to_local("calendar-1", &event)
                .unwrap()
                .rrule
                .as_deref(),
            Some("FREQ=WEEKLY;COUNT=2")
        );
    }
}
