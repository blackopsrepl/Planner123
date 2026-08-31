use super::*;
impl Worker {
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
}
