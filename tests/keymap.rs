/* Tests for the keymap registry.

`src/keys/registry` is the single source of truth for what a key does (resolve),
what the status bar says (bar chips) and what the help overlay documents. These
tests pin the invariants that keep the three from drifting apart again: no key
bound twice in one surface, no action that cannot be reached, no documented key
that is not really bound, and a status bar that still reads as it did before the
registry existed.
*/

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use planner123::keys::{self, Action, Context, Key, View};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

fn char_key(character: char) -> KeyEvent {
    key(KeyCode::Char(character))
}

/* Every action the TUI can dispatch. A new variant lands here, in the registry,
or in INTERNAL — there is no silent fourth option. */
#[rustfmt::skip]
const ACTIONS: &[Action] = &[
    Action::Quit, Action::Help, Action::FocusSidebar, Action::FocusMain,
    Action::ViewMonth, Action::ViewWeek, Action::ViewDay, Action::ViewAgenda,
    Action::PrevPeriod, Action::NextPeriod, Action::PrevUnit, Action::NextUnit,
    Action::PrevDay, Action::NextDay, Action::JumpToday, Action::JumpToDate,
    Action::CreateEvent, Action::EditEvent, Action::DeleteEvent, Action::SelectEvent,
    Action::FormNextField, Action::FormPrevField, Action::FormSubmit, Action::FormCancel,
    Action::ToggleCalendar, Action::CalendarUp, Action::CalendarDown,
    Action::QuickAdd, Action::InputChar('\0'), Action::InputBackspace,
    Action::InputSubmit, Action::InputCancel,
    Action::Palette, Action::PaletteRun, Action::PaletteUp, Action::PaletteDown,
    Action::GoogleManage, Action::GoogleSync, Action::GoogleDiscoverCalendars,
    Action::GoogleImportCalendar, Action::GoogleLogin, Action::GoogleAuthLogout,
    Action::ImportIcal, Action::ExportIcal,
    Action::PlannerInbox, Action::CreateTask, Action::PlannerOptimize, Action::PlannerApply,
    Action::PlannerSettings,
    Action::ScrollUp, Action::ScrollDown, Action::ScrollPageUp, Action::ScrollPageDown,
    Action::Escape,
];

/* Actions that are deliberately not bound to a key. */
const INTERNAL: &[Action] = &[Action::None];

/* Actions that lost their letter to the `:` palette. `src/app/tests.rs` checks
the palette really offers each of them, so this list cannot rot into a claim. */
const PALETTE_ONLY: &[Action] = &[Action::JumpToDate, Action::ImportIcal, Action::ExportIcal];

#[test]
fn no_surface_binds_a_key_twice() {
    for context in keys::contexts() {
        let mut seen: Vec<Key> = Vec::new();
        for row in keys::rows(context) {
            for binding in row.keys {
                assert!(
                    !seen.contains(binding),
                    "{context:?} binds {binding:?} twice"
                );
                seen.push(*binding);
            }
        }
    }
}

#[test]
fn the_text_wildcard_never_shadows_an_explicit_key() {
    for context in keys::contexts() {
        let rows = keys::rows(context);
        let Some(wildcard) = rows.iter().position(|row| row.keys.contains(&Key::AnyChar)) else {
            continue;
        };
        for row in rows.iter().skip(wildcard + 1) {
            for binding in row.keys {
                assert!(
                    !matches!(binding, Key::Char(_) | Key::AnyChar),
                    "{context:?} binds {binding:?} after the wildcard row"
                );
            }
        }
    }
}

#[test]
fn every_action_is_reachable_from_the_keymap() {
    let mut bound: Vec<Action> = keys::CONTROL.iter().map(|row| row.action.clone()).collect();
    for context in keys::contexts() {
        for row in keys::rows(context) {
            if row.keys.is_empty() {
                continue;
            }
            bound.push(row.action.clone());
        }
    }

    for action in ACTIONS {
        assert!(
            bound.contains(action) || INTERNAL.contains(action) || PALETTE_ONLY.contains(action),
            "{action:?} is neither bound to a key, internal, nor reachable from the palette"
        );
    }
}

