/* Tests for the help overlay.

The overlay is generated from the keymap registry, so what it can get wrong is
order and size: it must open on the surface you are in, that section must fit
the box without scrolling, and every chip the status bar shows must be
explained somewhere in it.
*/

use planner123::keys::{self, View};
use planner123::ui::util::centered_rect;
use ratatui::layout::Rect;

/* Rows the overlay can actually show at a terminal size: the box is 85% of the
height, minus its two border rows. Derived from the layout the renderer uses,
so this test breaks if the box shrinks rather than quietly passing. */
fn visible_rows(width: u16, height: u16) -> usize {
    let box_area = centered_rect(70, 85, Rect::new(0, 0, width, height));
    (box_area.height as usize).saturating_sub(2)
}

fn every_view() -> Vec<View> {
    vec![
        View::Month,
        View::Week,
        View::Day,
        View::Agenda,
        View::CalendarList,
        View::EventForm,
        View::IcalImport,
        View::QuickAdd,
        View::Help,
        View::GoogleManage,
        View::GoogleAuth,
        View::PlannerInbox,
        View::PlannerTaskForm,
        View::PlannerSettingsForm,
    ]
}

/* The overlay is read on the surface you are in: that section has to come
first, and it has to be on screen without scrolling at the terminal sizes
people actually run. */
#[test]
fn help_leads_with_the_current_surface_and_fits_the_box() {
    for (width, height) in [(80u16, 24u16), (100, 30), (120, 40), (200, 50)] {
        let budget = visible_rows(width, height);
        for view in every_view() {
            let sections = keys::help_sections(&view);
            assert!(!sections.is_empty(), "{view:?} has no help at all");
            let rows = sections[0].2.len() + 1; // heading
            assert!(
                rows <= budget,
                "{view:?} needs {rows} rows for its own keys, {width}x{height} shows {budget}"
            );
        }
    }
}

#[test]
fn help_opens_on_the_surface_you_are_looking_from() {
    for view in every_view() {
        let sections = keys::help_sections(&view);
        let Some(current) = keys::section_of(&view) else {
            continue;
        };
        assert_eq!(
            sections[0].0, current,
            "{view:?} does not open help on its own section"
        );
    }
}

/* A chip that points at a key the overlay never explains is how the old map
started to rot. `1-4` and `Tab/↑↓` are grouped labels for keys documented one
by one. */
#[test]
fn every_status_bar_chip_is_documented() {
    let grouped = [("1-4", "1"), ("Tab/↑↓", "Tab")];

    for view in every_view() {
        let documented: Vec<String> = keys::help_sections(&view)
            .iter()
            .flat_map(|(_, _, rows)| rows.iter().map(|(key_name, _)| flatten(key_name)))
            .collect();
        for (chip, _) in keys::hints(&view) {
            let lookup = grouped
                .iter()
                .find(|(label, _)| *label == chip)
                .map(|(_, target)| *target)
                .unwrap_or(chip);
            let target = flatten(lookup);
            assert!(
                documented.iter().any(|line| line.contains(&target)),
                "{view:?} shows a `{chip}` chip that help never explains"
            );
        }
    }
}

/* Key labels compare across spellings: `j/k` and `j / k` are the same key. */
fn flatten(label: &str) -> String {
    label
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}
