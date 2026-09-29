/* Overlay and panel surface rows: the calendar sidebar, the forms, the quick
add bar, help, and the Google and planner surfaces. */

use crate::keys::registry::{b, bar, help, Binding, Key::*, Section as S, TEXT_INPUT};
use crate::keys::Action as A;

/* The keys every surface repeats.

No view resolves against these rows: they exist so the help overlay can list
them once, and so `global_documentation_is_bound` can prove each one is really
bound in some surface. */
#[rustfmt::skip]
pub static GLOBAL: &[Binding] = &[
    b!(Global, [Char('q'), Ctrl('c')], A::Quit, None, help!(S::Global, "q / Ctrl+C", "Quit")),
    b!(Global, [Char('?')], A::Help, None, help!(S::Global, "?", "Toggle this help")),
    b!(Global, [Char(':')], A::Palette, None, help!(S::Global, ":", "Open the command palette")),
    b!(Global, [Char('1')], A::ViewMonth, None, help!(S::Global, "1", "Month view")),
    b!(Global, [Char('2')], A::ViewWeek, None, help!(S::Global, "2", "Week view")),
    b!(Global, [Char('3')], A::ViewDay, None, help!(S::Global, "3", "Day view")),
    b!(Global, [Char('4')], A::ViewAgenda, None, help!(S::Global, "4", "Agenda view")),
    b!(Global, [Tab], A::FocusSidebar, None, help!(S::Global, "Tab", "Focus calendar sidebar")),
    b!(Global, [Char('G')], A::GoogleManage, None, help!(S::Global, "G", "Open Google management")),
    b!(Global, [Char('S')], A::GoogleSync, None, help!(S::Global, "S", "Sync with Google Calendar")),
];

/* Calendar sidebar: visibility toggles plus the surface-owned keys. */
#[rustfmt::skip]
pub static CALENDAR_LIST: &[Binding] = &[
    b!(CalendarList, [Char('k'), Up], A::CalendarUp, bar!("j/k", "nav"), help!(S::CalendarList, "j / k", "Navigate calendars")),
    b!(CalendarList, [Char('j'), Down], A::CalendarDown, None, None),
    b!(CalendarList, [Space], A::ToggleCalendar, bar!("Space", "toggle"), help!(S::CalendarList, "Space", "Toggle calendar visibility")),
    b!(CalendarList, [Tab, Esc], A::FocusMain, bar!("Tab", "main"), help!(S::CalendarList, "Tab / Esc", "Return to main view")),
    b!(CalendarList, [Char(':')], A::Palette, bar!(":", "cmd"), help!(S::Global, ":", "Open the command palette")),
    b!(pin CalendarList, [Char('?')], A::Help, bar!("?", "help"), help!(S::Global, "?", "Toggle this help")),
    b!(CalendarList, [Char('c')], A::CreateEvent, None, None),
    b!(CalendarList, [Char('q')], A::Quit, None, None),
];

/* Event, planner task and planner settings forms share one key set: fields
with movement keys, save on Enter, cancel on Esc. Select fields are cycled by
typing left and right, which is why the character wildcard sits last. */
#[rustfmt::skip]
pub static EVENT_FORM: &[Binding] = &[
    b!(EventForm, [Tab, Down], A::FormNextField, bar!("Tab/↑↓", "field"), help!(S::EventForm, "Tab / ↓", "Next field")),
    b!(EventForm, [BackTab, Up], A::FormPrevField, None, help!(S::EventForm, "Shift+Tab / ↑", "Previous field")),
    b!(EventForm, [Left], A::InputChar('h'), None, None),
    b!(EventForm, [Right], A::InputChar('l'), None, None),
    b!(EventForm, [], A::None, None, help!(S::EventForm, "← / → (or h / l)", "Previous / next option, on a select")),
    b!(EventForm, [], A::None, None, help!(S::EventForm, "Space (on AllDay)", "Toggle")),
    b!(EventForm, [Enter], A::FormSubmit, bar!("Enter", "save"), help!(S::EventForm, "Enter", "Save event")),
    b!(EventForm, [Esc], A::FormCancel, bar!("Esc", "cancel"), help!(S::EventForm, "Esc", "Cancel")),
    b!(EventForm, [AnyChar], TEXT_INPUT, None, None),
    b!(EventForm, [Backspace], A::InputBackspace, None, None),
];

