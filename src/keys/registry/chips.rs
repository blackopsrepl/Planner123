/* Status bar chips, derived from the table.

A surface's chips are its rows that carry a `bar` hint, in row order. The
budgeted form reserves the pinned chip (the one that opens help) before it
fills the rest, so a narrow terminal drops trailing chips instead of the way
out of the keymap. */

use crate::keys::Hint;

use super::Context;

/* Width of a rendered chip: " key " + " label " + 2 columns of gap. */
pub fn hint_width(hint: &Hint) -> usize {
    hint.0.chars().count() + hint.1.chars().count() + 6
}

/* The status bar chips of a surface, in row order. */
pub fn bar_hints(context: Context) -> Vec<Hint> {
    super::rows(context)
        .iter()
        .filter_map(|row| row.bar.map(|hint| (hint.key, hint.label)))
        .collect()
}

/* The chips that fit `budget` columns.

The pinned chip (the one that opens help) is reserved first, so a narrow
terminal drops trailing chips instead of the way out. */
pub fn bar_hints_within(context: Context, budget: u16) -> Vec<Hint> {
    let budget = budget as usize;
    let mut pinned: Option<Hint> = None;
    let mut kept: Vec<Hint> = Vec::new();
    let mut used = 0usize;

    for row in super::rows(context) {
        let Some(hint) = row.bar else { continue };
        let hint = (hint.key, hint.label);
        if row.pin {
            pinned = Some(hint);
            continue;
        }
        let width = hint_width(&hint);
        if used + width <= budget {
            used += width;
            kept.push(hint);
        }
    }

    if let Some(pin) = pinned {
        let width = hint_width(&pin);
        while used + width > budget {
            match kept.pop() {
                Some(dropped) => used -= hint_width(&dropped),
                None => break,
            }
        }
        if width <= budget {
            kept.push(pin);
        }
    }

    kept
}
