use super::*;

pub(super) fn resolve_google_auth(key: KeyEvent) -> Action {
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

pub(super) fn resolve_google_manage(key: KeyEvent) -> Action {
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

pub(super) fn resolve_planner_inbox(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc | Char('q') => Action::Escape,
        Char('n') => Action::CreateTask,
        Char('o') => Action::PlannerOptimize,
        Char('a') => Action::PlannerApply,
        Char('s') => Action::PlannerSettings,
        Char('j') | Down => Action::NextUnit,
        Char('k') | Up => Action::PrevUnit,
        _ => Action::None,
    }
}

// ── Status bar hints ─────────────────────────────────────────────────
