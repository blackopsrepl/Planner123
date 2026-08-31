/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */
/* Top-level render dispatch — the View function in TEA.  Layout:   ┌──────────────────────────────────┐ ← Length(1) header bar   │  📅  SolverForge Calendar   ...  │   ├──────────┬───────────────────────┤ ← Fill(1) main body   │Calendars │  Month / Week / Day / │   │ 22 cols  │       Agenda view     │   ├──────────┴───────────────────────┤ ← Length(1) status bar   │  h/l day  H/L month  c create…  │   └──────────────────────────────────┘  */

pub mod agenda_view;
pub mod calendar_list;
pub mod day_view;
pub mod event_form;
pub mod google_auth;
pub mod google_manage;
pub mod help;
pub mod ical_import;
pub mod month_view;
pub mod planner;
pub mod quick_add;
pub mod status_bar;
pub mod util;
pub mod week_view;

use ratatui::{
    layout::{Constraint, Layout},
    Frame,
};

use crate::app::App;
use crate::keys::View;

/* Render the entire application for the current frame. */
pub fn render(app: &mut App, frame: &mut Frame) {
    let [header_area, body_area, status_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    // ── Header bar ───────────────────────────────────────────────
    status_bar::render_header(app, frame, header_area);

    // ── Main body: sidebar (22 cols) + content (fill) ────────────
    let sidebar_width = 22u16;
    let [sidebar_area, content_area] =
        Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Fill(1)])
            .areas(body_area);

    // Sidebar is always visible (calendars + projects)
    calendar_list::render_calendar_list(app, frame, sidebar_area);

    // Main content area based on current view
    match &app.view {
        View::Month | View::CalendarList => {
            month_view::render_month(app, frame, content_area);
        }
        View::Week => {
            week_view::render_week(app, frame, content_area);
        }
        View::Day => {
            day_view::render_day(app, frame, content_area);
        }
        View::Agenda => {
            agenda_view::render_agenda(app, frame, content_area);
        }
        View::EventForm => {
            // Show month view behind the form
            month_view::render_month(app, frame, content_area);
        }
        View::IcalImport => {
            month_view::render_month(app, frame, content_area);
        }
        View::QuickAdd => {
            month_view::render_month(app, frame, content_area);
        }
        View::Help => {
            month_view::render_month(app, frame, content_area);
        }
        View::GoogleAuth => {
            month_view::render_month(app, frame, content_area);
        }
        View::GoogleManage => {
            month_view::render_month(app, frame, content_area);
        }
        View::PlannerInbox => planner::render_inbox(app, frame, content_area),
        View::PlannerTaskForm => planner::render_inbox(app, frame, content_area),
        View::PlannerSettingsForm => planner::render_inbox(app, frame, content_area),
    }

    // ── Status / bottom bar ──────────────────────────────────────
    match &app.view {
        View::QuickAdd => quick_add::render_quick_add(app, frame, status_area),
        _ => status_bar::render_status_bar(app, frame, status_area),
    }

    // ── Overlays (rendered on top of everything) ─────────────────
    match &app.view {
        View::EventForm => event_form::render_event_form(app, frame),
        View::Help => help::render_help(app, frame),
        View::GoogleManage => google_manage::render_google_manage(app, frame),
        View::GoogleAuth => google_auth::render_google_auth(app, frame),
        View::IcalImport => ical_import::render_ical_import(app, frame),
        View::PlannerTaskForm => planner::render_task_form(app, frame),
        View::PlannerSettingsForm => planner::render_settings_form(app, frame),
        _ => {}
    }
}
