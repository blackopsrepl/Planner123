/* Library re-exports for test access.  */

pub mod app;
pub mod calendar_service;
pub mod cli;
pub mod dag;
pub mod db;
pub mod event;
pub mod event_service;
pub mod google;
pub mod ical;
pub mod keys;
pub mod models;
pub mod notifications;
pub mod planner;
solverforge::planning_model! {
    root = "src";

    mod planner_domain;

    pub use planner_domain::{SolverPlan, SolverSlot, SolverTask, PLANNER_MANAGER};
}
pub mod recurrence;
pub mod sync;
pub mod theme;
pub mod time;
pub mod ui;
pub mod worker;
