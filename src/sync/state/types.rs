use serde::{Deserialize, Serialize};

pub const GOOGLE_PROVIDER: &str = "google";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Clean,
    PendingCreate,
    PendingUpdate,
    PendingDelete,
    Conflicted,
    Error,
}

impl std::fmt::Display for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "clean"),
            Self::PendingCreate => write!(f, "pending_create"),
            Self::PendingUpdate => write!(f, "pending_update"),
            Self::PendingDelete => write!(f, "pending_delete"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl SyncState {
    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "pending_create" => Self::PendingCreate,
            "pending_update" => Self::PendingUpdate,
            "pending_delete" => Self::PendingDelete,
            "conflicted" => Self::Conflicted,
            "error" => Self::Error,
            _ => Self::Clean,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxOperation {
    Create,
    Update,
    Delete,
}

impl std::fmt::Display for OutboxOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create => write!(f, "create"),
            Self::Update => write!(f, "update"),
            Self::Delete => write!(f, "delete"),
        }
    }
}

impl OutboxOperation {
    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "create" => Self::Create,
            "delete" => Self::Delete,
            _ => Self::Update,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStatus {
    Pending,
    Resolved,
}

impl std::fmt::Display for ConflictResolutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Resolved => write!(f, "resolved"),
        }
    }
}

impl ConflictResolutionStatus {
    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "resolved" => Self::Resolved,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    KeepLocal,
    KeepRemote,
}

impl std::fmt::Display for ConflictResolutionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepLocal => write!(f, "keep_local"),
            Self::KeepRemote => write!(f, "keep_remote"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSyncState {
    pub calendar_id: String,
    pub google_access_role: Option<String>,
    pub writable: bool,
    pub last_synced_at: Option<String>,
    pub last_sync_error_code: Option<String>,
    pub last_sync_error_message: Option<String>,
    pub pending_outbox: usize,
    pub pending_conflicts: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSyncState {
    pub event_id: String,
    pub sync_state: SyncState,
    pub last_synced_at: Option<String>,
    pub last_push_attempt_at: Option<String>,
    pub last_remote_modified_at: Option<String>,
    pub last_sync_error_code: Option<String>,
    pub last_sync_error_message: Option<String>,
    pub remote_payload: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: String,
    pub provider: String,
    pub calendar_id: String,
    pub event_id: String,
    pub operation: OutboxOperation,
    pub enqueued_at: String,
    pub attempt_count: i64,
    pub last_attempt_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: String,
    pub event_id: String,
    pub calendar_id: String,
    pub local_snapshot: String,
    pub remote_snapshot: String,
    pub remote_etag: Option<String>,
    pub detected_at: String,
    pub resolution_status: ConflictResolutionStatus,
    pub resolution_strategy: Option<ConflictResolutionStrategy>,
    pub resolved_at: Option<String>,
}
