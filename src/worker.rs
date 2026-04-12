use std::sync::{mpsc, Arc};

use anyhow::Result;

use crate::models::{Calendar, Event, EventDependency, Project};

/* Results sent back from background tasks. */
#[derive(Debug)]
pub enum WorkerResult {
    CalendarsLoaded(Vec<Calendar>),
    ProjectsLoaded(Vec<Project>),
    EventsLoaded {
        events: Vec<Event>,
    },
    DependenciesLoaded(Vec<EventDependency>),
    EventSaved(Box<Event>),
    EventDeleted(String),
    GoogleAuthComplete(Arc<crate::google::auth::GoogleClient>),
    GoogleSyncComplete {
        events_added: usize,
        events_updated: usize,
    },
    Error(String),
    StatusMessage(String),
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
                    let _ = tx.send(WorkerResult::EventSaved(Box::new(ev)));
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
            match crate::google::auth::authorize_and_persist(&client_id, &client_secret).await {
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

    /// Trigger a Google Calendar sync for all Google-sourced calendars.
    pub fn google_sync(
        &self,
        calendars: Vec<Calendar>,
        google_client: std::sync::Arc<crate::google::auth::GoogleClient>,
    ) {
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            for cal in calendars
                .iter()
                .filter(|c| c.source == crate::models::CalendarSource::Google)
            {
                match crate::google::sync::sync_calendar(google_client.as_ref(), cal).await {
                    Ok((added, updated)) => {
                        let _ = tx.send(WorkerResult::GoogleSyncComplete {
                            events_added: added,
                            events_updated: updated,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(WorkerResult::Error(format!(
                            "Google sync failed for '{}': {}",
                            cal.name, e
                        )));
                    }
                }
            }
            let _ = tx.send(WorkerResult::StatusMessage(
                "Google sync complete.".to_string(),
            ));
        });
    }
}
