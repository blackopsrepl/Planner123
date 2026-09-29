/* Shared harness for the tests that drive a real `App`.

Each test binary picks the parts it needs, so unused items here are expected. */
#![allow(dead_code)]

/* Shared harness for the tests that drive a real `App`.

The app and its worker touch `XDG_DATA_HOME` and the keyring service, so every
test that builds one takes the same process-wide lock and points the data
directory at its own TempDir. */

use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, NaiveDate};
use planner123::app::App;
use planner123::models::Event;
use tempfile::TempDir;
use tokio::runtime::Runtime;

static SERIAL: Mutex<()> = Mutex::new(());

pub fn serial_guard() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/* Run the app's own loop (dispatch + worker pump) until the predicate holds. */
pub fn pump_until(
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

/* Build an App against a throwaway data directory containing one event per
supplied start/end pair. Waits for the startup load to settle so later
assertions observe only navigation-driven loads. */
pub fn seeded_app(dir: &TempDir, events: &[(&str, &str)]) -> (Runtime, App) {
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

pub fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/* A month `delta` months away from today, as (year, month). */
pub fn offset_month(delta: i32) -> (i32, u32) {
    let today = Local::now().date_naive();
    let total = today.year() * 12 + today.month() as i32 - 1 + delta;
    (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32)
}

/* A date on the 10th of the month `delta` months from today. */
pub fn offset_month_date(delta: i32) -> NaiveDate {
    let (year, month) = offset_month(delta);
    date(year, month, 10)
}
