/* Regression tests for issue #2.

Month/date navigation must load the visible event range before rendering.
Previously `reload_events_if_needed` compared the display month against
`focused_date`, which month navigation moves in lockstep, so it never fired and
the grid rendered a stale (often empty) event set. `j`/`k` unit navigation and
`switch_view` did not request a reload at all.

The tests exercise the real event loop (dispatch + worker pump) against a
throwaway database seeded with events two months away, outside the window the
app loads for the current month on startup. */

mod common;

use std::time::Duration;

use common::{date, offset_month, offset_month_date, pump_until, seeded_app, serial_guard};
use planner123::keys::{Action, View};
use tempfile::TempDir;

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

#[test]
fn escaping_an_overlay_returns_to_the_view_it_was_opened_from() {
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

    // Day view: step one day at a time so every load narrows the window to a
    // single date, and the display month stops following the cursor.
    app.dispatch(Action::ViewDay);
    while app.focused_date != target {
        app.dispatch(Action::NextPeriod);
    }
    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| app
            .event_window
            == Some((target, target))
            && !app.loading),
        "day navigation must settle on a single-day window"
    );

    // An overlay opened from the day view closes back into the day view: the
    // cursor must not be thrown into the month grid.
    app.dispatch(Action::Help);
    app.dispatch(Action::Escape);

    assert_eq!(app.view, View::Day);
    assert_eq!(
        app.event_window,
        Some((target, target)),
        "returning must keep the day view's loaded range"
    );
}

#[test]
fn returning_to_the_month_view_from_an_overlay_loads_the_focused_month() {
    let _serial = serial_guard();
    let dir = TempDir::new().unwrap();
    let target = offset_month_date(2);
    // Second event in the same month, on a day the app never loads while the
    // cursor walks out to `target` one day at a time.
    let (target_year, target_month) = offset_month(2);
    let other_day = date(target_year, target_month, 20);
    let (rt, mut app) = seeded_app(
        &dir,
        &[
            (
                &format!("{} 09:00:00", target),
                &format!("{} 10:00:00", target),
            ),
            (
                &format!("{} 09:00:00", other_day),
                &format!("{} 10:00:00", other_day),
            ),
        ],
    );

    app.dispatch(Action::ViewDay);
    while app.focused_date != target {
        app.dispatch(Action::NextPeriod);
    }

    // Back to the month grid, then an overlay round trip: the grid must show
    // the focused month with that month's events loaded, not the day window the
    // cursor walked out on.
    app.dispatch(Action::ViewMonth);
    app.dispatch(Action::Help);
    app.dispatch(Action::Escape);

    assert_eq!(app.view, View::Month);
    assert!(
        pump_until(&mut app, &rt, Duration::from_secs(5), |app| app.view_month
            == target_month
            && app.view_year == target_year
            && !app.events_on_date(other_day).is_empty()),
        "the month grid must re-anchor to the focused month and load it"
    );
}
