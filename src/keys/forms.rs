use super::*;

pub(super) fn resolve_calendar_list(key: KeyEvent) -> Action {
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

pub(super) fn resolve_event_form(key: KeyEvent) -> Action {
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

pub(super) fn resolve_input(key: KeyEvent) -> Action {
    use KeyCode::*;
    match key.code {
        Esc => Action::InputCancel,
        Enter => Action::InputSubmit,
        Char(c) => Action::InputChar(c),
        Backspace => Action::InputBackspace,
        _ => Action::None,
    }
}

pub(super) fn resolve_ical_import(key: KeyEvent) -> Action {
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

pub(super) fn resolve_help(key: KeyEvent) -> Action {
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
