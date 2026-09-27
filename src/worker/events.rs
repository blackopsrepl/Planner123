use super::*;
impl Worker {
    /* Load events overlapping the inclusive date window `[start, end]`.

    The caller owns window selection so navigation can compare the window it
    just loaded against the window the current view needs. Returns the request's
    sequence id so the app can discard superseded responses. */
    pub fn load_events(&self, start: chrono::NaiveDate, end: chrono::NaiveDate) -> u64 {
        let tx = self.tx.clone();
        let seq = self
            .load_seq
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        self.rt.spawn_blocking(move || {
            let result = (|| -> Result<_> {
                let conn = crate::db::open()?;
                let from_str = format!("{} 00:00:00", start);
                let to_str = format!("{} 23:59:59", end);
                let events = crate::db::load_events_in_range(&conn, &from_str, &to_str)?;
                Ok(events)
            })();
            match result {
                Ok(events) => {
                    let _ = tx.send(WorkerResult::EventsLoaded {
                        seq,
                        start,
                        end,
                        events,
                    });
                }
                Err(e) => {
                    let _ = tx.send(WorkerResult::Error(e.to_string()));
                }
            }
        });
        seq
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
