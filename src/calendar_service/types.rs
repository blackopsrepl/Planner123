#[derive(Debug, Clone)]
pub struct CreateCalendarInput {
    pub name: String,
    pub color: String,
    pub source: CalendarSource,
    pub google_id: Option<String>,
    pub visible: bool,
    pub position: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UpdateCalendarInput {
    pub id: String,
    pub name: Option<String>,
    pub color: Option<String>,
    pub source: Option<CalendarSource>,
    pub google_id: Option<Option<String>>,
    pub visible: Option<bool>,
    pub position: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarServiceError {
    NotFound { resource: &'static str, id: String },
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl std::fmt::Display for CalendarServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { resource, id } => write!(f, "{} '{}' not found", resource, id),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for CalendarServiceError {}
