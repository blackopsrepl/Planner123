use std::sync::{mpsc, Arc};

use anyhow::Result;

use crate::models::{Calendar, Event, EventDependency, Project};

/* Results sent back from background tasks. */
#[derive(Debug)]
pub enum WorkerResult {
    CalendarsLoaded(Vec<Calendar>),
    CalendarSyncStatesLoaded(Vec<crate::sync::state::CalendarSyncState>),
    ProjectsLoaded(Vec<Project>),
    EventsLoaded {
        seq: u64,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
        events: Vec<Event>,
    },
    DependenciesLoaded(Vec<EventDependency>),
    EventSaved(Event),
    EventDeleted(String),
    GoogleAuthComplete(Arc<crate::google::auth::GoogleClient>),
    GoogleCalendarsDiscovered(Vec<crate::google::discovery::DiscoveredGoogleCalendar>),
    GoogleSyncFinished {
        calendars_succeeded: usize,
        calendars_failed: usize,
        events_added: usize,
        events_updated: usize,
        conflicts_detected: usize,
    },
    IcalImported(crate::ical::ImportReport),
    Error(String),
    StatusMessage(String),
    PlannerTasksLoaded(Vec<crate::models::PlanningTask>),
    PlannerSettingsLoaded(crate::models::PlannerSettings),
    PlannerProposalReady(crate::planner::ProposalDetail),
    PlannerProposalApplied(crate::planner::ProposalDetail),
}

pub struct Worker {
    tx: mpsc::Sender<WorkerResult>,
    pub rx: mpsc::Receiver<WorkerResult>,
    rt: tokio::runtime::Handle,
    /* Monotonic id assigned to each event load so the app can discard results
    that arrive after a newer window was requested. */
    load_seq: std::sync::atomic::AtomicU64,
}

impl Worker {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            rt,
            load_seq: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Drain all pending results without blocking.
    pub fn drain(&self) -> Vec<WorkerResult> {
        let mut results = Vec::new();
        while let Ok(r) = self.rx.try_recv() {
            results.push(r);
        }
        results
    }
}

mod events;
mod google;
mod ical;
mod planner;
