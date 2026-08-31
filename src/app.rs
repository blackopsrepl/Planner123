use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{Datelike, Duration, Local, NaiveDate};
use tokio::sync::RwLock;

use crate::dag::EventDag;
use crate::google::discovery::DiscoveredGoogleCalendar;
use crate::keys::{Action, View};
use crate::models::{Calendar, Event, EventDependency, Project};
use crate::models::{CognitiveLoad, PlanningTask, TaskPriority};
use crate::sync::state::CalendarSyncState;
use crate::worker::{Worker, WorkerResult};

mod dispatch;
mod event_form;
mod integrations;
mod navigation;
mod planner;
mod query;
mod state;
mod utilities;
use utilities::*;
#[cfg(test)]
mod tests;
mod worker_results;

pub use state::{App, FormField};
