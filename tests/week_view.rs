/* The week grid draws every timed event of the week, overlapping ones side by
side, and keeps all-day events out of the hours. Rendered through the real
`ui::render` into a TestBackend, so the assertions read the drawn cells. */

mod common;

use chrono::{Datelike, Duration, Local};
use common::{pump_until, seeded_app, serial_guard};
use planner123::keys::View;
use ratatui::{backend::TestBackend, Terminal};

const EVENTS: [(i64, &str, &str, &str); 10] = [
    (0, "09:00:00", "11:00:00", "Long"),
    (0, "10:00:00", "10:30:00", "Inner"),
    (1, "09:30:00", "10:00:00", "Dent"),
    (1, "10:00:00", "11:30:00", "Demo"),
    (2, "14:15:00", "14:45:00", "Ab"),
    (2, "14:45:00", "15:30:00", "Cd"),
    (3, "13:00:00", "15:00:00", "Big"),
    (3, "13:30:00", "14:00:00", "Mid"),
    (3, "14:00:00", "16:00:00", "Tail"),
    (4, "00:00:00", "23:59:59", "Holiday"),
];

fn rendered_week() -> Vec<String> {
    let dir = tempfile::TempDir::new().unwrap();
    let today = Local::now().date_naive();
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let at = |day: i64, time: &str| format!("{} {}", monday + Duration::days(day), time);
    let bounds: Vec<(String, String)> = EVENTS
        .iter()
        .map(|(day, start, end, _)| (at(*day, start), at(*day, end)))
        .collect();
    let refs: Vec<(&str, &str)> = bounds
        .iter()
        .map(|(s, e)| (s.as_str(), e.as_str()))
        .collect();
    let (rt, mut app) = seeded_app(&dir, &refs);
    assert!(pump_until(
        &mut app,
        &rt,
        std::time::Duration::from_secs(5),
        |app| app.events.len() == EVENTS.len()
    ));
    for event in app.events.iter_mut() {
        let (_, _, _, title) = EVENTS
            .iter()
            .zip(&bounds)
            .find(|(_, (s, e))| *s == event.start_at && *e == event.end_at)
            .map(|(spec, _)| *spec)
            .unwrap();
        event.title = title.into();
        event.all_day = title == "Holiday";
    }
    app.view = View::Week;
    app.week_scroll = 8;

    let mut terminal = Terminal::new(TestBackend::new(128, 40)).unwrap();
    terminal
        .draw(|frame| planner123::ui::render(&mut app, frame))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect()
        })
        .collect()
}

#[test]
fn every_timed_event_is_drawn_even_when_it_overlaps_another() {
    let _guard = serial_guard();
    let screen = rendered_week().join("\n");
    for (_, _, _, title) in EVENTS {
        assert!(
            screen.contains(title),
            "{title} is not on screen:\n{screen}"
        );
    }
}

#[test]
fn back_to_back_events_start_on_their_own_rows() {
    let _guard = serial_guard();
    let rows = rendered_week();
    let row_of = |title: &str| rows.iter().position(|row| row.contains(title)).unwrap();
    // Two text rows per hour: 09:30 is one row below 09:00, 10:00 one below that.
    assert_eq!(row_of("Demo"), row_of("Dent") + 1);
    assert_eq!(row_of("Dent"), row_of("Long") + 1);
    assert_eq!(row_of("Inner"), row_of("Long") + 2);
}

#[test]
fn all_day_events_sit_in_the_strip_not_on_the_hours() {
    let _guard = serial_guard();
    let rows = rendered_week();
    let strip = rows.iter().position(|row| row.contains("all ")).unwrap();
    let first_event = rows.iter().position(|row| row.contains("Long")).unwrap();
    assert!(strip < first_event);
    assert!(rows[strip].contains("Holiday"));
    assert_eq!(rows.iter().filter(|row| row.contains("Holiday")).count(), 1);
}
