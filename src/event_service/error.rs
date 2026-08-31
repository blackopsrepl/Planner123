#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventServiceError {
    NotFound { resource: &'static str, id: String },
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl std::fmt::Display for EventServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { resource, id } => write!(f, "{} '{}' not found", resource, id),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for EventServiceError {}
