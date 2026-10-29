mod occurrences;

pub use occurrences::{occurrence_starts, occurrences_between, OCCURRENCE_LIMIT};

/* Standard recurrence presets for the event form UI. */
#[derive(Debug, Clone, PartialEq)]
pub enum RecurrencePreset {
    None,
    Daily,
    WeeklyOnDay, // weekly on the same weekday as start
    Weekdays,    // Mon-Fri
    Weekly,
    BiWeekly,
    Monthly,
    Yearly,
    Custom(String), // raw RRULE
}

impl RecurrencePreset {
    pub fn label(&self) -> &str {
        match self {
            Self::None => "Does not repeat",
            Self::Daily => "Every day",
            Self::WeeklyOnDay => "Every week (same day)",
            Self::Weekdays => "Every weekday (Mon-Fri)",
            Self::Weekly => "Every week",
            Self::BiWeekly => "Every 2 weeks",
            Self::Monthly => "Every month",
            Self::Yearly => "Every year",
            Self::Custom(_) => "Custom…",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::None,
            Self::Daily,
            Self::WeeklyOnDay,
            Self::Weekdays,
            Self::Weekly,
            Self::BiWeekly,
            Self::Monthly,
            Self::Yearly,
            Self::Custom(String::new()),
        ]
    }
}
