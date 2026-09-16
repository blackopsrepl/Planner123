//! Planning solution for the calendar planner.
//!
//! `SolverPlan` is built by the loader from persisted state and handed to the
//! SolverForge runtime. It owns no scheduling policy: facts and tasks carry
//! domain data, and every rule lives in `crate::planner::constraints`.

use std::sync::LazyLock;

use solverforge::{prelude::*, SolverConfig, SolverManager};

use super::{SolverAvailability, SolverBusy, SolverCognitiveWindow, SolverSlot, SolverTask};

/// Full planning solution passed to the solver runtime.
#[planning_solution(
    constraints = "crate::planner::constraints::create_constraints",
    config = "planner_solver_config",
    solver_toml = "../../planner-solver.toml"
)]
pub struct SolverPlan {
    /// Candidate start instants; value range for `SolverTask.start_idx`.
    #[problem_fact_collection]
    pub slots: Vec<SolverSlot>,
    /// Intervals no task may overlap.
    #[problem_fact_collection]
    pub busy: Vec<SolverBusy>,
    /// Raw weekly availability windows.
    #[problem_fact_collection]
    pub availability: Vec<SolverAvailability>,
    /// Cognitive preference windows per load level.
    #[problem_fact_collection]
    pub cognitive_windows: Vec<SolverCognitiveWindow>,
    /// Inbox tasks the solver places.
    #[planning_entity_collection]
    pub tasks: Vec<SolverTask>,
    #[planning_score]
    pub score: Option<HardMediumSoftScore>,
    /// Per-solve wall-clock budget; applied by `planner_solver_config`.
    pub solve_seconds: u64,
}

impl SolverPlan {
    /// Builds a normalized plan from facts and task entities.
    pub fn new(
        slots: Vec<SolverSlot>,
        busy: Vec<SolverBusy>,
        availability: Vec<SolverAvailability>,
        cognitive_windows: Vec<SolverCognitiveWindow>,
        tasks: Vec<SolverTask>,
        solve_seconds: u64,
    ) -> Self {
        let mut plan = Self {
            slots,
            busy,
            availability,
            cognitive_windows,
            tasks,
            score: None,
            solve_seconds,
        };
        plan.rebuild_derived_fields();
        plan
    }

    /// Recomputes dense indexes and drops out-of-range scalar assignments after
    /// transport decoding.
    pub fn rebuild_derived_fields(&mut self) {
        for (index, task) in self.tasks.iter_mut().enumerate() {
            task.index = index;
            task.start_idx = task.start_idx.filter(|idx| *idx < self.slots.len());
        }
    }
}

/// Applies the per-solve time budget on top of the embedded `solver.toml`.
fn planner_solver_config(plan: &SolverPlan, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(plan.solve_seconds)
}

/// Global retained solver manager, mirroring the reference apps.
pub static PLANNER_MANAGER: LazyLock<SolverManager<SolverPlan>> = LazyLock::new(SolverManager::new);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};

    #[test]
    fn rebuild_filters_out_of_range_assignments() {
        let mut plan = SolverPlan::new(slots(4), vec![], vec![], vec![], vec![task(3, 60)], 1);
        plan.tasks[0].start_idx = Some(99);
        plan.rebuild_derived_fields();
        assert_eq!(plan.tasks[0].start_idx, None);
        assert_eq!(plan.tasks[0].index, 0);
    }
}
