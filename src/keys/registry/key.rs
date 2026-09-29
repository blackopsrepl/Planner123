/* The keys a row can listen on.

Named keys are variants of this enum so a row can list them directly next to
characters. `AnyChar` is the text-field wildcard: it matches any printable
character and is always listed last in a surface so explicit character bindings
win. Rows that document a typed convention rather than a binding carry no keys
at all and are listed under that surface's help section. */

use crossterm::event::{KeyCode, KeyEvent};

/* One key a row listens on.

Named keys are variants of this enum so a row can list them directly next to
characters. `AnyChar` is the text-field wildcard: it matches any printable
character and is always listed last in a surface so explicit character bindings
win. Rows that document a typed convention rather than a binding carry no keys
at all and are listed under that surface's help section. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Backspace,
    Space,
    AnyChar,
}

impl Key {
    /* Modifiers are ignored here, exactly as the resolvers before this table
    did: a control chord is filtered out before matching (see `resolve`). */
    pub(super) fn matches(self, event: KeyEvent) -> bool {
        match (self, event.code) {
            (Key::Ctrl(expected), KeyCode::Char(actual)) => expected == actual,
            (Key::Char(expected), KeyCode::Char(actual)) => expected == actual,
            (Key::AnyChar, KeyCode::Char(_)) => true,
            (named, code) => named.code() == Some(code),
        }
    }

    pub(super) fn code(self) -> Option<KeyCode> {
        match self {
            Key::Enter => Some(KeyCode::Enter),
            Key::Esc => Some(KeyCode::Esc),
            Key::Tab => Some(KeyCode::Tab),
            Key::BackTab => Some(KeyCode::BackTab),
            Key::Left => Some(KeyCode::Left),
            Key::Right => Some(KeyCode::Right),
            Key::Up => Some(KeyCode::Up),
            Key::Down => Some(KeyCode::Down),
            Key::PageUp => Some(KeyCode::PageUp),
            Key::PageDown => Some(KeyCode::PageDown),
            Key::Backspace => Some(KeyCode::Backspace),
            Key::Space => Some(KeyCode::Char(' ')),
            Key::Char(_) | Key::Ctrl(_) | Key::AnyChar => None,
        }
    }
}
