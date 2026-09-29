use crossterm::event::KeyEvent;

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
    Palette,
    DateJump,
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

    // ── Command palette ──────────────────────────────────────────
    Palette,     // : = open the palette
    PaletteRun,  // Enter = run the selected command
    PaletteUp,   // ↑
    PaletteDown, // ↓

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

/* Key hint tuple: (key label, description). */
pub type Hint = (&'static str, &'static str);

mod registry;

pub use registry::{
    bar_hints, bar_hints_within, contexts, help_sections, hint_width, rows, section_of, tables,
    Binding, Context, Key, Section, CONTROL,
};

/* Resolve a key event to an action for the current view.

The keymap itself lives in `registry`: one row per binding, read by this
resolver, the status bar and the help overlay alike. */
pub fn resolve(view: &View, key: KeyEvent) -> Action {
    registry::resolve(Context::of(view), key)
}

/* The status bar chips of a view, in the order the registry lists them. */
pub fn hints(view: &View) -> Vec<Hint> {
    bar_hints(Context::of(view))
}

/* The status bar chips of a view that fit `budget` columns. */
pub fn hints_within(view: &View, budget: u16) -> Vec<Hint> {
    bar_hints_within(Context::of(view), budget)
}
