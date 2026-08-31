use super::*;
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
            ("p", "planner"),
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
            ("p", "planner"),
            ("?", "help"),
        ],
        View::Agenda => vec![
            ("j/k", "scroll"),
            ("n", "today"),
            ("c", "create"),
            ("e", "edit"),
            ("d", "del"),
            ("1-4", "view"),
            ("p", "planner"),
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
        View::PlannerInbox => vec![
            ("n", "new task"),
            ("o", "optimize"),
            ("a", "apply"),
            ("s", "settings"),
            ("Esc", "close"),
        ],
        View::PlannerTaskForm => vec![("Tab/↑↓", "field"), ("Enter", "save"), ("Esc", "cancel")],
        View::PlannerSettingsForm => {
            vec![("Tab/↑↓", "field"), ("Enter", "save"), ("Esc", "cancel")]
        }
    }
}
