/* The keymap registry.

Every key the TUI understands is one row of this table: the surface it belongs
to, the keys that trigger it, the action it dispatches, and how it is presented
in the status bar and the help overlay. `resolve`, the status bar and the help
overlay all read the same rows, so the three can no longer disagree about what
a key does.

Rows are listed in display order for the surface: the status bar takes the
`bar` chips in row order, and a narrow terminal drops from the tail.
*/

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Action, View};

mod calendar;
mod chips;
mod key;
mod overlays;
mod sections;

pub use chips::{bar_hints, bar_hints_within, hint_width};
pub use key::Key;
pub use sections::{help_sections, help_sections_for, section_of, Section};

use calendar::{AGENDA, DAY, MONTH, WEEK};
use overlays::{
    CALENDAR_LIST, DATE_JUMP, EVENT_FORM, GLOBAL, GOOGLE_AUTH, GOOGLE_MANAGE, HELP, ICAL_IMPORT,
    PALETTE, PLANNER_INBOX, QUICK_ADD,
};

/* The surface a binding belongs to.

Resolution reads only the current surface's rows, so a key unbound in a
surface stays unbound there even when another surface binds it. `Global` holds
no resolving rows at all: it exists so the help overlay can list, once, the
keys every surface repeats, and so a test can prove those keys are really
bound somewhere. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Global,
    Month,
    Week,
    Day,
    Agenda,
    CalendarList,
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

impl Context {
    /* The surface the given view resolves keys in. */
    pub fn of(view: &View) -> Self {
        match view {
            View::Month => Context::Month,
            View::Week => Context::Week,
            View::Day => Context::Day,
            View::Agenda => Context::Agenda,
            View::CalendarList => Context::CalendarList,
            View::EventForm => Context::EventForm,
            View::IcalImport => Context::IcalImport,
            View::QuickAdd => Context::QuickAdd,
            View::Palette => Context::Palette,
            View::DateJump => Context::DateJump,
            View::Help => Context::Help,
            View::GoogleManage => Context::GoogleManage,
            View::GoogleAuth => Context::GoogleAuth,
            View::PlannerInbox => Context::PlannerInbox,
            View::PlannerTaskForm => Context::PlannerTaskForm,
            View::PlannerSettingsForm => Context::PlannerSettingsForm,
        }
    }
}

/* A status bar chip: the key label shown and what it does. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarHint {
    pub key: &'static str,
    pub label: &'static str,
}

/* One help overlay line, in the section it belongs to. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpRow {
    pub section: Section,
    pub key: &'static str,
    pub desc: &'static str,
}

/* One row of the table. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub context: Context,
    pub keys: &'static [Key],
    pub action: Action,
    pub bar: Option<BarHint>,
    pub help: Option<HelpRow>,
    /* Keep this row's status bar chip even when the bar has to drop hints. */
    pub pin: bool,
}

macro_rules! b {
    (pin $ctx:ident, [$($key:expr),* $(,)?], $action:expr, $bar:expr, $help:expr) => {
        $crate::keys::registry::Binding {
            context: $crate::keys::registry::Context::$ctx,
            keys: &[$($key),*],
            action: $action,
            bar: $bar,
            help: $help,
            pin: true,
        }
    };
    ($ctx:ident, [$($key:expr),* $(,)?], $action:expr, $bar:expr, $help:expr) => {
        $crate::keys::registry::Binding {
            context: $crate::keys::registry::Context::$ctx,
            keys: &[$($key),*],
            action: $action,
            bar: $bar,
            help: $help,
            pin: false,
        }
    };
}

macro_rules! bar {
    ($key:expr, $label:expr) => {
        Some($crate::keys::registry::BarHint {
            key: $key,
            label: $label,
        })
    };
}

macro_rules! help {
    ($section:expr, $key:expr, $desc:expr) => {
        Some($crate::keys::registry::HelpRow {
            section: $section,
            key: $key,
            desc: $desc,
        })
    };
}

pub(crate) use {b, bar, help};

/* Placeholder action for the text-field wildcard: `resolve` replaces it with
the character the user typed, so the value stored here is never dispatched. */
pub const TEXT_INPUT: Action = Action::InputChar('\0');

/* Terminal control keys. They fire in every surface, before the surface's own
rows are consulted, and are matched by key code alone. */
#[rustfmt::skip]
pub static CONTROL: &[Binding] = &[
    b!(Global, [Key::Ctrl('c'), Key::Ctrl('q')], Action::Quit, None, None),
];

/* The rows that resolve keys in the given surface. */
pub fn rows(context: Context) -> &'static [Binding] {
    match context {
        Context::Global => &[],
        Context::Month => MONTH,
        Context::Week => WEEK,
        Context::Day => DAY,
        Context::Agenda => AGENDA,
        Context::CalendarList => CALENDAR_LIST,
        Context::EventForm | Context::PlannerTaskForm | Context::PlannerSettingsForm => EVENT_FORM,
        Context::IcalImport => ICAL_IMPORT,
        Context::QuickAdd => QUICK_ADD,
        Context::Palette => PALETTE,
        Context::DateJump => DATE_JUMP,
        Context::Help => HELP,
        Context::GoogleManage => GOOGLE_MANAGE,
        Context::GoogleAuth => GOOGLE_AUTH,
        Context::PlannerInbox => PLANNER_INBOX,
    }
}

/* Every table, in the order the help overlay reads them. */
pub fn tables() -> Vec<&'static [Binding]> {
    vec![
        GLOBAL,
        MONTH,
        WEEK,
        DAY,
        AGENDA,
        CALENDAR_LIST,
        EVENT_FORM,
        ICAL_IMPORT,
        QUICK_ADD,
        PALETTE,
        DATE_JUMP,
        HELP,
        GOOGLE_MANAGE,
        GOOGLE_AUTH,
        PLANNER_INBOX,
    ]
}

/* Every surface that resolves keys. */
pub fn contexts() -> Vec<Context> {
    vec![
        Context::Month,
        Context::Week,
        Context::Day,
        Context::Agenda,
        Context::CalendarList,
        Context::EventForm,
        Context::IcalImport,
        Context::QuickAdd,
        Context::Palette,
        Context::DateJump,
        Context::Help,
        Context::GoogleManage,
        Context::GoogleAuth,
        Context::PlannerInbox,
        Context::PlannerTaskForm,
        Context::PlannerSettingsForm,
    ]
}

/* Resolve a key event to an action for the current surface. */
pub fn resolve(context: Context, event: KeyEvent) -> Action {
    /* A control chord is never a surface binding. */
    if event.modifiers == KeyModifiers::CONTROL {
        return CONTROL
            .iter()
            .find(|row| row.keys.iter().any(|key| key.matches(event)))
            .map(|row| row.action.clone())
            .unwrap_or(Action::None);
    }

    rows(context)
        .iter()
        .find_map(|row| {
            row.keys
                .iter()
                .find(|key| key.matches(event))
                .map(|_| action_for(row, event))
        })
        .unwrap_or(Action::None)
}

/* The action a matched row dispatches.

Only the text-field wildcard substitutes the pressed character. A named key
mapped onto the input path (an arrow key standing in for `h`, so every surface
cycles a select value the same way) dispatches the character it names. */
fn action_for(row: &Binding, event: KeyEvent) -> Action {
    let wildcard = matches!(row.keys, [Key::AnyChar]);
    match (row.action.clone(), event.code) {
        (Action::InputChar(_), KeyCode::Char(character)) if wildcard => {
            Action::InputChar(character)
        }
        (action, _) => action,
    }
}
