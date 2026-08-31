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
}

impl Worker {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx, rt }
    }

    /// Drain all pending results without blocking.
    pub fn drain(&self) -> Vec<WorkerResult> {
        let mut results = Vec::new();
        while let Ok(r) = self.rx.try_recv() {
            results.push(r);
        }
        results
    }

    // ── Database tasks (run on tokio's blocking thread pool) ─────────

    pub fn load_calendars(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::db::load_calendars(&conn)
            })();
            match result {
                Ok(cals) => {
                    let _ = tx.send(WorkerResult::CalendarsLoaded(cals));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn load_calendar_sync_states(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::sync::state::load_calendar_sync_states(&conn)
            })();
            match result {
                Ok(states) => {
                    let _ = tx.send(WorkerResult::CalendarSyncStatesLoaded(states));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn load_projects(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::db::load_projects(&conn)
            })();
            match result {
                Ok(projs) => {
                    let _ = tx.send(WorkerResult::ProjectsLoaded(projs));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn load_dependencies(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::db::load_dependencies(&conn)
            })();
            match result {
                Ok(deps) => {
                    let _ = tx.send(WorkerResult::DependenciesLoaded(deps));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn load_planner_tasks(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::planner::list_tasks(&conn).map_err(anyhow::Error::from)
            })();
            match result {
                Ok(tasks) => {
                    let _ = tx.send(WorkerResult::PlannerTasksLoaded(tasks));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(error.to_string()));
                }
            }
        });
    }

    pub fn load_planner_settings(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::planner::settings(&conn).map_err(Into::into)
            })();
            match result {
                Ok(settings) => {
                    let _ = tx.send(WorkerResult::PlannerSettingsLoaded(settings));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(error.to_string()));
                }
            }
        });
    }

    pub fn update_planner_settings(&self, update: crate::planner::SettingsUpdate) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::planner::update_settings(&conn, update).map_err(Into::into)
            })();
            match result {
                Ok(settings) => {
                    let _ = tx.send(WorkerResult::PlannerSettingsLoaded(settings));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(error.to_string()));
                }
            }
        });
    }

    pub fn optimize_planner(&self) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::planner::optimize(&conn, None).map_err(anyhow::Error::from)
            })();
            match result {
                Ok(proposal) => {
                    let _ = tx.send(WorkerResult::PlannerProposalReady(proposal));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(error.to_string()));
                }
            }
        });
    }

    pub fn create_planner_task(&self, input: crate::planner::CreateTaskInput) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                let task =
                    crate::planner::create_task(&conn, input).map_err(anyhow::Error::from)?;
                let tasks = crate::planner::list_tasks(&conn).map_err(anyhow::Error::from)?;
                Ok::<_, anyhow::Error>((task, tasks))
            })();
            match result {
                Ok((task, tasks)) => {
                    let _ = tx.send(WorkerResult::StatusMessage(format!(
                        "Task added to Planner Inbox: {}",
                        task.title
                    )));
                    let _ = tx.send(WorkerResult::PlannerTasksLoaded(tasks));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(error.to_string()));
                }
            }
        });
    }

    pub fn apply_planner_proposal(&self, proposal_id: String) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::planner::apply_proposal(&conn, &proposal_id).map_err(anyhow::Error::from)
            })();
            match result {
                Ok(proposal) => {
                    let _ = tx.send(WorkerResult::PlannerProposalApplied(proposal));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(error.to_string()));
                }
            }
        });
    }

    /// Load events for the visible date range + 2-week buffer around it.
    pub fn load_events(&self, year: i32, month: u32) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                // Window: month - 7 days to month + 37 days (covers 5-week grid + next month)
                let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
                    .unwrap_or_default()
                    .pred_opt()
                    .unwrap_or_default()
                    .pred_opt()
                    .unwrap_or_default();
                let end = chrono::NaiveDate::from_ymd_opt(
                    if month == 12 { year + 1 } else { year },
                    if month == 12 { 1 } else { month + 1 },
                    1,
                )
                .unwrap_or_default();

                let from_str = format!("{} 00:00:00", start);
                let to_str = format!("{} 23:59:59", end);
                let events = crate::db::load_events_in_range(&conn, &from_str, &to_str)?;
                Ok(events)
            })();
            match result {
                Ok(events) => {
                    let _ = tx.send(WorkerResult::EventsLoaded { events });
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn save_event(&self, event: Event, is_new: bool) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::event_service::save_event(&conn, event, is_new).map_err(Into::into)
            })();
            match result {
                Ok(ev) => {
                    let _ = tx.send(WorkerResult::EventSaved(ev));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn delete_event(&self, event_id: String) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::event_service::delete_event(&conn, &event_id)?;
                Ok(event_id)
            })();
            match result {
                Ok(id) => {
                    let _ = tx.send(WorkerResult::EventDeleted(id));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn complete_google_auth(&self, client_id: String, client_secret: String) {
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let client_secret = if client_secret.trim().is_empty() {
                None
            } else {
                Some(client_secret.as_str())
            };
            match crate::google::auth::authorize_and_persist(&client_id, client_secret).await {
                Ok(client) => {
                    let _ = tx.send(WorkerResult::GoogleAuthComplete(Arc::new(client)));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(format!(
                        "Google authorization failed: {}",
                        e
                    )));
                }
            }
        });
    }

    pub fn discover_google_calendars(
        &self,
        google_client: std::sync::Arc<crate::google::auth::GoogleClient>,
    ) {
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            match crate::google::discovery::discover_calendars(google_client.as_ref()).await {
                Ok(calendars) => {
                    let _ = tx.send(WorkerResult::GoogleCalendarsDiscovered(calendars));
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(format!(
                        "Google calendar discovery failed: {}",
                        e
                    )));
                }
            }
        });
    }

    /// Trigger a Google Calendar sync for all Google-sourced calendars.
    pub fn google_sync(
        &self,
        calendars: Vec<Calendar>,
        google_client: std::sync::Arc<crate::google::auth::GoogleClient>,
    ) {
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let mut calendars_succeeded = 0usize;
            let mut calendars_failed = 0usize;
            let mut events_added = 0usize;
            let mut events_updated = 0usize;
            let mut conflicts_detected = 0usize;
            for cal in calendars
                .iter()
                .filter(|c| c.source == crate::models::CalendarSource::Google)
            {
                match crate::google::sync::sync_calendar(google_client.as_ref(), cal).await {
                    Ok(report) => {
                        calendars_succeeded += 1;
                        events_added += report.events_added;
                        events_updated += report.events_updated + report.pushed_updates;
                        conflicts_detected += report.conflicts_detected;
                    }
                    Err(e) => {
                        calendars_failed += 1;
                        if let Ok(conn) = crate::db::open() {
                            let _ = crate::sync::engine::mark_calendar_sync_error(
                                &conn,
                                cal,
                                &e.to_string(),
                            );
                        }
                    }
                }
            }
            let _ = tx.send(WorkerResult::GoogleSyncFinished {
                calendars_succeeded,
                calendars_failed,
                events_added,
                events_updated,
                conflicts_detected,
            });
        });
    }

    pub fn import_ical(&self, calendar_id: String, path: String, timezone: String) {
        let tx = self.tx.clone();
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                crate::ical::import_from_file(
                    &conn,
                    &calendar_id,
                    std::path::Path::new(&path),
                    &timezone,
                )
            })();
            match result {
                Ok(report) => {
                    let _ = tx.send(WorkerResult::IcalImported(report));
                }
                Err(error) => {
                    let _ = tx.send(WorkerResult::Error(format!(
                        "iCal import failed: {}",
                        error
                    )));
                }
            }
        });
    }
}
