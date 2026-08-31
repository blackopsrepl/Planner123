use super::*;
impl App {
    pub fn open_event_form(&mut self, event: Option<&Event>) {
        match event {
            None => {
                // New event defaults
                self.form_is_new = true;
                self.form_editing_event = None;
                self.form_title.clear();
                self.form_date = self.focused_date.format("%Y-%m-%d").to_string();
                self.form_start_time = "09:00".to_string();
                self.form_end_time = "10:00".to_string();
                self.form_location.clear();
                self.form_description.clear();
                self.form_rrule.clear();
                self.form_reminder = "15".to_string();
                self.form_all_day = false;
                self.form_calendar_index = 0;
                self.form_timezone = crate::time::local_timezone_name();
                self.form_project_index = 0;
                self.form_recurrence_index = 0;
            }
            Some(ev) => {
                self.form_is_new = false;
                self.form_editing_event = Some(ev.clone());
                self.form_title = ev.title.clone();
                self.form_date = ev.start_at[..10].to_string();
                self.form_start_time = if ev.start_at.len() >= 16 {
                    ev.start_at[11..16].to_string()
                } else {
                    "09:00".to_string()
                };
                self.form_end_time = if ev.end_at.len() >= 16 {
                    ev.end_at[11..16].to_string()
                } else {
                    "10:00".to_string()
                };
                self.form_location = ev.location.clone().unwrap_or_default();
                self.form_description = ev.description.clone().unwrap_or_default();
                self.form_rrule = ev.rrule.clone().unwrap_or_default();
                self.form_reminder = ev
                    .reminder_minutes
                    .map(|m| m.to_string())
                    .unwrap_or_default();
                self.form_all_day = ev.all_day;
                self.form_calendar_index = self
                    .calendars
                    .iter()
                    .position(|c| c.id == ev.calendar_id)
                    .unwrap_or(0);
                self.form_timezone = ev.timezone.clone();
                self.form_project_index = ev
                    .project_id
                    .as_ref()
                    .and_then(|pid| self.projects.iter().position(|p| p.id == *pid))
                    .map(|i| i + 1)
                    .unwrap_or(0);
            }
        }
        self.form_field_index = 0;
        self.view = View::EventForm;
    }

    pub(super) fn open_edit_form(&mut self) {
        if let Some(event) = self.selected_event().cloned() {
            self.open_event_form(Some(&event));
        } else {
            self.set_status("No event selected.", false);
        }
    }

    pub(super) fn form_next_field(&mut self) {
        if self.form_field_index + 1 < self.form_fields.len() {
            self.form_field_index += 1;
        }
    }

    pub(super) fn form_prev_field(&mut self) {
        if self.form_field_index > 0 {
            self.form_field_index -= 1;
        }
    }

    pub(super) fn form_input_char(&mut self, c: char) {
        match self.view {
            View::QuickAdd => self.quick_add_input.push(c),
            View::GoogleAuth => {
                if self.google_auth_field == 0 {
                    self.google_auth_client_id.push(c);
                } else {
                    self.google_auth_client_secret.push(c);
                }
            }
            View::IcalImport => self.ical_import_input_char(c),
            View::EventForm => self.form_active_field_push(c),
            _ => {}
        }
    }

    pub(super) fn form_input_backspace(&mut self) {
        match self.view {
            View::QuickAdd => {
                self.quick_add_input.pop();
            }
            View::GoogleAuth => {
                if self.google_auth_field == 0 {
                    self.google_auth_client_id.pop();
                } else {
                    self.google_auth_client_secret.pop();
                }
            }
            View::IcalImport => self.ical_import_input_backspace(),
            View::EventForm => self.form_active_field_pop(),
            _ => {}
        }
    }

    pub(super) fn form_active_field_push(&mut self, c: char) {
        match self.form_fields.get(self.form_field_index) {
            Some(FormField::Title) => self.form_title.push(c),
            Some(FormField::Date) => self.form_date.push(c),
            Some(FormField::StartTime) => self.form_start_time.push(c),
            Some(FormField::EndTime) => self.form_end_time.push(c),
            Some(FormField::Location) => self.form_location.push(c),
            Some(FormField::Description) => self.form_description.push(c),
            Some(FormField::Timezone) => self.form_timezone.push(c),
            Some(FormField::Recurrence) => self.form_rrule.push(c),
            Some(FormField::Reminder) if c.is_ascii_digit() => self.form_reminder.push(c),
            Some(FormField::Calendar) => {
                // Cycle through calendars with +/-
                if c == '+' || c == 'l' {
                    if self.form_calendar_index + 1 < self.calendars.len() {
                        self.form_calendar_index += 1;
                    }
                } else if (c == '-' || c == 'h') && self.form_calendar_index > 0 {
                    self.form_calendar_index -= 1;
                }
            }
            Some(FormField::Project) => {
                if c == '+' || c == 'l' {
                    if self.form_project_index < self.projects.len() {
                        self.form_project_index += 1;
                    }
                } else if (c == '-' || c == 'h') && self.form_project_index > 0 {
                    self.form_project_index -= 1;
                }
            }
            Some(FormField::AllDay) if c == ' ' => self.form_all_day = !self.form_all_day,
            _ => {}
        }
    }

