#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub calendar_id: String,
    pub calendar_name: String,
    pub path: String,
    pub imported: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedProperty {
    name: String,
    params: HashMap<String, String>,
    value: String,
}

#[derive(Debug, Default, Clone)]
struct ParsedEventBlock {
    properties: Vec<ParsedProperty>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ImportCandidate {
    event: Option<Event>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
enum ParsedEventTime {
    AllDay {
        date: NaiveDate,
    },
    Timed {
        utc: DateTime<Utc>,
        timezone: String,
    },
}

// ── Export ────────────────────────────────────────────────────────────

/* Export a list of events to an iCal string. */
