/* Calendar surface rows: month grid, week grid, day grid and agenda. */

use crate::keys::registry::{b, bar, help, Binding, Key::*, Section as S};
use crate::keys::Action as A;

/* Month grid: the calendar overview. Rows are in status bar order. */
#[rustfmt::skip]
pub static MONTH: &[Binding] = &[
    b!(Month, [Char('h'), Left], A::PrevDay, bar!("h/l", "day"), help!(S::Month, "h / l", "Previous / next day")),
    b!(Month, [Char('l'), Right], A::NextDay, None, None),
    b!(Month, [Char('H')], A::PrevPeriod, bar!("H/L", "month"), help!(S::Month, "H / L", "Previous / next month")),
    b!(Month, [Char('L')], A::NextPeriod, None, None),
    b!(Month, [Char('k'), Up], A::PrevUnit, bar!("j/k", "row"), help!(S::Month, "j / k", "Row down / up (7 days)")),
    b!(Month, [Char('j'), Down], A::NextUnit, None, None),
    b!(Month, [Char('n')], A::JumpToday, bar!("n", "today"), help!(S::Month, "n", "Jump to today")),
    b!(Month, [Char('c')], A::CreateEvent, bar!("c", "create"), help!(S::Month, "c", "Create event on selected day")),
    b!(Month, [Char('e')], A::EditEvent, bar!("e", "edit"), help!(S::Month, "e", "Edit selected event")),
    b!(Month, [Char('d')], A::DeleteEvent, bar!("d", "del"), help!(S::Month, "d", "Delete selected event")),
    b!(Month, [Char('1')], A::ViewMonth, bar!("1-4", "view"), help!(S::Global, "1", "Month view")),
    b!(Month, [Char('2')], A::ViewWeek, None, help!(S::Global, "2", "Week view")),
    b!(Month, [Char('3')], A::ViewDay, None, help!(S::Global, "3", "Day view")),
    b!(Month, [Char('4')], A::ViewAgenda, None, help!(S::Global, "4", "Agenda view")),
    b!(Month, [Tab], A::FocusSidebar, bar!("Tab", "sidebar"), help!(S::Global, "Tab", "Focus calendar sidebar")),
    b!(Month, [Char('p')], A::PlannerInbox, bar!("p", "planner"), help!(S::Planner, "p", "Open Planner Inbox")),
    b!(pin Month, [Char('?')], A::Help, bar!("?", "help"), help!(S::Global, "?", "Toggle this help")),
    b!(Month, [Char('q')], A::Quit, None, help!(S::Global, "q / Ctrl+C", "Quit")),
    b!(Month, [Char('G')], A::GoogleManage, None, help!(S::Global, "G", "Open Google management")),
    b!(Month, [Char('S')], A::GoogleSync, None, help!(S::Global, "S", "Sync with Google Calendar")),
    b!(Month, [Char('i')], A::ImportIcal, None, help!(S::Global, "i", "Open .ics import")),
    b!(Month, [Char('x')], A::ExportIcal, None, help!(S::Global, "x", "Export .ics file")),
    b!(Month, [Char('g')], A::JumpToDate, None, None),
    b!(Month, [Enter], A::SelectEvent, None, help!(S::Month, "Enter", "Switch to day view")),
    b!(Month, [Char('/')], A::QuickAdd, None, help!(S::Month, "/", "Quick-add event")),
    b!(Month, [Esc], A::Escape, None, None),
];

/* Week grid: hourly time grid. Rows are in status bar order. */
#[rustfmt::skip]
pub static WEEK: &[Binding] = &[
    b!(Week, [Char('h'), Left], A::PrevPeriod, bar!("h/l", "week"), help!(S::Week, "h / l", "Previous / next week")),
    b!(Week, [Char('l'), Right], A::NextPeriod, None, None),
    b!(Week, [Char('k'), Up], A::PrevUnit, bar!("j/k", "event"), help!(S::Week, "j / k", "Next / previous event")),
    b!(Week, [Char('j'), Down], A::NextUnit, None, None),
    b!(Week, [Char('n')], A::JumpToday, bar!("n", "now"), help!(S::Week, "n", "Jump to current time")),
    b!(Week, [Char('c')], A::CreateEvent, bar!("c", "create"), help!(S::Week, "c", "Create event")),
    b!(Week, [Char('e')], A::EditEvent, bar!("e", "edit"), help!(S::Week, "e", "Edit selected event")),
    b!(Week, [Char('d')], A::DeleteEvent, bar!("d", "del"), help!(S::Week, "d", "Delete event")),
    b!(Week, [Char('1')], A::ViewMonth, bar!("1-4", "view"), None),
    b!(Week, [Char('2')], A::ViewWeek, None, None),
    b!(Week, [Char('3')], A::ViewDay, None, None),
    b!(Week, [Char('4')], A::ViewAgenda, None, None),
    b!(Week, [Tab], A::FocusSidebar, bar!("Tab", "sidebar"), None),
    b!(Week, [Char('p')], A::PlannerInbox, bar!("p", "planner"), help!(S::Planner, "p", "Open Planner Inbox")),
    b!(pin Week, [Char('?')], A::Help, bar!("?", "help"), help!(S::Global, "?", "Toggle this help")),
    b!(Week, [PageUp], A::ScrollPageUp, None, help!(S::Week, "PgUp / PgDn", "Scroll time grid")),
    b!(Week, [PageDown], A::ScrollPageDown, None, None),
    b!(Week, [Char('q')], A::Quit, None, None),
    b!(Week, [Char('G')], A::GoogleManage, None, None),
    b!(Week, [Char('S')], A::GoogleSync, None, None),
    b!(Week, [Char('i')], A::ImportIcal, None, None),
    b!(Week, [Char('x')], A::ExportIcal, None, None),
    b!(Week, [Enter], A::SelectEvent, None, None),
    b!(Week, [Char('/')], A::QuickAdd, None, None),
    b!(Week, [Esc], A::Escape, None, None),
];

