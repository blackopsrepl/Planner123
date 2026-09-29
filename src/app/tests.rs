use chrono::NaiveDate;

use super::palette::{fuzzy_match, parse_date_input};
use super::utilities::google_sync_finished_status;

#[test]
fn google_sync_finished_status_prefers_failure_over_success_banner() {
    let (message, is_error) = google_sync_finished_status(1, 1, 2, 3, 0);
    assert!(is_error);
    assert!(message.contains("1 succeeded"));
    assert!(message.contains("1 failed"));
}

#[test]
fn google_sync_finished_status_reports_success_totals() {
    let (message, is_error) = google_sync_finished_status(2, 0, 4, 5, 1);
    assert!(!is_error);
    assert_eq!(message, "Google sync: +4 events, 5 updated, 1 conflicts.");
}

#[test]
fn palette_filter_matches_subsequences_case_insensitively() {
    assert!(fuzzy_match("", "Export .ics file"));
    assert!(fuzzy_match("gcal", "Sync with Google Calendar"));
    assert!(fuzzy_match("EXPORT", "Export .ics file"));
    assert!(fuzzy_match("ics", "Import .ics file"));
    assert!(!fuzzy_match("zzz", "Export .ics file"));
    assert!(!fuzzy_match("cal", "Quit"));
}

#[test]
fn date_input_accepts_iso_dates_and_day_offsets() {
    let today = NaiveDate::from_ymd_opt(2026, 9, 29).unwrap();
    let date = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();

    assert_eq!(
        parse_date_input("2026-10-15", today).unwrap(),
        date(2026, 10, 15)
    );
    assert_eq!(parse_date_input("+21", today).unwrap(), date(2026, 10, 20));
    assert_eq!(parse_date_input("-7", today).unwrap(), date(2026, 9, 22));
    assert_eq!(parse_date_input("+0", today).unwrap(), today);

    assert!(parse_date_input("tomorrow", today).is_err());
    assert!(parse_date_input("+soon", today).is_err());
    assert!(parse_date_input("2026-13-01", today).is_err());
}
