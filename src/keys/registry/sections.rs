/* Help overlay sections, generated from the registry.

A section is a surface's documentation: the keys that work there, in row
order. The overlay opens with the section for the surface you are looking from,
then the global keys, then everything else. */

use crate::keys::{Hint, View};

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
            Section::IcalImport => "ICAL IMPORT",
            Section::GoogleManage => "GOOGLE MANAGEMENT",
            Section::Planner => "PLANNER",
            Section::Help => "HELP",
        }
    }
}

/* Section reading order when nothing is current. */
const ORDER: [Section; 12] = [
    Section::Global,
    Section::Month,
    Section::Week,
    Section::Day,
    Section::Agenda,
    Section::CalendarList,
    Section::EventForm,
    Section::QuickAdd,
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

/* The help overlay contents for a view: sections in reading order, each with
its key lines. */
pub fn help_sections(view: &View) -> Vec<(Section, &'static str, Vec<Hint>)> {
    let current = section_of(view);

    /* Rows are collected in table order and deduplicated: a key bound in five
    surfaces is documented once per section it names. */
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

    let mut order = ORDER.to_vec();
    if let Some(current) = current {
        order.sort_by_key(|section| *section != current);
    }

    order
        .into_iter()
        .filter_map(|section| {
            let lines: Vec<Hint> = documented
                .iter()
                .filter(|(row_section, _, _)| *row_section == section)
                .map(|(_, key, desc)| (*key, *desc))
                .collect();
            if lines.is_empty() {
                None
            } else {
                Some((section, section.title(), lines))
            }
        })
        .collect()
}
