use super::*;
impl Worker {
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
