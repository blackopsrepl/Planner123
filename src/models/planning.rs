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