    pub(super) fn form_active_field_pop(&mut self) {
        match self.form_fields.get(self.form_field_index) {
            Some(FormField::Title) => {
                self.form_title.pop();
            }
            Some(FormField::Date) => {
                self.form_date.pop();
            }
            Some(FormField::StartTime) => {
                self.form_start_time.pop();
            }
            Some(FormField::EndTime) => {
                self.form_end_time.pop();
            }
            Some(FormField::Location) => {
                self.form_location.pop();
            }
            Some(FormField::Description) => {
                self.form_description.pop();
            }
            Some(FormField::Timezone) => {
                self.form_timezone.pop();
            }
            Some(FormField::Recurrence) => {
                self.form_rrule.pop();
            }
            Some(FormField::Reminder) => {
                self.form_reminder.pop();
            }
            _ => {}
        }
    }

    pub(super) fn form_submit(&mut self) {
        if self.form_title.trim().is_empty() {
            self.set_status("Title cannot be empty.", true);
            return;
        }

        let cal_id = self
            .calendars
            .get(self.form_calendar_index)
            .map(|c| c.id.clone())
            .unwrap_or_default();
        if cal_id.is_empty() {
            self.set_status("No calendar selected.", true);
            return;
        }

        let start_at = format!("{} {}:00", self.form_date, self.form_start_time);
        let end_at = format!("{} {}:00", self.form_date, self.form_end_time);
        let timezone = self.form_timezone.trim().to_string();

        let mut event = if let Some(existing) = &self.form_editing_event {
            existing.clone()
        } else {
            Event::new(&cal_id, &self.form_title, &start_at, &end_at, &timezone)
        };

        event.calendar_id = cal_id;
        event.title = self.form_title.clone();
        event.start_at = start_at;
        event.end_at = end_at;
        event.all_day = self.form_all_day;
        event.timezone = timezone;
        event.location = if self.form_location.is_empty() {
            None
        } else {
            Some(self.form_location.clone())
        };
        event.description = if self.form_description.is_empty() {
            None
        } else {
            Some(self.form_description.clone())
        };
        event.rrule = if self.form_rrule.is_empty() {
            None
        } else {
            Some(self.form_rrule.clone())
        };
        event.reminder_minutes = self.form_reminder.parse().ok();
        event.project_id = if self.form_project_index == 0 {
            None
        } else {
            self.projects
                .get(self.form_project_index - 1)
                .map(|p| p.id.clone())
        };

        self.loading = true;
        self.worker.save_event(event, self.form_is_new);
    }

    pub(super) fn handle_input_submit(&mut self) {
        match self.view {
            View::QuickAdd => {
                let input = self.quick_add_input.trim().to_string();
                if !input.is_empty() {
                    self.parse_and_create_event(&input);
                }
                self.view = View::Month;
            }
            View::GoogleAuth => {
                if self.google_auth_field == 0 {
                    self.google_auth_field = 1;
                } else {
                    self.complete_google_auth();
                }
            }
            View::IcalImport => self.ical_import_submit(),
            _ => {}
        }
    }

    pub(super) fn parse_and_create_event(&mut self, input: &str) {
        // Simple natural language parser: "title at HH:MM" or "title on YYYY-MM-DD at HH:MM"
        let title = input.to_string();
        let date = self.focused_date.format("%Y-%m-%d").to_string();
        let start = format!("{} 09:00:00", date);
        let end = format!("{} 10:00:00", date);

        let cal_id = self
            .calendars
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_default();
        if cal_id.is_empty() {
            self.set_status("No calendar available.", true);
            return;
        }

        let event = Event::new(cal_id, title, start, end, "UTC");
        let mut event = event;
        event.timezone = crate::time::local_timezone_name();
        self.loading = true;
        self.worker.save_event(event, true);
    }

    // ── Calendar list actions ─────────────────────────────────────
}
