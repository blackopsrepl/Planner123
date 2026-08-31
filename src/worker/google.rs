use super::*;
impl Worker {
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
}
