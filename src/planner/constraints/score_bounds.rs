use crate::planner::PlannerError;
use crate::planner_domain::{SolverCognitiveWindow, SolverTask};

/// Proves the soft-score magnitude cannot overflow during constraint
/// evaluation. Cognitive cost is bounded by every task minute falling outside
/// its window; recovery can charge each high-load task at most once.
pub(crate) fn validate_soft_score_range(
    tasks: &[SolverTask],
    windows: &[SolverCognitiveWindow],
) -> Result<(), PlannerError> {
    let cognitive: i128 = tasks
        .iter()
        .map(|task| {
            let penalty = windows
                .iter()
                .find(|window| window.load == task.load)
                .map_or(0, |window| window.outside_penalty);
            i128::from(task.duration_minutes) * i128::from(penalty)
        })
        .sum();
    let recovery: i128 = tasks
        .iter()
        .filter(|task| task.load == 2)
        .map(|task| i128::from(task.excess_high_penalty))
        .sum();
    if cognitive + recovery > i128::from(i64::MAX) {
        return Err(PlannerError::Validation(
            "planner cognitive and recovery weights exceed the supported score range".into(),
        ));
    }
    Ok(())
}
