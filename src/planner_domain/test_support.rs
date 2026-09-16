//! Shared builders for rule-level and end-to-end planner tests.

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;

use super::{SolverAppliedBlock, SolverSlot, SolverTask};

/// A fixed Monday origin so grid math is deterministic in tests.
pub fn origin() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap()
}

/// Builds an unassigned task with sane defaults.
pub fn task(priority_weight: i64, duration_minutes: i64) -> SolverTask {
    SolverTask {
        id: 0,
        task_id: "task-0".into(),
        index: 0,
        duration_minutes,
        priority_weight,
        load: 1,
        earliest_at: None,
        hard_deadline: None,
        soft_deadline: None,
        depends_on: Vec::new(),
        not_before: origin(),
        timezone: Tz::UTC,
        recovery_minutes: 60,
        excess_high_penalty: 5,
        start_idx: None,
    }
}

/// A slot grid of `count` 30-minute slots starting at the fixed origin.
pub fn slots(count: usize) -> Vec<SolverSlot> {
    (0..count)
        .map(|index| SolverSlot {
            id: index,
            start: origin() + chrono::Duration::minutes(index as i64 * 30),
        })
        .collect()
}

/// An applied task block measured in minutes from the fixed origin.
pub fn applied_block(
    start_offset_minutes: i64,
    end_offset_minutes: i64,
    high: bool,
    successors: Vec<usize>,
) -> SolverAppliedBlock {
    SolverAppliedBlock {
        id: format!("applied:{start_offset_minutes}:{end_offset_minutes}"),
        start: origin() + chrono::Duration::minutes(start_offset_minutes),
        end: origin() + chrono::Duration::minutes(end_offset_minutes),
        high,
        successors,
    }
}