/* .ics import: a path field and a target calendar. */
#[rustfmt::skip]
pub static ICAL_IMPORT: &[Binding] = &[
    b!(IcalImport, [Tab, Down], A::FormNextField, bar!("Tab/↑↓", "field"), None),
    b!(IcalImport, [BackTab, Up], A::FormPrevField, None, None),
    b!(IcalImport, [], A::None, None, help!(S::IcalImport, "Type path", "Path to the .ics file")),
    b!(IcalImport, [], A::None, None, help!(S::IcalImport, "h / l", "Change target calendar")),
    b!(IcalImport, [Enter], A::FormSubmit, bar!("Enter", "import"), help!(S::IcalImport, "Enter", "Import")),
    b!(IcalImport, [Esc], A::FormCancel, bar!("Esc", "cancel"), help!(S::IcalImport, "Esc", "Cancel")),
    b!(IcalImport, [AnyChar], TEXT_INPUT, None, None),
    b!(IcalImport, [Backspace], A::InputBackspace, None, None),
];

/* Command palette: a typed filter over the actions that do not earn a key. */
#[rustfmt::skip]
pub static PALETTE: &[Binding] = &[
    b!(Palette, [Up], A::PaletteUp, bar!("↑/↓", "move"), help!(S::Palette, "↑ / ↓", "Move through matches")),
    b!(Palette, [Down], A::PaletteDown, None, None),
    b!(Palette, [Enter], A::PaletteRun, bar!("Enter", "run"), help!(S::Palette, "Enter", "Run the selected command")),
    b!(Palette, [Esc], A::Escape, bar!("Esc", "close"), help!(S::Palette, "Esc", "Close the palette")),
    b!(Palette, [AnyChar], TEXT_INPUT, None, help!(S::Palette, "Type", "Filter commands")),
    b!(Palette, [Backspace], A::InputBackspace, None, None),
];

/* Go to date prompt: an absolute date or an offset in days. */
#[rustfmt::skip]
pub static DATE_JUMP: &[Binding] = &[
    b!(DateJump, [Enter], A::InputSubmit, bar!("Enter", "go"), help!(S::DateJump, "Enter", "Jump to the date")),
    b!(DateJump, [Esc], A::InputCancel, bar!("Esc", "cancel"), help!(S::DateJump, "Esc", "Cancel")),
    b!(DateJump, [AnyChar], TEXT_INPUT, None, help!(S::DateJump, "Type", "YYYY-MM-DD, or +21 / -7 for days from today")),
    b!(DateJump, [Backspace], A::InputBackspace, None, None),
];

/* Quick add bar: type a title, Enter creates it on the focused date. */
#[rustfmt::skip]
pub static QUICK_ADD: &[Binding] = &[
    b!(QuickAdd, [Enter], A::InputSubmit, bar!("Enter", "add"), help!(S::QuickAdd, "Enter", "Create event")),
    b!(QuickAdd, [Esc], A::InputCancel, bar!("Esc", "cancel"), help!(S::QuickAdd, "Esc", "Cancel")),
    b!(QuickAdd, [], A::None, None, help!(S::QuickAdd, "Type title", "Event title (uses focused date)")),
    b!(QuickAdd, [AnyChar], TEXT_INPUT, None, None),
    b!(QuickAdd, [Backspace], A::InputBackspace, None, None),
];

/* Help overlay: scroll it, close it. */
#[rustfmt::skip]
pub static HELP: &[Binding] = &[
    b!(Help, [Char('j'), Down], A::ScrollDown, bar!("j/k", "scroll"), help!(S::Help, "j / k", "Scroll")),
    b!(Help, [Char('k'), Up], A::ScrollUp, None, None),
    b!(Help, [Char('/')], A::HelpFilter, bar!("/", "filter"), help!(S::Help, "/", "Filter the keys by typing")),
    b!(Help, [Esc, Char('q'), Char('?')], A::Escape, bar!("Esc", "close"), help!(S::Help, "Esc / ?", "Close help")),
    b!(Help, [PageUp], A::ScrollPageUp, None, help!(S::Help, "PgUp / PgDn", "Scroll page")),
    b!(Help, [PageDown], A::ScrollPageDown, None, None),
];

