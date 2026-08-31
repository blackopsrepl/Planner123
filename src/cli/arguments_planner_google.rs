#[derive(Debug, Args)]
pub struct TaskCreateArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub duration_minutes: i64,
    #[arg(long)]
    pub target_calendar_id: String,
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long, value_enum, default_value_t = TaskPriorityArg::Normal)]
    pub priority: TaskPriorityArg,
    #[arg(long, value_enum, default_value_t = CognitiveLoadArg::Medium)]
    pub cognitive_load: CognitiveLoadArg,
    #[arg(long, value_parser = parse_timestamp_arg)]
    pub earliest_at: Option<String>,
    #[arg(long, value_enum, default_value_t = DeadlineKindArg::None)]
    pub deadline_kind: DeadlineKindArg,
    #[arg(long, value_parser = parse_timestamp_arg)]
    pub deadline_at: Option<String>,
}

#[derive(Debug, Args)]
pub struct TaskUpdateArgs {
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub duration_minutes: Option<i64>,
    #[arg(long)]
    pub target_calendar_id: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub clear_project_id: bool,
    #[arg(long, value_enum)]
    pub priority: Option<TaskPriorityArg>,
    #[arg(long, value_enum)]
    pub cognitive_load: Option<CognitiveLoadArg>,
    #[arg(long, value_parser = parse_timestamp_arg)]
    pub earliest_at: Option<String>,
    #[arg(long)]
    pub clear_earliest_at: bool,
    #[arg(long, value_enum)]
    pub deadline_kind: Option<DeadlineKindArg>,
    #[arg(long, value_parser = parse_timestamp_arg)]
    pub deadline_at: Option<String>,
    #[arg(long)]
    pub clear_deadline_at: bool,
}

#[derive(Debug, Args)]
pub struct TaskDependencyArgs {
    #[arg(long)]
    pub from_task_id: String,
    #[arg(long)]
    pub to_task_id: String,
}

#[derive(Debug, Args)]
pub struct PlannerOptimizeArgs {
    #[arg(long)]
    pub horizon_days: Option<i64>,
}

#[derive(Debug, Args)]
pub struct PlannerSettingsArgs {
    #[arg(long)]
    pub timezone: Option<String>,
    #[arg(long)]
    /// Replace weekly availability with one or more DAY=HH:MM-HH:MM windows.
    #[arg(long, value_name = "DAY=START-END")]
    pub availability: Vec<String>,
    #[arg(long)]
    pub horizon_days: Option<i64>,
    #[arg(long)]
    pub slot_minutes: Option<i64>,
    #[arg(long)]
    pub solve_seconds: Option<i64>,
    #[arg(long)]
    pub priority_low_weight: Option<i64>,
    #[arg(long)]
    pub priority_normal_weight: Option<i64>,
    #[arg(long)]
    pub priority_high_weight: Option<i64>,
    #[arg(long, value_parser = clap::builder::BoolishValueParser::new())]
    pub cognitive_enabled: Option<bool>,
    #[arg(long)]
    pub low_window_start: Option<String>,
    #[arg(long)]
    pub low_window_end: Option<String>,
    #[arg(long)]
    pub low_outside_penalty: Option<i64>,
    #[arg(long)]
    pub medium_window_start: Option<String>,
    #[arg(long)]
    pub medium_window_end: Option<String>,
    #[arg(long)]
    pub medium_outside_penalty: Option<i64>,
    #[arg(long)]
    pub high_window_start: Option<String>,
    #[arg(long)]
    pub high_window_end: Option<String>,
    #[arg(long)]
    pub high_outside_penalty: Option<i64>,
    #[arg(long)]
    pub high_streak_limit: Option<i64>,
    #[arg(long)]
    pub recovery_minutes: Option<i64>,
    #[arg(long)]
    pub excess_high_penalty: Option<i64>,
}

#[derive(Debug, Args)]
pub struct GoogleSyncArgs {
    #[arg(long)]
    calendar_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct GoogleAuthLoginArgs {
    #[arg(long)]
    client_id: Option<String>,
    #[arg(long)]
    client_secret: Option<String>,
}

#[derive(Debug, Args)]
pub struct GoogleCalendarImportArgs {
    #[arg(long)]
    google_id: String,
    #[arg(long)]
    position: Option<i64>,
}

#[derive(Debug, Args)]
pub struct GoogleSyncStatusArgs {
    #[arg(long)]
    calendar_id: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConflictStrategyArg {
    KeepLocal,
    KeepRemote,
}

#[derive(Debug, Args)]
pub struct GoogleConflictResolveArgs {
    id: String,
    #[arg(long, value_enum)]
    strategy: ConflictStrategyArg,
}

#[derive(Debug, Args)]
pub struct IcalImportArgs {
    #[arg(long)]
    calendar_id: String,
    #[arg(long)]
    path: String,
    #[arg(long)]
    timezone: Option<String>,
}
