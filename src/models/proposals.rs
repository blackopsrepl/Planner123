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
    pub diagnostics: PlannerProposalDiagnostics,
}

/* Structured, persisted evidence for a planner proposal item. */
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlannerProposalDiagnostics {
    pub outcome: PlannerProposalOutcome,
    pub busy_blockers: Vec<PlannerBusyBlocker>,
    pub busy_blockers_omitted: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerProposalOutcome {
    #[default]
    Scheduled,
    Unassigned,
    NoHardFeasibleSlot,
    FeasibleButNotSelected,
}

/* A calendar occurrence that removed an otherwise hard-feasible candidate slot. */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerBusyBlocker {
    pub event_id: String,
    pub event_title: String,
    pub calendar_id: String,
    pub start_at: String,
    pub end_at: String,
    pub recurring: bool,
}
