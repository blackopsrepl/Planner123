use std::sync::{Arc, Barrier};

use crate::db;
use tempfile::TempDir;

#[test]
fn simultaneous_first_launches_share_one_complete_database() {
    for _ in 0..8 {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("calendar.db");
        let start = Arc::new(Barrier::new(6));
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..6)
                .map(|_| {
                    let start = start.clone();
                    let path = path.clone();
                    scope.spawn(move || {
                        start.wait();
                        let conn = db::open_at(path).unwrap();
                        assert_eq!(db::load_calendars(&conn).unwrap().len(), 1);
                        let integrity: String = conn
                            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                            .unwrap();
                        assert_eq!(integrity, "ok");
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
        });
    }
}
