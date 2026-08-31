use super::*;

pub(super) fn resolve_month(key: KeyEvent) -> Action {
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
        Char('p') => Action::PlannerInbox,
        Esc => Action::Escape,
        _ => Action::None,
    }
}

pub(super) fn resolve_time_grid(key: KeyEvent) -> Action {
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
        Char('p') => Action::PlannerInbox,
        Esc => Action::Escape,
        PageUp => Action::ScrollPageUp,
        PageDown => Action::ScrollPageDown,
        _ => Action::None,
    }
}

pub(super) fn resolve_agenda(key: KeyEvent) -> Action {
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
        Char('p') => Action::PlannerInbox,
        PageUp => Action::ScrollPageUp,
        PageDown => Action::ScrollPageDown,
        Esc => Action::Escape,
        _ => Action::None,
    }
}
