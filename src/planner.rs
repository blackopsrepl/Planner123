use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

use anyhow::Result;
use chrono::{DateTime, Days, Duration, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use solverforge::SolverEvent;
use uuid::Uuid;

use crate::{
    db, event_service,
    models::{
        CalendarSource, CognitiveLoad, DeadlineKind, Event, PlannerProposal,
        PlannerProposalDiagnostics, PlannerProposalItem, PlannerProposalOutcome, PlannerSettings,
        PlanningTask, PlanningTaskState, TaskPriority,
    },
    planner_domain::{
        SolverAppliedBlock, SolverAvailability, SolverBusy, SolverCognitiveWindow, SolverPlan,
        SolverSlot, SolverTask, PLANNER_MANAGER,
    },
    sync::state,
    time,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerError {
    NotFound { resource: &'static str, id: String },
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { resource, id } => write!(f, "{} '{}' not found", resource, id),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for PlannerError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Availability(pub BTreeMap<String, Vec<TimeWindow>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    pub title: String,
    pub duration_minutes: i64,
    pub target_calendar_id: String,
    pub project_id: Option<String>,
    pub priority: TaskPriority,
    pub cognitive_load: CognitiveLoad,
    pub earliest_at: Option<String>,
    pub deadline_kind: DeadlineKind,
    pub deadline_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub duration_minutes: Option<i64>,
    pub target_calendar_id: Option<String>,
    pub project_id: Option<Option<String>>,
    pub priority: Option<TaskPriority>,
    pub cognitive_load: Option<CognitiveLoad>,
    pub earliest_at: Option<Option<String>>,
    pub deadline_kind: Option<DeadlineKind>,
    pub deadline_at: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsUpdate {
    pub timezone: Option<String>,
    pub availability: Option<Availability>,
    pub horizon_days: Option<i64>,
    pub slot_minutes: Option<i64>,
    pub solve_seconds: Option<i64>,
    pub priority_low_weight: Option<i64>,
    pub priority_normal_weight: Option<i64>,
    pub priority_high_weight: Option<i64>,
    pub cognitive_enabled: Option<bool>,
    pub low_window_start: Option<String>,
    pub low_window_end: Option<String>,
    pub low_outside_penalty: Option<i64>,
    pub medium_window_start: Option<String>,
    pub medium_window_end: Option<String>,
    pub medium_outside_penalty: Option<i64>,
    pub high_window_start: Option<String>,
    pub high_window_end: Option<String>,
    pub high_outside_penalty: Option<i64>,
    pub high_streak_limit: Option<i64>,
    pub recovery_minutes: Option<i64>,
    pub excess_high_penalty: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalDetail {
    pub proposal: PlannerProposal,
    pub items: Vec<PlannerProposalItem>,
    pub applicability: ProposalApplicability,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProposalSnapshot {
    task_versions: HashMap<String, String>,
    event_versions: HashMap<String, String>,
    #[serde(default)]
    settings_json: Option<String>,
    #[serde(default)]
    dependencies: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalApplicability {
    pub can_apply: bool,
    pub reasons: Vec<String>,
}

impl ProposalApplicability {
    fn ready() -> Self {
        Self {
            can_apply: true,
            reasons: Vec::new(),
        }
    }
}

mod applicability;
mod busy_time;
pub mod constraints;
mod database;
mod diagnostics;
mod load;
mod optimization;
mod proposals;
mod scheduling;
mod settings;
mod settings_validation;
mod task_validation;
mod tasks;
#[cfg(test)]
mod tests;

use applicability::validate_snapshot;
use busy_time::*;
use database::*;
use load::*;
use scheduling::*;
use settings_validation::{
    canonical_settings_snapshot, normalize_clock, normalize_timezone, parse_clock, require_task,
    validate_settings,
};
use task_validation::*;

pub use applicability::proposal_applicability;
pub use optimization::optimize;
pub use proposals::{apply_proposal, list_proposals, proposal, return_to_inbox};
pub use settings::{settings, settings_json, update_settings};
pub use settings_validation::validate_availability;
pub use tasks::{
    add_dependency, create_task, delete_task, get_task, list_dependencies, list_inbox_tasks,
    list_tasks, remove_dependency, update_task,
};
