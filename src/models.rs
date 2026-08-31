use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Calendar ─────────────────────────────────────────────────────────

/* A calendar (local or Google-synced). Maps to the `calendars` table. */
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

// ── Event ────────────────────────────────────────────────────────────

/* A calendar event. Maps to the `events` table. */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub calendar_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: String, // ISO 8601 datetime
    pub end_at: String,
    pub all_day: bool,
    pub rrule: Option<String>, // RFC 5545 RRULE property value, beginning with FREQ=
    pub google_id: Option<String>,
    pub google_etag: Option<String>,
    pub reminder_minutes: Option<i64>,
    pub timezone: String, // IANA timezone name e.g. "Europe/Lisbon"
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Event {
    pub fn new(
        calendar_id: impl Into<String>,
        title: impl Into<String>,
        start_at: impl Into<String>,
        end_at: impl Into<String>,
        timezone: impl Into<String>,
    ) -> Self {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            calendar_id: calendar_id.into(),
            project_id: None,
            title: title.into(),
            description: None,
            location: None,
            start_at: start_at.into(),
            end_at: end_at.into(),
            all_day: false,
            rrule: None,
            google_id: None,
            google_etag: None,
            reminder_minutes: None,
            timezone: timezone.into(),
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Parse `start_at` into a `DateTime<Utc>` for calendar math.
    pub fn start_dt(&self) -> Option<DateTime<Utc>> {
        self.start_local_dt().map(|dt| dt.with_timezone(&Utc))
    }

    /// Parse `end_at` into a `DateTime<Utc>`.
    pub fn end_dt(&self) -> Option<DateTime<Utc>> {
        self.end_local_dt().map(|dt| dt.with_timezone(&Utc))
    }

    pub fn timezone_tz(&self) -> Option<Tz> {
        crate::time::parse_timezone(&self.timezone).ok()
    }

    pub fn start_local_dt(&self) -> Option<DateTime<Tz>> {
        crate::time::resolve_local_datetime(&self.start_at, &self.timezone).ok()
    }

    pub fn end_local_dt(&self) -> Option<DateTime<Tz>> {
        crate::time::resolve_local_datetime(&self.end_at, &self.timezone).ok()
    }

    /// Duration in minutes.
    pub fn duration_minutes(&self) -> Option<i64> {
        let start = self.start_dt()?;
        let end = self.end_dt()?;
        Some((end - start).num_minutes())
    }

    /// True if this event occurs on the given wall-clock date.
    pub fn occurs_on(&self, date: NaiveDate) -> bool {
        if let (Some(start), Some(end)) = (self.start_local_dt(), self.end_local_dt()) {
            let s = start.date_naive();
            let e = end.date_naive();
            date >= s && date <= e
        } else {
            false
        }
    }

    pub fn is_recurring(&self) -> bool {
        self.rrule.is_some()
    }
}

// ── EventDependency ──────────────────────────────────────────────────

/* A directed dependency edge between two events. Maps to `event_dependencies` table. */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDependency {
    pub id: String,
    pub from_event_id: String, // the blocking event
    pub to_event_id: String,   // the event that is blocked
    pub dependency_type: DependencyType,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Blocks,  // from must finish before to can start
    Related, // soft informational link
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Blocks => write!(f, "blocks"),
            DependencyType::Related => write!(f, "related"),
        }
    }
}

// ── Planner inbox ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
}

impl TaskPriority {
    pub fn weight(&self, low: i64, normal: i64, high: i64) -> i64 {
        match self {
            Self::Low => low,
            Self::Normal => normal,
            Self::High => high,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CognitiveLoad {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeadlineKind {
    #[default]
    None,
    Hard,
    Soft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningTaskState {
    Inbox,
    Applied,
    MissingEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningTask {
    pub id: String,
    pub title: String,
    pub duration_minutes: i64,
    pub target_calendar_id: String,
    pub project_id: Option<String>,
    pub priority: TaskPriority,
    pub cognitive_load: CognitiveLoad,
    pub earliest_at: Option<String>,
    pub deadline_kind: DeadlineKind,
    pub deadline_at: Option<String>,
    pub state: PlanningTaskState,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerSettings {
    pub timezone: Option<String>,
    pub availability_json: String,
    pub horizon_days: i64,
    pub slot_minutes: i64,
    pub solve_seconds: i64,
    pub priority_low_weight: i64,
    pub priority_normal_weight: i64,
    pub priority_high_weight: i64,
    pub cognitive_enabled: bool,
    pub low_window_start: String,
    pub low_window_end: String,
    pub low_outside_penalty: i64,
    pub medium_window_start: String,
    pub medium_window_end: String,
    pub medium_outside_penalty: i64,
    pub high_window_start: String,
    pub high_window_end: String,
    pub high_outside_penalty: i64,
    pub high_streak_limit: i64,
    pub recovery_minutes: i64,
    pub excess_high_penalty: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerProposal {
    pub id: String,
    pub status: String,
    pub horizon_start: String,
    pub horizon_days: i64,
    pub timezone: String,
    pub score: Option<String>,
    pub created_at: String,
    pub applied_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerProposalItem {
    pub id: String,
    pub proposal_id: String,
    pub task_id: String,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub scheduled: bool,
    pub cognitive_penalty: i64,
    pub fatigue_penalty: i64,
    pub explanation: Option<String>,
}
