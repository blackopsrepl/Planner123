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

/// A blocked interval that no task may overlap: an existing event, a recurring
/// occurrence, or a previously applied high-load block.
#[problem_fact]
pub struct SolverBusy {
    #[planning_id]
    pub id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// Whether this interval is a high cognitive-load block from an applied
    /// proposal. Used only by the recovery preference rules.
    pub high: bool,
    /// Inbox task indexes that must start at or after this interval. Populated
    /// for already-applied predecessors.
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
