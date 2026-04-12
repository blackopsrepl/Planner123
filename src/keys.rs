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
    }
}

fn resolve_month(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Char('q') => Action::Quit,
        Char('?') => Action::Help,
        Char('1') => Action::ViewMonth,
        Char('2') => Action::ViewWeek,
        Char('3') => Action::ViewDay,
        Char('4') => Action::ViewAgenda,
        Tab => Action::FocusSidebar,
        Char('h') | Left => Action::PrevDay,
        Char('l') | Right => Action::NextDay,
        Char('k') | Up => Action::PrevUnit,
        Char('j') | Down => Action::NextUnit,
        Char('H') => Action::PrevPeriod,
        Char('L') => Action::NextPeriod,
        Char('n') => Action::JumpToday,
        Char('g') => Action::JumpToDate,
        Char('c') => Action::CreateEvent,
        Char('e') => Action::EditEvent,
        Char('d') => Action::DeleteEvent,
        Enter => Action::SelectEvent,
        Char('/') => Action::QuickAdd,
        Char('G') => Action::GoogleManage,
        Char('S') => Action::GoogleSync,
        Char('i') => Action::ImportIcal,
        Char('x') => Action::ExportIcal,
        Esc => Action::Escape,
        _ => Action::None,
    }
}

fn resolve_time_grid(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Char('q') => Action::Quit,
        Char('?') => Action::Help,
        Char('1') => Action::ViewMonth,
        Char('2') => Action::ViewWeek,
        Char('3') => Action::ViewDay,
        Char('4') => Action::ViewAgenda,
        Tab => Action::FocusSidebar,
        Char('h') | Left => Action::PrevPeriod,
        Char('l') | Right => Action::NextPeriod,
        Char('k') | Up => Action::PrevUnit,
        Char('j') | Down => Action::NextUnit,
        Char('n') => Action::JumpToday,
        Char('c') => Action::CreateEvent,
        Char('e') => Action::EditEvent,
        Char('d') => Action::DeleteEvent,
        Enter => Action::SelectEvent,
        Char('/') => Action::QuickAdd,
        Char('G') => Action::GoogleManage,
        Char('S') => Action::GoogleSync,
        Char('i') => Action::ImportIcal,
        Char('x') => Action::ExportIcal,
        Esc => Action::Escape,
        PageUp => Action::ScrollPageUp,
        PageDown => Action::ScrollPageDown,
        _ => Action::None,
    }
}

fn resolve_agenda(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Char('q') => Action::Quit,
        Char('?') => Action::Help,
        Char('1') => Action::ViewMonth,
        Char('2') => Action::ViewWeek,
        Char('3') => Action::ViewDay,
        Char('4') => Action::ViewAgenda,
        Tab => Action::FocusSidebar,
        Char('k') | Up => Action::ScrollUp,
        Char('j') | Down => Action::ScrollDown,
        Char('n') => Action::JumpToday,
        Char('c') => Action::CreateEvent,
        Char('e') => Action::EditEvent,
        Char('d') => Action::DeleteEvent,
        Enter => Action::SelectEvent,
        Char('/') => Action::QuickAdd,
        Char('G') => Action::GoogleManage,
        Char('S') => Action::GoogleSync,
        Char('i') => Action::ImportIcal,
        Char('x') => Action::ExportIcal,
        PageUp => Action::ScrollPageUp,
        PageDown => Action::ScrollPageDown,
        Esc => Action::Escape,
        _ => Action::None,
    }
}

fn resolve_calendar_list(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Char('q') => Action::Quit,
        Tab | Esc => Action::FocusMain,
        Char('k') | Up => Action::CalendarUp,
        Char('j') | Down => Action::CalendarDown,
        Char(' ') => Action::ToggleCalendar,
        Char('c') => Action::CreateEvent,
        Char('G') => Action::GoogleManage,
        Char('S') => Action::GoogleSync,
        Char('i') => Action::ImportIcal,
        Char('x') => Action::ExportIcal,
        Char('?') => Action::Help,
        _ => Action::None,
    }
}

