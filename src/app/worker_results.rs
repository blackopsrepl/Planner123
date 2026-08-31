use super::*;
impl App {
    pub fn handle_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Poll worker for results
        for result in self.worker.drain() {
            self.handle_worker_result(result);
        }
    }

    fn handle_worker_result(&mut self, result: WorkerResult) {
        match result {
            WorkerResult::CalendarsLoaded(cals) => {
                self.calendars = cals;
                // Now load events for the current view window
                self.worker.load_events(self.view_year, self.view_month);
                self.loading = false; // spinner shows until EventsLoaded arrives
            }
            WorkerResult::CalendarSyncStatesLoaded(states) => {
                self.calendar_sync_state = states
                    .into_iter()
                    .map(|state| (state.calendar_id.clone(), state))
                    .collect();
            }
            WorkerResult::ProjectsLoaded(projs) => {
                self.projects = projs;
            }
            WorkerResult::EventsLoaded { events } => {
                self.events = events.clone();
                // Update shared arc for notification task
                let arc = self.events_arc.clone();
                tokio::spawn(async move {
                    *arc.write().await = events;
                });
                self.loading = false;
            }
            WorkerResult::DependenciesLoaded(deps) => {
                self.dag = EventDag::from_dependencies(&deps);
                self.dependencies = deps;
            }
            WorkerResult::PlannerTasksLoaded(tasks) => {
                self.planner_tasks = tasks;
                self.planner_selected_index = self
                    .planner_selected_index
                    .min(self.planner_tasks.len().saturating_sub(1));
            }
            WorkerResult::PlannerSettingsLoaded(settings) => {
                self.planner_settings_timezone = settings
                    .timezone
                    .clone()
                    .unwrap_or_else(crate::time::local_timezone_name);
                self.planner_settings_availability = serde_json::from_str::<
                    crate::planner::Availability,
                >(&settings.availability_json)
                .map(|availability| availability_to_specs(&availability))
                .ok()
                .filter(|availability| !availability.is_empty())
                .unwrap_or_else(default_weekly_availability);
                self.planner_settings_horizon_days = settings.horizon_days.to_string();
                self.planner_settings_slot_minutes = settings.slot_minutes.to_string();
                self.planner_settings_solve_seconds = settings.solve_seconds.to_string();
                self.planner_settings = Some(settings);
                self.loading = false;
            }
            WorkerResult::PlannerProposalReady(proposal) => {
                self.planner_proposal = Some(proposal);
                self.loading = false;
                self.set_status(
                    "Planner proposal is ready for review; press a to apply.",
                    false,
                );
            }
            WorkerResult::PlannerProposalApplied(proposal) => {
                self.planner_proposal = Some(proposal);
                self.worker.load_planner_tasks();
                self.worker.load_events(self.view_year, self.view_month);
                self.loading = false;
                self.set_status("Scheduled planner tasks were applied.", false);
            }
            WorkerResult::EventSaved(ev) => {
                // Refresh events for the current window
                self.worker.load_events(self.view_year, self.view_month);
                self.set_status(format!("Saved: {}", ev.title), false);
                self.view = View::Month;
            }
            WorkerResult::EventDeleted(id) => {
                self.events.retain(|e| e.id != id);
                self.set_status("Event deleted.", false);
            }
            WorkerResult::GoogleAuthComplete(client) => {
                self.google_client = Some(client);
                self.set_status("Google authorization complete.", false);
                self.view = View::GoogleManage;
                self.worker.load_calendar_sync_states();
                self.discover_google_calendars();
                self.loading = false;
            }
            WorkerResult::GoogleCalendarsDiscovered(calendars) => {
                self.google_discovered_calendars = calendars;
                if self.google_discovery_index >= self.google_discovered_calendars.len() {
                    self.google_discovery_index =
                        self.google_discovered_calendars.len().saturating_sub(1);
                }
                self.view = View::GoogleManage;
                self.loading = false;
                self.set_status("Google calendar discovery updated.", false);
            }
            WorkerResult::GoogleSyncFinished {
                calendars_succeeded,
                calendars_failed,
                events_added,
                events_updated,
                conflicts_detected,
            } => {
                self.worker.load_events(self.view_year, self.view_month);
                self.worker.load_calendar_sync_states();
                let (status, is_error) = google_sync_finished_status(
                    calendars_succeeded,
                    calendars_failed,
                    events_added,
                    events_updated,
                    conflicts_detected,
                );
                self.set_status(status, is_error);
                self.loading = false;
            }
            WorkerResult::IcalImported(report) => {
                self.worker.load_events(self.view_year, self.view_month);
                self.worker.load_calendar_sync_states();
                self.view = View::Month;
                let warning_suffix = if report.warnings.is_empty() {
                    String::new()
                } else {
                    format!(" {} warnings.", report.warnings.len())
                };
                self.set_status(
                    format!(
                        "Imported {} events from {}.{}",
                        report.imported, report.path, warning_suffix
                    ),
                    false,
                );
                self.loading = false;
            }
            WorkerResult::StatusMessage(msg) => {
                self.set_status(msg, false);
                self.loading = false;
            }
            WorkerResult::Error(e) => {
                self.set_status(e, true);
                self.loading = false;
            }
        }
    }

    // ── Navigation helpers ────────────────────────────────────────
}
