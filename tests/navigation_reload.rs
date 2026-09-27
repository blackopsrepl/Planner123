/* Regression tests for issue #2.

Month/date navigation must load the visible event range before rendering.
Previously `reload_events_if_needed` compared the display month against
`focused_date`, which month navigation moves in lockstep, so it never fired and
the grid rendered a stale (often empty) event set. `j`/`k` unit navigation and
`switch_view` did not request a reload at all.

The tests exercise the real event loop (dispatch + worker pump) against a
throwaway database seeded with events two months away, outside the window the
app loads for the current month on startup. */

use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, NaiveDate};
use planner123::app::App;
use planner123::keys::Action;
use planner123::models::Event;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// The app and worker touch `XDG_DATA_HOME`/`PLANNER123_TEST_KEYRING_SERVICE`,
/// so these tests must not run concurrently with each other.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial_guard() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn pump_until(
    app: &mut App,
    rt: &Runtime,
    timeout: Duration,
    predicate: impl Fn(&App) -> bool,
) -> bool {
    let _guard = rt.enter();
    let deadline = Instant::now() + timeout;
    loop {
        if predicate(app) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        app.handle_tick();
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Build an App against a throwaway data directory containing one event per
/// supplied start/end pair. Waits for the startup load to settle so later
/// assertions observe only navigation-driven loads.
fn seeded_app(dir: &TempDir, events: &[(&str, &str)]) -> (Runtime, App) {
    std::env::set_var("XDG_DATA_HOME", dir.path());

    {
        let conn = planner123::db::open().unwrap();
        let calendar_id = planner123::db::load_calendars(&conn).unwrap()[0].id.clone();
        for (start, end) in events {
            let event = Event::new(calendar_id.clone(), "Probe", *start, *end, "UTC");
            planner123::event_service::save_event(&conn, event, true).unwrap();
        }
    }

    let rt = Runtime::new().unwrap();
    let mut app = App::new(rt.handle().clone());
    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| {
            !app.calendars.is_empty() && !app.loading
        }),
        "startup calendar/event load did not settle"
    );
    (rt, app)
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// A month `delta` months away from today, as (year, month).
fn offset_month(delta: i32) -> (i32, u32) {
    let today = Local::now().date_naive();
    let total = today.year() * 12 + today.month() as i32 - 1 + delta;
    (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32)
}

/// A date on the 10th of the month `delta` months from today.
fn offset_month_date(delta: i32) -> NaiveDate {
    let (year, month) = offset_month(delta);
    date(year, month, 10)
}

#[test]
fn next_month_navigation_loads_visible_range() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let target = offset_month_date(2);
    let (rt, mut app) = seeded_app(
        &dir,
        &[(
            &format!("{} 09:00:00", target),
            &format!("{} 10:00:00", target),
        )],
    );

    app.dispatch(Action::NextPeriod);
    app.dispatch(Action::NextPeriod);

    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| !app
            .events_on_date(target)
            .is_empty()),
        "advancing two months must load the target month's events"
    );
}

#[test]
fn previous_month_navigation_loads_visible_range() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let target = offset_month_date(-2);
    let (rt, mut app) = seeded_app(
        &dir,
        &[(
            &format!("{} 09:00:00", target),
            &format!("{} 10:00:00", target),
        )],
    );

    app.dispatch(Action::PrevPeriod);
    app.dispatch(Action::PrevPeriod);

    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| !app
            .events_on_date(target)
            .is_empty()),
        "stepping back two months must load the target month's events"
    );
}

#[test]
fn unit_navigation_across_month_boundary_loads_visible_range() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let target = offset_month_date(2);
    let (rt, mut app) = seeded_app(
        &dir,
        &[(
            &format!("{} 09:00:00", target),
            &format!("{} 10:00:00", target),
        )],
    );

    // Step the cursor a week at a time until it leaves the current grid and the
    // month display re-anchors to the target month.
    let (_, target_month) = offset_month(2);
    for _ in 0..12 {
        if app.view_month == target_month {
            break;
        }
        app.dispatch(Action::NextUnit);
    }

    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| !app
            .events_on_date(target)
            .is_empty()),
        "j/k cursor movement across a month boundary must load the new range"
    );
}

#[test]
fn switching_from_month_to_week_loads_visible_range() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let target = offset_month_date(2);
    let (rt, mut app) = seeded_app(
        &dir,
        &[(
            &format!("{} 09:00:00", target),
            &format!("{} 10:00:00", target),
        )],
    );

    // Place the cursor in the target month without triggering a navigation
    // load, then switch views: the view switch itself must load the week.
    let (target_year, target_month) = offset_month(2);
    app.focused_date = target;
    app.view_month = target_month;
    app.view_year = target_year;

    app.dispatch(Action::ViewWeek);

    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| !app
            .events_on_date(target)
            .is_empty()),
        "switching from month to week must load the week's events"
    );
}

#[test]
fn navigation_requests_window_covering_the_focused_month() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let (rt, mut app) = seeded_app(&dir, &[]);
    let _guard = rt.enter();

    let target = offset_month_date(2);
    app.dispatch(Action::NextPeriod);
    app.dispatch(Action::NextPeriod);

    let (start, end) = app
        .event_window
        .expect("navigation must record the requested window");
    assert!(
        start <= target && target <= end,
        "requested window {start}..{end} must include {target}"
    );
}
