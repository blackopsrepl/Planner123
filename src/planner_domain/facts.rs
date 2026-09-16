//! Immutable problem facts for the calendar planner.
//!
//! The loader builds these directly from persisted state. They carry raw
//! domain data only: candidate slot identities, blocked intervals, weekly
//! availability windows, and cognitive preference windows. No fact contains a
//! precomputed feasibility or cost verdict.

use chrono::{DateTime, NaiveTime, Utc};
use solverforge::prelude::*;

/// A candidate start instant. The planning variable stores an index into
/// `SolverPlan.slots`; the grid geometry is carried by the grid itself.
#[problem_fact]
pub struct SolverSlot {
    #[planning_id]
    pub id: usize,
    /// Uniform-grid start instant for this candidate slot.
    pub start: DateTime<Utc>,
}

/// One expanded occurrence of an existing calendar event. Consumed by the
/// hard overlap rule and by proposal blocker evidence.
#[problem_fact]
pub struct SolverBusy {
    #[planning_id]
    pub id: String,
    /// Calendar event that produced this occurrence; consumed by blocker
    /// evidence so a proposal can name what blocked a slot.
    pub event_id: String,
    pub event_title: String,
    pub calendar_id: String,
    /// Whether the occurrence comes from a recurrence expansion.
    pub recurring: bool,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// One previously applied task block, loaded even when it lies outside the
/// current horizon: inbox successors must still start after it, and high-load
/// blocks feed the recovery preference.
#[problem_fact]
pub struct SolverAppliedBlock {
    #[planning_id]
    pub id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// Whether the applied task carried high cognitive load.
    pub high: bool,
    /// Inbox task indexes that must start at or after this block.
    pub successors: Vec<usize>,
}

/// One weekly availability window. The loader canonicalizes each weekday into
/// disjoint intervals, so a minute is contained by at most one fact; split
/// shifts remain separate facts.
#[problem_fact]
pub struct SolverAvailability {
    #[planning_id]
    pub id: String,
    /// Days from Monday, 0..=6.
    pub weekday: u32,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

/// One cognitive preference window for a load level. A task is joined to the
/// window that owns its load level.
#[problem_fact]
pub struct SolverCognitiveWindow {
    #[planning_id]
    pub id: String,
    /// Load level key: 0 low, 1 medium, 2 high.
    pub load: usize,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub outside_penalty: i64,
}
