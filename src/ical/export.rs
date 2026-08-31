pub fn export_events(events: &[Event], calendar_name: &str) -> String {
    let mut cal = ICalCalendar::new();
    cal.name(calendar_name);
    cal.description("Exported from SolverForge Calendar");

    for ev in events {
        let mut ical_event = ICalEvent::new();
        ical_event.uid(&ev.id);
        ical_event.summary(&ev.title);

        if let Some(desc) = &ev.description {
            ical_event.description(desc);
        }
        if let Some(loc) = &ev.location {
            ical_event.location(loc);
        }

        // Timestamps
        if ev.all_day {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&ev.start_at[..10], "%Y-%m-%d") {
                ical_event.add_property("DTSTART;VALUE=DATE", date.format("%Y%m%d").to_string());
            }
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&ev.end_at[..10], "%Y-%m-%d") {
                ical_event.add_property("DTEND;VALUE=DATE", date.format("%Y%m%d").to_string());
            }
        } else {
            if let Some(dt) = ev.start_dt() {
                ical_event.starts(dt);
            }
            if let Some(dt) = ev.end_dt() {
                ical_event.ends(dt);
            }
        }

        // RRULE
        if let Some(rrule) = &ev.rrule {
            ical_event.add_property("RRULE", rrule.strip_prefix("RRULE:").unwrap_or(rrule));
        }

        cal.push(ical_event.done());
    }

    cal.to_string()
}

/* Write events to an .ics file. */
pub fn export_to_file(events: &[Event], calendar_name: &str, path: &std::path::Path) -> Result<()> {
    let content = export_events(events, calendar_name);
    std::fs::write(path, content)
        .with_context(|| format!("cannot write iCal to {}", path.display()))
}

// ── Import ────────────────────────────────────────────────────────────
