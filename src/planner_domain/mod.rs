//! Planning-model manifest and public exports for the calendar planner.
//!
//! Keep the module list and exports in the same conceptual order the loader
//! builds them: facts, planning entity, then the solution.

solverforge::planning_model! {
    root = "src/planner_domain";

    mod facts;
    mod task;
    mod plan;

    pub use facts::{SolverAvailability, SolverBusy, SolverCognitiveWindow, SolverSlot};
    pub use task::SolverTask;
    pub use plan::{SolverPlan, PLANNER_MANAGER};
}

#[cfg(test)]
pub mod test_support;
