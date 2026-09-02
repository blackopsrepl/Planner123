use super::*;
impl Worker {
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
                crate::planner::list_inbox_tasks(&conn).map_err(anyhow::Error::from)
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
                let tasks = crate::planner::list_inbox_tasks(&conn).map_err(anyhow::Error::from)?;
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
}
