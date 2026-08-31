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