/* Day schedule. Navigation moves a single day, so the labels differ from the
week grid. */
#[rustfmt::skip]
pub static DAY: &[Binding] = &[
    b!(Day, [Char('h'), Left], A::PrevPeriod, bar!("h/l", "day"), help!(S::Day, "h / l", "Previous / next day")),
    b!(Day, [Char('l'), Right], A::NextPeriod, None, None),
    b!(Day, [Char('k'), Up], A::PrevUnit, bar!("j/k", "event"), help!(S::Day, "j / k", "Next / previous event")),
    b!(Day, [Char('j'), Down], A::NextUnit, None, None),
    b!(Day, [Char('n')], A::JumpToday, bar!("n", "now"), help!(S::Day, "n", "Jump to current time")),
    b!(Day, [Char('c')], A::CreateEvent, bar!("c", "create"), help!(S::Day, "c", "Create event")),
    b!(Day, [Char('e')], A::EditEvent, bar!("e", "edit"), help!(S::Day, "e", "Edit selected event")),
    b!(Day, [Char('d')], A::DeleteEvent, bar!("d", "del"), help!(S::Day, "d", "Delete event")),
    b!(Day, [Char('1')], A::ViewMonth, bar!("1-4", "view"), None),
    b!(Day, [Char('2')], A::ViewWeek, None, None),
    b!(Day, [Char('3')], A::ViewDay, None, None),
    b!(Day, [Char('4')], A::ViewAgenda, None, None),
    b!(Day, [Tab], A::FocusSidebar, bar!("Tab", "sidebar"), None),
    b!(Day, [Char('p')], A::PlannerInbox, bar!("p", "planner"), help!(S::Planner, "p", "Open Planner Inbox")),
    b!(pin Day, [Char('?')], A::Help, bar!("?", "help"), help!(S::Global, "?", "Toggle this help")),
    b!(Day, [PageUp], A::ScrollPageUp, None, help!(S::Day, "PgUp / PgDn", "Scroll time grid")),
    b!(Day, [PageDown], A::ScrollPageDown, None, None),
    b!(Day, [Char('q')], A::Quit, None, None),
    b!(Day, [Char('G')], A::GoogleManage, None, None),
    b!(Day, [Char('S')], A::GoogleSync, None, None),
    b!(Day, [Char('i')], A::ImportIcal, None, None),
    b!(Day, [Char('x')], A::ExportIcal, None, None),
    b!(Day, [Enter], A::SelectEvent, None, None),
    b!(Day, [Char('/')], A::QuickAdd, None, None),
    b!(Day, [Esc], A::Escape, None, None),
];

/* Agenda: the upcoming list. */
#[rustfmt::skip]
pub static AGENDA: &[Binding] = &[
    b!(Agenda, [Char('k'), Up], A::ScrollUp, bar!("j/k", "scroll"), help!(S::Agenda, "j / k", "Scroll")),
    b!(Agenda, [Char('j'), Down], A::ScrollDown, None, None),
    b!(Agenda, [Char('n')], A::JumpToday, bar!("n", "today"), help!(S::Agenda, "n", "Jump to today")),
    b!(Agenda, [Char('c')], A::CreateEvent, bar!("c", "create"), help!(S::Agenda, "c", "Create event")),
    b!(Agenda, [Char('e')], A::EditEvent, bar!("e", "edit"), help!(S::Agenda, "e", "Edit selected event")),
    b!(Agenda, [Char('d')], A::DeleteEvent, bar!("d", "del"), help!(S::Agenda, "d", "Delete event")),
    b!(Agenda, [Char('1')], A::ViewMonth, bar!("1-4", "view"), None),
    b!(Agenda, [Char('2')], A::ViewWeek, None, None),
    b!(Agenda, [Char('3')], A::ViewDay, None, None),
    b!(Agenda, [Char('4')], A::ViewAgenda, None, None),
    b!(Agenda, [Char('p')], A::PlannerInbox, bar!("p", "planner"), help!(S::Planner, "p", "Open Planner Inbox")),
    b!(pin Agenda, [Char('?')], A::Help, bar!("?", "help"), help!(S::Global, "?", "Toggle this help")),
    b!(Agenda, [Tab], A::FocusSidebar, None, help!(S::Global, "Tab", "Focus calendar sidebar")),
    b!(Agenda, [PageUp], A::ScrollPageUp, None, None),
    b!(Agenda, [PageDown], A::ScrollPageDown, None, None),
    b!(Agenda, [Char('q')], A::Quit, None, None),
    b!(Agenda, [Char('G')], A::GoogleManage, None, None),
    b!(Agenda, [Char('S')], A::GoogleSync, None, None),
    b!(Agenda, [Char('i')], A::ImportIcal, None, None),
    b!(Agenda, [Char('x')], A::ExportIcal, None, None),
    b!(Agenda, [Enter], A::SelectEvent, None, None),
    b!(Agenda, [Char('/')], A::QuickAdd, None, None),
    b!(Agenda, [Esc], A::Escape, None, None),
];
