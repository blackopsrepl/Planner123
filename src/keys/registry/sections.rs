/* Help overlay sections, generated from the registry.

A section is a surface's documentation: the keys that work there, in row
order. The overlay opens with the section for the surface you are looking from,
then the global keys, then everything else. */

use crate::keys::{fuzzy_match, Hint, View};

use super::{tables, Context};

/* One block of the help overlay. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Global,
    Month,
    Week,
    Day,
    Agenda,
    CalendarList,
    EventForm,
    QuickAdd,
    Palette,
    DateJump,
    IcalImport,
    GoogleManage,
    Planner,
    Help,
}

impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Section::Global => "GLOBAL",
            Section::Month => "MONTH VIEW",
            Section::Week => "WEEK VIEW",
            Section::Day => "DAY VIEW",
            Section::Agenda => "AGENDA VIEW",
            Section::CalendarList => "CALENDAR SIDEBAR (Tab)",
            Section::EventForm => "EVENT FORM",
            Section::QuickAdd => "QUICK-ADD BAR",
            Section::Palette => "COMMAND PALETTE",
            Section::DateJump => "GO TO DATE",
            Section::IcalImport => "ICAL IMPORT",
            Section::GoogleManage => "GOOGLE MANAGEMENT",
            Section::Planner => "PLANNER",
            Section::Help => "HELP",
        }
    }
}

/* Section reading order when nothing is current. */
const ORDER: [Section; 14] = [
    Section::Global,
    Section::Month,
    Section::Week,
    Section::Day,
    Section::Agenda,
    Section::CalendarList,
    Section::EventForm,
    Section::QuickAdd,
    Section::Palette,
    Section::DateJump,
    Section::IcalImport,
    Section::GoogleManage,
    Section::Planner,
    Section::Help,
];

/* The section a surface documents.

`None` for the surfaces whose own layout carries the instructions (the Google
sign-in wizard lists its fields and the step order), and for `Global`, which is
documentation for every surface rather than a surface of its own. */
pub fn section_for_context(context: Context) -> Option<Section> {
    match context {
        Context::Global => None,
        Context::Month => Some(Section::Month),
        Context::Week => Some(Section::Week),
        Context::Day => Some(Section::Day),
        Context::Agenda => Some(Section::Agenda),
        Context::CalendarList => Some(Section::CalendarList),
        Context::EventForm | Context::PlannerTaskForm | Context::PlannerSettingsForm => {
            Some(Section::EventForm)
        }
        Context::IcalImport => Some(Section::IcalImport),
        Context::QuickAdd => Some(Section::QuickAdd),
        Context::Palette => Some(Section::Palette),
        Context::DateJump => Some(Section::DateJump),
        Context::Help => Some(Section::Help),
        Context::GoogleManage => Some(Section::GoogleManage),
        Context::GoogleAuth => None,
        Context::PlannerInbox => Some(Section::Planner),
    }
}

/* The section the given view opens its help on. */
pub fn section_of(view: &View) -> Option<Section> {
    section_for_context(Context::of(view))
}

/* The help overlay contents for a view: sections in reading order, each with its
key lines, opening on that view's own section. */
pub fn help_sections(view: &View) -> Vec<(Section, &'static str, Vec<Hint>)> {
    help_sections_for(view, view, "")
}

/* The same, for an overlay opened from `came_from` and filtered by `query`.

`came_from` matters for the help overlay itself: `?` pressed while looking at
the month grid is a question about the month grid, so the month's keys lead and
the overlay's own keys follow it. A query keeps only the lines that match it,
and a section with no match disappears — which is what turns this from a scroll
into a lookup. */
pub fn help_sections_for(
    view: &View,
    came_from: &View,
    query: &str,
) -> Vec<(Section, &'static str, Vec<Hint>)> {
    let head = if *view == View::Help {
        section_of(came_from).or(Some(Section::Help))
    } else {
        section_of(view)
    };

    let documented = documented_lines();
    let mut order = ORDER.to_vec();
    if let Some(head) = head {
        order.sort_by_key(|section| *section != head);
    }

    order
        .into_iter()
        .filter_map(|section| {
            let lines: Vec<Hint> = documented
                .iter()
                .filter(|(row_section, _, _)| *row_section == section)
                .map(|(_, key, desc)| (*key, *desc))
                .filter(|(key, desc)| {
                    query.is_empty() || fuzzy_match(query, key) || fuzzy_match(query, desc)
                })
                .collect();
            (!lines.is_empty()).then_some((section, section.title(), lines))
        })
        .collect()
}

/* Documented lines, deduplicated, in table order: a key bound in five surfaces
is documented once per section it names. */
fn documented_lines() -> Vec<(Section, &'static str, &'static str)> {
    let mut documented: Vec<(Section, &'static str, &'static str)> = Vec::new();
    for table in tables() {
        for row in table.iter() {
            let Some(help) = row.help else { continue };
            let seen = documented.iter().any(|(section, key, desc)| {
                *section == help.section && *key == help.key && *desc == help.desc
            });
            if !seen {
                documented.push((help.section, help.key, help.desc));
            }
        }
    }
    documented
}
