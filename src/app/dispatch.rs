use super::*;
impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        let action = crate::keys::resolve(&self.view, key);
        self.dispatch(action);
    }

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::Help => self.view = View::Help,
            Action::Escape => self.handle_escape(),

            // View switching
            Action::ViewMonth => self.switch_view(View::Month),
            Action::ViewWeek => self.switch_view(View::Week),
            Action::ViewDay => self.switch_view(View::Day),
            Action::ViewAgenda => self.switch_view(View::Agenda),
            Action::PlannerInbox => {
                self.view = View::PlannerInbox;
                self.worker.load_planner_tasks();
                self.worker.load_planner_settings();
            }

            // Focus
            Action::FocusSidebar => {
                self.sidebar_focused = true;
                self.view = View::CalendarList;
            }
            Action::FocusMain => {
                self.sidebar_focused = false;
                self.view = View::Month; // return to last main view (simplified)
            }

            // Time navigation
            Action::PrevPeriod => self.prev_period(),
            Action::NextPeriod => self.next_period(),
            Action::PrevUnit => self.prev_unit(),
            Action::NextUnit => self.next_unit(),
            Action::PrevDay => self.move_day(-1),
            Action::NextDay => self.move_day(1),
            Action::JumpToday => self.jump_today(),

            // Event actions
            Action::CreateEvent => self.open_event_form(None),
            Action::EditEvent => self.open_edit_form(),
            Action::DeleteEvent => self.delete_selected_event(),
            Action::SelectEvent => self.select_event(),

            // Form
            Action::FormNextField => match self.view {
                View::GoogleAuth => {
                    if self.google_auth_field < 1 {
                        self.google_auth_field += 1;
                    }
                }
                View::IcalImport => self.ical_import_next_field(),
                View::PlannerTaskForm => self.planner_task_next_field(),
                View::PlannerSettingsForm => self.planner_settings_next_field(),
                _ => self.form_next_field(),
            },
            Action::FormPrevField => match self.view {
                View::GoogleAuth => {
                    if self.google_auth_field > 0 {
                        self.google_auth_field -= 1;
                    }
                }
                View::IcalImport => self.ical_import_prev_field(),
                View::PlannerTaskForm => self.planner_task_prev_field(),
                View::PlannerSettingsForm => self.planner_settings_prev_field(),
                _ => self.form_prev_field(),
            },
            Action::FormSubmit => match self.view {
                View::GoogleAuth => self.handle_input_submit(),
                View::IcalImport => self.ical_import_submit(),
                View::PlannerTaskForm => self.planner_task_submit(),
                View::PlannerSettingsForm => self.planner_settings_submit(),
                _ => self.form_submit(),
            },
            Action::FormCancel => self.handle_escape(),
            Action::InputChar(c) => {
                if self.view == View::PlannerTaskForm {
                    self.planner_task_input_char(c)
                } else if self.view == View::PlannerSettingsForm {
                    self.planner_settings_input_char(c)
                } else {
                    self.form_input_char(c)
                }
            }
            Action::InputBackspace => {
                if self.view == View::PlannerTaskForm {
                    self.planner_task_input_backspace()
                } else if self.view == View::PlannerSettingsForm {
                    self.planner_settings_input_backspace()
                } else {
                    self.form_input_backspace()
                }
            }
            Action::InputSubmit => self.handle_input_submit(),
            Action::InputCancel => self.handle_escape(),

            // Sidebar
            Action::CalendarUp => {
                if self.view == View::GoogleManage {
                    if self.google_discovery_index > 0 {
                        self.google_discovery_index -= 1;
                    }
                } else if self.calendar_list_index > 0 {
                    self.calendar_list_index -= 1;
                }
            }
            Action::CalendarDown => {
                if self.view == View::GoogleManage {
                    if self.google_discovery_index + 1 < self.google_discovered_calendars.len() {
                        self.google_discovery_index += 1;
                    }
                } else if self.calendar_list_index + 1 < self.calendars.len() {
                    self.calendar_list_index += 1;
                }
            }
            Action::ToggleCalendar => self.toggle_calendar_visibility(),

            // Scroll
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),
            Action::ScrollPageUp => self.scroll_page(-10),
            Action::ScrollPageDown => self.scroll_page(10),

            // Quick add
            Action::QuickAdd => {
                self.quick_add_input.clear();
                self.view = View::QuickAdd;
            }

            // Google
            Action::GoogleManage => self.google_manage(),
            Action::GoogleSync => self.google_sync(),
            Action::GoogleDiscoverCalendars => self.discover_google_calendars(),
            Action::GoogleImportCalendar => self.import_selected_google_calendar(),
            Action::GoogleLogin => self.view = View::GoogleAuth,
            Action::GoogleAuthLogout => self.google_logout(),

            // iCal
            Action::ImportIcal => self.open_ical_import(),
            Action::ExportIcal => self.export_ical(),

            Action::CreateTask => self.open_planner_task_form(),
            Action::PlannerOptimize => {
                if let Some(message) = self.planner_optimization_prerequisite() {
                    self.set_status(message, true);
                    self.open_planner_settings_form();
                    return;
                }
                self.loading = true;
                self.worker.optimize_planner();
            }
            Action::PlannerApply => {
                if let Some(proposal) = &self.planner_proposal {
                    if proposal.proposal.status == "ready" {
                        self.loading = true;
                        self.worker
                            .apply_planner_proposal(proposal.proposal.id.clone());
                    } else {
                        self.set_status("This planner proposal has already been applied.", true);
                    }
                } else {
                    self.set_status("No proposal is ready to apply.", true);
                }
            }
            Action::PlannerSettings => self.open_planner_settings_form(),

            Action::None | Action::JumpToDate => {}
        }
    }

    // ── Handle tick (animations, worker poll) ─────────────────────
}
