use std::sync::LazyLock;

use solverforge::{prelude::*, SolverConfig, SolverManager};

#[problem_fact]
pub struct SolverSlot {
    #[planning_id]
    pub id: usize,
}

#[planning_entity]
pub struct SolverTask {
    #[planning_id]
    pub id: usize,
    pub task_id: String,
    pub duration_minutes: i64,
    pub priority_weight: i64,
    pub deadline_slot: Option<usize>,
    pub soft_deadline: bool,
    pub high: bool,
    pub blocked_by: Vec<usize>,
    pub slot_minutes: i64,
    pub recovery_minutes: i64,
    pub excess_high_penalty: i64,
    pub feasible: Vec<bool>,
    pub cognitive: Vec<i64>,
    pub external_fatigue: Vec<i64>,
    #[planning_variable(value_range_provider = "slots", allows_unassigned = true)]
    pub start_slot_idx: Option<usize>,
}

impl SolverTask {
    fn valid(&self) -> bool {
        self.start_slot_idx
            .and_then(|idx| self.feasible.get(idx))
            .copied()
            .unwrap_or(false)
    }
    fn start(&self) -> Option<i64> {
        self.start_slot_idx
            .map(|idx| idx as i64 * self.slot_minutes)
    }
    pub fn end(&self) -> Option<i64> {
        self.start().map(|start| start + self.duration_minutes)
    }
    fn overlaps(&self, other: &Self) -> bool {
        matches!((self.start(), self.end(), other.start(), other.end()), (Some(a),Some(b),Some(c),Some(d)) if a < d && c < b)
    }
    fn dependency_late(&self, predecessor: &Self) -> bool {
        self.blocked_by.contains(&predecessor.id)
            && matches!((self.start(), predecessor.end()), (Some(start), Some(end)) if start < end)
    }
    fn cognitive_cost(&self) -> i64 {
        self.start_slot_idx
            .and_then(|idx| self.cognitive.get(idx))
            .copied()
            .unwrap_or(0)
    }
    fn late_minutes(&self) -> i64 {
        match (self.soft_deadline, self.start_slot_idx, self.deadline_slot) {
            (true, Some(start), Some(deadline)) => ((start as i64 * self.slot_minutes
                + self.duration_minutes)
                - deadline as i64 * self.slot_minutes)
                .max(0),
            _ => 0,
        }
    }
    pub fn external_fatigue_cost(&self) -> i64 {
        self.start_slot_idx
            .and_then(|idx| self.external_fatigue.get(idx))
            .copied()
            .unwrap_or(0)
    }
    fn high_load_streak_with(&self, other: &Self) -> bool {
        if !self.high || !other.high {
            return false;
        }
        match (self.start(), self.end(), other.start(), other.end()) {
            (Some(a_start), Some(a_end), Some(b_start), Some(b_end)) => {
                let gap = if a_end <= b_start {
                    b_start - a_end
                } else if b_end <= a_start {
                    a_start - b_end
                } else {
                    0
                };
                gap < self.recovery_minutes
            }
            _ => false,
        }
    }
}

#[planning_solution(
    constraints = "planner_constraints",
    config = "planner_solver_config",
    scalar_groups = "scalar_groups",
    solver_toml = "../planner-solver.toml"
)]
pub struct SolverPlan {
    #[problem_fact_collection]
    pub slots: Vec<SolverSlot>,
    #[planning_entity_collection]
    pub tasks: Vec<SolverTask>,
    #[planning_score]
    pub score: Option<HardMediumSoftScore>,
    pub solve_seconds: u64,
}

fn planner_solver_config(plan: &SolverPlan, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(plan.solve_seconds)
}

#[solverforge_constraints]
fn planner_constraints() -> impl ConstraintSet<SolverPlan, HardMediumSoftScore> {
    let unassigned = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.start_slot_idx.is_none())
        .penalize(|task: &SolverTask| HardMediumSoftScore::of(0, task.priority_weight, 0))
        .named("Prioritize scheduled inbox tasks");
    let invalid = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.start_slot_idx.is_some() && !task.valid())
        .penalize(HardMediumSoftScore::ONE_HARD)
        .named("Only feasible slots");
    let overlap = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::tasks()),
            |left: &SolverTask, right: &SolverTask| left.id < right.id && left.overlaps(right),
        ))
        .penalize(HardMediumSoftScore::ONE_HARD)
        .named("No task overlap");
    let dependencies = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::tasks()),
            |task: &SolverTask, predecessor: &SolverTask| task.dependency_late(predecessor),
        ))
        .penalize(HardMediumSoftScore::ONE_HARD)
        .named("Task dependencies");
    let soft_deadline = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.late_minutes() > 0)
        .penalize(|task: &SolverTask| HardMediumSoftScore::of(0, task.late_minutes(), 0))
        .named("Soft deadlines");
    let cognitive = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.cognitive_cost() > 0)
        .penalize(|task: &SolverTask| HardMediumSoftScore::of(0, 0, task.cognitive_cost()))
        .named("Cognitive timing");
    let fatigue = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::tasks()),
            |left: &SolverTask, right: &SolverTask| {
                left.id < right.id && left.high_load_streak_with(right)
            },
        ))
        .penalize(|left: &SolverTask, _right: &SolverTask| {
            HardMediumSoftScore::of(0, 0, left.excess_high_penalty)
        })
        .named("High cognitive-load recovery");
    let applied_fatigue = ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.external_fatigue_cost() > 0)
        .penalize(|task: &SolverTask| HardMediumSoftScore::of(0, 0, task.external_fatigue_cost()))
        .named("Applied high cognitive-load recovery");
    (
        unassigned,
        invalid,
        overlap,
        dependencies,
        soft_deadline,
        cognitive,
        fatigue,
        applied_fatigue,
    )
}

pub fn scalar_groups() -> Vec<ScalarGroup<SolverPlan>> {
    vec![ScalarGroup::assignment(
        "planner_assignment",
        SolverPlan::tasks().scalar("start_slot_idx"),
    )
    .with_required_entity(|plan, task_idx| plan.tasks[task_idx].feasible.iter().any(|value| *value))
    .with_entity_order(|plan, task_idx| -plan.tasks[task_idx].priority_weight)]
}

pub static PLANNER_MANAGER: LazyLock<SolverManager<SolverPlan>> = LazyLock::new(SolverManager::new);
