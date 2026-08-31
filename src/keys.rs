use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/* All distinct views the application can be in. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Month,
    Week,
    Day,
    Agenda,
    CalendarList, // sidebar focused
    EventForm,
    IcalImport,
    QuickAdd,
    Help,
    GoogleManage,
    GoogleAuth,
    PlannerInbox,
    PlannerTaskForm,
    PlannerSettingsForm,
}

/* Every user-facing action the app can take. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // ── Navigation ──────────────────────────────────────────────
    Quit,
    Help,
    FocusSidebar,
    FocusMain,

    // ── View switching ──────────────────────────────────────────
    ViewMonth,
    ViewWeek,
    ViewDay,
    ViewAgenda,

    // ── Time navigation ─────────────────────────────────────────
    PrevPeriod, // ← in month = prev month; in week = prev week; in day = prev day
    NextPeriod, // →
    PrevUnit,   // ↑ in month = up row; in week/day = prev event; in agenda = up
    NextUnit,   // ↓
    PrevDay,    // h
    NextDay,    // l
    JumpToday,  // n = jump to today / now
    JumpToDate, // g = go to specific date (opens quick input)

    // ── Event actions ────────────────────────────────────────────
    CreateEvent,
    EditEvent,
    DeleteEvent,
    SelectEvent, // Enter

    // ── Form actions ─────────────────────────────────────────────
    FormNextField,
    FormPrevField,
    FormSubmit,
    FormCancel,

    // ── Calendar list ─────────────────────────────────────────────
    ToggleCalendar, // Space = toggle visibility
    CalendarUp,
    CalendarDown,

    // ── Quick-add bar ─────────────────────────────────────────────
    QuickAdd,
    InputChar(char),
    InputBackspace,
    InputSubmit,
    InputCancel,

    // ── Google Calendar ──────────────────────────────────────────
    GoogleManage,
    GoogleSync,
    GoogleDiscoverCalendars,
    GoogleImportCalendar,
    GoogleLogin,
    GoogleAuthLogout,

    // ── iCal import/export ───────────────────────────────────────
    ImportIcal,
    ExportIcal,

    // ── Planner inbox ───────────────────────────────────────────
    PlannerInbox,
    CreateTask,
    PlannerOptimize,
    PlannerApply,
    PlannerSettings,

    // ── Scroll (help, agenda) ────────────────────────────────────
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,

    // ── Misc ─────────────────────────────────────────────────────
    Escape,
    None,
}

/* Resolve a key event to an action for the current view. */
pub fn resolve(view: &View, key: KeyEvent) -> Action {
    use KeyCode::*;
    use KeyModifiers as Mod;

    // ── Global (Ctrl-modified) ───────────────────────────────────
    if key.modifiers == Mod::CONTROL {
        return match key.code {
            Char('c') | Char('q') => Action::Quit,
            _ => Action::None,
        };
    }

    // ── View-specific ─────────────────────────────────────────────
    match view {
        View::Month => resolve_month(key),
        View::Week => resolve_time_grid(key),
        View::Day => resolve_time_grid(key),
        View::Agenda => resolve_agenda(key),
        View::CalendarList => resolve_calendar_list(key),
        View::EventForm => resolve_event_form(key),
        View::IcalImport => resolve_ical_import(key),
        View::QuickAdd => resolve_input(key),
        View::Help => resolve_help(key),
        View::GoogleManage => resolve_google_manage(key),
        View::GoogleAuth => resolve_google_auth(key),
        View::PlannerInbox => resolve_planner_inbox(key),
        View::PlannerTaskForm => resolve_event_form(key),
        View::PlannerSettingsForm => resolve_event_form(key),
    }
}

mod forms;
mod google;
mod hints;
mod navigation;

use forms::*;
use google::*;
use navigation::*;

pub use hints::{hints, Hint};
