//! The incremental score must equal a full re-evaluation after every change.
//!
//! Rule-level tests score whole plans with `evaluate_all`; the solver only
//! ever sees the incremental path. This walks a plan that exercises every
//! rule through a fixed sequence of assign, move, and unassign changes and
//! compares the two after each one.

use crate::planner::constraints::create_constraints;
use crate::planner_domain::test_support::{applied_block, origin, slots, task};
use crate::planner_domain::{
    SolverAvailability, SolverBusy, SolverCognitiveWindow, SolverPlan, SolverTask,
};
use chrono::{Duration, NaiveTime};
use solverforge::{ConstraintSet, ScoreDirector};

fn clock(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
}

fn busy(id: usize, start: i64, end: i64) -> SolverBusy {
    SolverBusy {
        id: format!("busy-{id}"),
        event_id: format!("event-{id}"),
        event_title: "Existing".into(),
        calendar_id: "cal".into(),
        recurring: false,
        start: origin() + Duration::minutes(start),
        end: origin() + Duration::minutes(end),
    }
}

fn plan() -> SolverPlan {
    let mut tasks: Vec<SolverTask> = (0..6)
        .map(|index| {
            let mut task = task(3 + index as i64, 30 + 15 * (index as i64 % 4));
            task.id = index;
            task.index = index;
            task.task_id = format!("task-{index}");
            task.load = index % 3;
            task
        })
        .collect();
    tasks[1].depends_on = vec![0];
    tasks[3].depends_on = vec![1, 2];
    tasks[2].earliest_at = Some(origin() + Duration::minutes(120));
    tasks[4].hard_deadline = Some(origin() + Duration::minutes(600));
    tasks[5].soft_deadline = Some(origin() + Duration::minutes(300));
    tasks[5].applied_predecessor_ends = vec![origin() + Duration::minutes(90)];

    let availability = vec![
        SolverAvailability {
            id: "availability-0".into(),
            weekday: 0,
            start: clock(1, 0),
            end: clock(6, 0),
        },
        SolverAvailability {
            id: "availability-1".into(),
            weekday: 0,
            start: clock(8, 0),
            end: clock(20, 0),
        },
    ];
    let cognitive = (0..3)
        .map(|load| SolverCognitiveWindow {
            id: format!("cognitive-{load}"),
            load,
            start: clock(2 + load as u32 * 3, 0),
            end: clock(5 + load as u32 * 3, 30),
            outside_penalty: 1 + load as i64,
        })
        .collect();
    SolverPlan::new(
        slots(48),
        vec![busy(0, 60, 150), busy(1, 240, 270), busy(2, 600, 720)],
        vec![applied_block(30, 90, true, vec![5])],
        availability,
        cognitive,
        tasks,
        1,
    )
}

#[test]
fn incremental_score_matches_full_evaluation_after_every_change() {
    let mut director = ScoreDirector::new(plan(), create_constraints());
    director.calculate_score();

    // A deterministic walk over tasks and slots, including unassignment.
    let mut state = 7u64;
    for step in 0..400 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let task = (state >> 33) as usize % 6;
        let value = match (state >> 17) % 9 {
            0 => None,
            v => Some(((state >> 40) as usize + v as usize) % 48),
        };
        let incremental = director.do_change(0, task, |plan| plan.tasks[task].start_idx = value);
        let full = create_constraints().evaluate_all(director.working_solution());
        assert_eq!(
            incremental, full,
            "step {step}: task {task} -> {value:?} drifted from full evaluation"
        );
    }
}