fn resolve_event_form(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc => Action::FormCancel,
        Enter => Action::FormSubmit,
        Tab | Down => Action::FormNextField,
        BackTab | Up => Action::FormPrevField,
        Char(c) => Action::InputChar(c),
        Backspace => Action::InputBackspace,
        _ => Action::None,
    }
}

fn resolve_input(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc => Action::InputCancel,
        Enter => Action::InputSubmit,
        Char(c) => Action::InputChar(c),
        Backspace => Action::InputBackspace,
        _ => Action::None,
    }
}

fn resolve_ical_import(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc => Action::FormCancel,
        Enter => Action::FormSubmit,
        Tab | Down => Action::FormNextField,
        BackTab | Up => Action::FormPrevField,
        Char(c) => Action::InputChar(c),
        Backspace => Action::InputBackspace,
        _ => Action::None,
    }
}

fn resolve_help(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Char('q') | Esc | Char('?') => Action::Escape,
        Char('k') | Up => Action::ScrollUp,
        Char('j') | Down => Action::ScrollDown,
        PageUp => Action::ScrollPageUp,
        PageDown => Action::ScrollPageDown,
        _ => Action::None,
    }
}

fn resolve_google_auth(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc | Char('q') => Action::Escape,
        Enter => Action::FormSubmit,
        Char(c) => Action::InputChar(c),
        Backspace => Action::InputBackspace,
        Tab | Down => Action::FormNextField,
        BackTab | Up => Action::FormPrevField,
        _ => Action::None,
    }
}

fn resolve_google_manage(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc | Char('q') => Action::Escape,
        Char('k') | Up => Action::CalendarUp,
        Char('j') | Down => Action::CalendarDown,
        Char('r') => Action::GoogleDiscoverCalendars,
        Char('i') | Enter => Action::GoogleImportCalendar,
        Char('l') => Action::GoogleLogin,
        Char('o') => Action::GoogleAuthLogout,
        Char('s') | Char('S') => Action::GoogleSync,
        _ => Action::None,
    }
}

// ── Status bar hints ─────────────────────────────────────────────────

/* Key hint tuple: (key label, description). */
pub type Hint = (&'static str, &'static str);

/* Returns the status bar key hints for the current view. */
pub fn hints(view: &View) -> Vec<Hint> {
    match view {
        View::Month => vec![
            ("h/l", "day"),
            ("H/L", "month"),
            ("j/k", "row"),
            ("n", "today"),
            ("c", "create"),
            ("e", "edit"),
            ("d", "del"),
            ("1-4", "view"),
            ("Tab", "sidebar"),
            ("?", "help"),
        ],
        View::Week | View::Day => vec![
            ("h/l", "week"),
            ("j/k", "event"),
            ("n", "now"),
            ("c", "create"),
            ("e", "edit"),
            ("d", "del"),
            ("1-4", "view"),
            ("Tab", "sidebar"),
            ("?", "help"),
        ],
        View::Agenda => vec![
            ("j/k", "scroll"),
            ("n", "today"),
            ("c", "create"),
            ("e", "edit"),
            ("d", "del"),
            ("1-4", "view"),
            ("?", "help"),
        ],
        View::CalendarList => vec![
            ("j/k", "nav"),
            ("Space", "toggle"),
            ("Tab", "main"),
            ("?", "help"),
        ],
        View::EventForm => vec![("Tab/↑↓", "field"), ("Enter", "save"), ("Esc", "cancel")],
        View::IcalImport => vec![("Tab/↑↓", "field"), ("Enter", "import"), ("Esc", "cancel")],
        View::QuickAdd => vec![("Enter", "add"), ("Esc", "cancel")],
        View::Help => vec![("j/k", "scroll"), ("Esc", "close")],
        View::GoogleManage => vec![
            ("j/k", "nav"),
            ("i", "import"),
            ("r", "refresh"),
            ("l", "login"),
            ("o", "logout"),
            ("s", "sync"),
            ("Esc", "close"),
        ],
        View::GoogleAuth => vec![("Tab", "field"), ("Enter", "confirm"), ("Esc", "cancel")],
    }
}