#[test]
fn documented_global_keys_are_really_bound() {
    let mut bound: Vec<Key> = keys::CONTROL
        .iter()
        .flat_map(|row| row.keys.iter().copied())
        .collect();
    for context in keys::contexts() {
        for row in keys::rows(context) {
            bound.extend(row.keys.iter().copied());
        }
    }

    for row in keys::rows(Context::Global) {
        assert!(
            !row.keys.is_empty(),
            "{:?} documents no key",
            row.help.map(|help| help.key)
        );
        for documented in row.keys {
            assert!(
                bound.contains(documented),
                "{documented:?} is documented as global but no surface binds it"
            );
        }
    }
}

/* The status bar as the registry orders it: verbs first, then the selection
keys, then the structural keys, then the date navigation (which the help
overlay documents in full). Two rows differ from the pre-registry bar on
purpose: Day view said `h/l week` while the key moves a single day, and quick
add (`/`) is a verb and now earns a chip. */
#[rustfmt::skip]
const EXPECTED_BAR: &[(View, &[&str])] = &[
    (View::Month, &["c create", "e edit", "d del", "/ quick", "j/k row", "1-4 view", "Tab sidebar", "p planner", "n today", ": cmd", "? help"]),
    (View::Week, &["c create", "e edit", "d del", "/ quick", "j/k event", "1-4 view", "Tab sidebar", "p planner", "n now", ": cmd", "? help"]),
    (View::Day, &["c create", "e edit", "d del", "/ quick", "j/k event", "1-4 view", "Tab sidebar", "p planner", "n now", ": cmd", "? help"]),
    (View::Agenda, &["c create", "e edit", "d del", "/ quick", "j/k scroll", "1-4 view", "p planner", "n today", ": cmd", "? help"]),
    (View::CalendarList, &["j/k nav", "Space toggle", "Tab main", ": cmd", "? help"]),
    (View::EventForm, &["Tab/↑↓ field", "Enter save", "Esc cancel"]),
    (View::IcalImport, &["Tab/↑↓ field", "Enter import", "Esc cancel"]),
    (View::QuickAdd, &["Enter add", "Esc cancel"]),
    (View::Palette, &["↑/↓ move", "Enter run", "Esc close"]),
    (View::DateJump, &["Enter go", "Esc cancel"]),
    (View::Help, &["j/k scroll", "/ filter", "Esc close"]),
    (View::GoogleManage, &["j/k nav", "Enter import", "r refresh", "l login", "o logout", "S sync", "Esc close"]),
    (View::GoogleAuth, &["Tab field", "Enter confirm", "Esc cancel"]),
    (View::PlannerInbox, &["j/k inspect task", "n new task", "o optimize", "A apply", "s settings", "Esc close"]),
    (View::PlannerTaskForm, &["Tab/↑↓ field", "Enter save", "Esc cancel"]),
    (View::PlannerSettingsForm, &["Tab/↑↓ field", "Enter save", "Esc cancel"]),
];

#[test]
fn status_bar_chips_match_the_reviewed_table() {
    for (view, expected) in EXPECTED_BAR {
        let rendered: Vec<String> = keys::hints(view)
            .into_iter()
            .map(|(k, d)| format!("{k} {d}"))
            .collect();
        assert_eq!(rendered, *expected, "status bar changed for {view:?}");
    }
}

#[test]
fn a_wide_bar_keeps_every_chip() {
    for (view, expected) in EXPECTED_BAR {
        assert_eq!(
            keys::hints_within(view, 500).len(),
            expected.len(),
            "{view:?} dropped a chip it had room for"
        );
    }
}

#[test]
fn a_narrow_bar_drops_the_tail_and_keeps_help() {
    for (view, expected) in EXPECTED_BAR {
        for budget in [80u16, 100, 120, 160] {
            let chips = keys::hints_within(view, budget);
            let width: usize = chips.iter().map(keys::hint_width).sum();
            assert!(
                width <= budget as usize,
                "{view:?} overflows a {budget} column budget with {width} columns"
            );
            assert!(
                chips.len() <= expected.len(),
                "{view:?} invented a chip for a {budget} column budget"
            );
            if expected.contains(&"? help") {
                assert_eq!(
                    chips.last(),
                    Some(&("?", "help")),
                    "{view:?} dropped its way out of help"
                );
            }
        }
    }
}

