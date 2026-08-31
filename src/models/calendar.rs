#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String, // UUID v4
    pub name: String,
    pub color: String, // hex e.g. "#50f872"
    pub source: CalendarSource,
    pub google_id: Option<String>,
    pub visible: bool,
    pub position: i64,      // display order
    pub created_at: String, // ISO 8601
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalendarSource {
    Local,
    Google,
}

impl std::fmt::Display for CalendarSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalendarSource::Local => write!(f, "local"),
            CalendarSource::Google => write!(f, "google"),
        }
    }
}

// ── Project ──────────────────────────────────────────────────────────

/* A project groups events into a DAG with dependency ordering. Maps to `projects` table. */
