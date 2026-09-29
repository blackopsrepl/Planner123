/* Tests for the actions that cannot be undone.

`d` used to delete the selected event on the keystroke and `x` wrote every event
to a fixed path. Both now arm first, commit on `y`, and cancel on anything else,
with nothing else bound while the question is up. These tests drive the real
app loop against a throwaway database. */

mod common;

use std::time::Duration;

use chrono::Local;
use common::{pump_until, seeded_app, serial_guard};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use planner123::keys::Action;
use tempfile::TempDir;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/* An app holding one event today, loaded and ready to be acted on. */
fn app_with_one_event(dir: &TempDir) -> (tokio::runtime::Runtime, planner123::app::App) {
    let today = Local::now().date_naive();
    let (rt, mut app) = seeded_app(
        dir,
        &[(&format!("{today} 09:00:00"), &format!("{today} 10:00:00"))],
    );
    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| !app
            .events_on_date(today)
            .is_empty()),
        "the seeded event must be loaded before the test can act on it"
    );
    (rt, app)
}

#[test]
fn deleting_an_event_arms_first_and_commits_only_on_confirmation() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let today = Local::now().date_naive();
    let (rt, mut app) = app_with_one_event(&dir);

    app.dispatch(Action::DeleteEvent);
    let confirm = app
        .pending_confirm
        .as_ref()
        .expect("delete must arm a confirmation");
    assert!(confirm.prompt.starts_with("Delete"), "{}", confirm.prompt);
    assert!(
        !app.events_on_date(today).is_empty(),
        "the event must survive the request"
    );

    app.handle_key(key(KeyCode::Char('y')));

    assert!(app.pending_confirm.is_none());
    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| app
            .events_on_date(today)
            .is_empty()),
        "confirming must delete the event"
    );
}

#[test]
fn any_other_key_cancels_a_pending_delete() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let today = Local::now().date_naive();
    let (rt, mut app) = app_with_one_event(&dir);

    app.dispatch(Action::DeleteEvent);
    app.handle_key(key(KeyCode::Char('n')));
    app.handle_key(key(KeyCode::Char('y')));

    /* The `y` above must not have reached the confirmation: it was consumed by
    the cancel that preceded it. */
    assert!(app.pending_confirm.is_none());
    let _guard = rt.enter();
    for _ in 0..20 {
        app.handle_tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !app.events_on_date(today).is_empty(),
        "a cancelled delete must leave the event alone"
    );
    assert_eq!(app.status_message, "Cancelled.");
}

#[test]
fn a_confirmation_swallows_the_keys_it_does_not_answer() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let today = Local::now().date_naive();
    let (rt, mut app) = app_with_one_event(&dir);
    let before = app.view.clone();

    /* Quit, help, a second `d`: none of them run while the question is up. The
    first key that is not `y` cancels the question and does nothing else. */
    for code in [KeyCode::Char('q'), KeyCode::Char('?'), KeyCode::Char('d')] {
        app.dispatch(Action::DeleteEvent);
        app.handle_key(key(code));
        assert!(app.running, "{code:?} quit during a confirmation");
        assert_eq!(app.view, before, "{code:?} acted during a confirmation");
        assert!(
            app.pending_confirm.is_none(),
            "{code:?} left the confirmation armed"
        );
    }

    let _guard = rt.enter();
    for _ in 0..20 {
        app.handle_tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !app.events_on_date(today).is_empty(),
        "nothing that was not a confirmation may delete the event"
    );
}