/* A 80 column terminal is the common case this whole exercise is about: the
verbs must survive there. */
#[test]
fn narrow_bars_keep_the_verbs() {
    for view in [View::Month, View::Week, View::Day] {
        let chips = keys::hints_within(&view, 60);
        let keys_shown: Vec<&str> = chips.iter().map(|(key, _)| *key).collect();
        assert_eq!(
            keys_shown,
            vec!["c", "e", "d", "/", "?"],
            "{view:?} lost a verb on a narrow bar"
        );
    }
}

/* A sample of today's behaviour, surface by surface, including the cases that
are easy to get wrong when a keymap is rewritten: the control chord that wins
in a form, the surfaces where `q` closes instead of quitting, and the sidebar
that never had view switching. */
#[test]
fn representative_keys_resolve_as_before() {
    let cases: &[(View, KeyEvent, Action)] = &[
        (View::Month, char_key('h'), Action::PrevDay),
        (View::Month, char_key('H'), Action::PrevPeriod),
        (View::Month, key(KeyCode::Enter), Action::SelectEvent),
        (View::Month, char_key('1'), Action::ViewMonth),
        (View::Month, char_key(':'), Action::Palette),
        (View::Palette, char_key('j'), Action::InputChar('j')),
        (View::Palette, key(KeyCode::Up), Action::PaletteUp),
        (View::Palette, key(KeyCode::Enter), Action::PaletteRun),
        (View::DateJump, char_key('+'), Action::InputChar('+')),
        (View::DateJump, key(KeyCode::Enter), Action::InputSubmit),
        (View::Month, ctrl('c'), Action::Quit),
        (View::Week, char_key('h'), Action::PrevPeriod),
        (View::Agenda, char_key('j'), Action::ScrollDown),
        (View::Agenda, key(KeyCode::PageDown), Action::ScrollPageDown),
        (
            View::CalendarList,
            key(KeyCode::Char(' ')),
            Action::ToggleCalendar,
        ),
        (View::CalendarList, char_key('1'), Action::None),
        (View::CalendarList, key(KeyCode::Tab), Action::FocusMain),
        (View::EventForm, char_key('q'), Action::InputChar('q')),
        (View::EventForm, ctrl('q'), Action::Quit),
        (View::EventForm, char_key(' '), Action::InputChar(' ')),
        (View::EventForm, key(KeyCode::Left), Action::InputChar('h')),
        (View::EventForm, key(KeyCode::Right), Action::InputChar('l')),
        (View::EventForm, char_key('+'), Action::InputChar('+')),
        (View::QuickAdd, key(KeyCode::Enter), Action::InputSubmit),
        (View::GoogleAuth, char_key('q'), Action::Escape),
        (View::GoogleManage, char_key('S'), Action::GoogleSync),
        (View::GoogleManage, char_key('s'), Action::None),
        (
            View::GoogleManage,
            key(KeyCode::Enter),
            Action::GoogleImportCalendar,
        ),
        (View::GoogleManage, char_key('i'), Action::None),
        (View::GoogleManage, char_key('o'), Action::GoogleAuthLogout),
        (View::Month, char_key('g'), Action::None),
        (View::Month, char_key('i'), Action::None),
        (View::Month, char_key('x'), Action::None),
        (View::CalendarList, char_key('G'), Action::None),
        (View::PlannerInbox, char_key('s'), Action::PlannerSettings),
        (View::PlannerInbox, char_key('A'), Action::PlannerApply),
        (View::PlannerInbox, char_key('a'), Action::None),
        (View::Help, char_key('?'), Action::Escape),
        (View::Help, char_key('/'), Action::HelpFilter),
        (View::Help, key(KeyCode::PageUp), Action::ScrollPageUp),
    ];

    for (view, event, expected) in cases {
        assert_eq!(
            keys::resolve(view, *event),
            *expected,
            "{event:?} in {view:?} changed meaning"
        );
    }
}