/* Google management overlay: connection state, discovery, import, sync. */
#[rustfmt::skip]
pub static GOOGLE_MANAGE: &[Binding] = &[
    b!(GoogleManage, [Char('k'), Up], A::CalendarUp, bar!("j/k", "nav"), help!(S::GoogleManage, "j / k", "Move through discoverable calendars")),
    b!(GoogleManage, [Char('j'), Down], A::CalendarDown, None, None),
    b!(GoogleManage, [Enter], A::GoogleImportCalendar, bar!("Enter", "import"), help!(S::GoogleManage, "Enter", "Import selected Google calendar")),
    b!(GoogleManage, [Char('r')], A::GoogleDiscoverCalendars, bar!("r", "refresh"), help!(S::GoogleManage, "r", "Refresh calendar discovery")),
    b!(GoogleManage, [Char('l')], A::GoogleLogin, bar!("l", "login"), help!(S::GoogleManage, "l", "Login / reconnect")),
    b!(GoogleManage, [Char('o')], A::GoogleAuthLogout, bar!("o", "logout"), help!(S::GoogleManage, "o", "Logout")),
    b!(GoogleManage, [Char('S')], A::GoogleSync, bar!("S", "sync"), help!(S::GoogleManage, "S", "Sync now")),
    b!(GoogleManage, [Esc, Char('q')], A::Escape, bar!("Esc", "close"), help!(S::GoogleManage, "Esc", "Close")),
];

/* Google sign-in wizard. The bar documents it; the help overlay does not,
because the wizard lists its own fields. */
#[rustfmt::skip]
pub static GOOGLE_AUTH: &[Binding] = &[
    b!(GoogleAuth, [Tab, Down], A::FormNextField, bar!("Tab", "field"), None),
    b!(GoogleAuth, [BackTab, Up], A::FormPrevField, None, None),
    b!(GoogleAuth, [Enter], A::FormSubmit, bar!("Enter", "confirm"), None),
    b!(GoogleAuth, [Esc, Char('q')], A::Escape, bar!("Esc", "cancel"), None),
    b!(GoogleAuth, [AnyChar], TEXT_INPUT, None, None),
    b!(GoogleAuth, [Backspace], A::InputBackspace, None, None),
];

/* Planner inbox: the triage surface. Review before apply. */
#[rustfmt::skip]
pub static PLANNER_INBOX: &[Binding] = &[
    b!(PlannerInbox, [Char('j'), Down], A::NextUnit, bar!("j/k", "inspect task"), help!(S::Planner, "j / k", "Select planner task")),
    b!(PlannerInbox, [Char('k'), Up], A::PrevUnit, None, None),
    b!(PlannerInbox, [Char('n')], A::CreateTask, bar!("n", "new task"), help!(S::Planner, "n", "Create planner task")),
    b!(PlannerInbox, [Char('o')], A::PlannerOptimize, bar!("o", "optimize"), help!(S::Planner, "o", "Optimize inbox into a proposal")),
    b!(PlannerInbox, [Char('A')], A::PlannerApply, bar!("A", "apply"), help!(S::Planner, "A", "Apply the reviewed proposal (writes events)")),
    b!(PlannerInbox, [Char('s')], A::PlannerSettings, bar!("s", "settings"), help!(S::Planner, "s", "Configure planner settings")),
    b!(PlannerInbox, [], A::None, None, help!(S::Planner, "Timezone", "IANA name: Europe/Rome or UTC (detected local shown)")),
    b!(PlannerInbox, [], A::None, None, help!(S::Planner, "Availability", "day=HH:MM-HH:MM, comma-separated; e.g. mon=09:00-17:00 (full days accepted)")),
    b!(PlannerInbox, [Esc, Char('q')], A::Escape, bar!("Esc", "close"), None),
];
