use super::*;
impl App {
    pub(super) fn open_planner_task_form(&mut self) {
        self.planner_task_field = 0;
        self.planner_task_title.clear();
        self.planner_task_duration = "60".to_string();
        self.planner_task_calendar_index = 0;
        self.planner_task_priority_index = 1;
        self.planner_task_cognitive_index = 1;
        self.view = View::PlannerTaskForm;
    }

    pub(super) fn planner_task_next_field(&mut self) {
        self.planner_task_field = (self.planner_task_field + 1) % 5;
    }

    pub(super) fn planner_task_prev_field(&mut self) {
        self.planner_task_field = self.planner_task_field.checked_sub(1).unwrap_or(4);
    }

    pub(super) fn planner_task_input_char(&mut self, c: char) {
        match self.planner_task_field {
            0 => self.planner_task_title.push(c),
            1 if c.is_ascii_digit() => self.planner_task_duration.push(c),
            2 if !self.calendars.is_empty() => {
                self.planner_task_calendar_index =
                    (self.planner_task_calendar_index + 1) % self.calendars.len();
            }
            3 => self.planner_task_priority_index = (self.planner_task_priority_index + 1) % 3,
            4 => self.planner_task_cognitive_index = (self.planner_task_cognitive_index + 1) % 3,
            _ => {}
        }
    }

    pub(super) fn planner_task_input_backspace(&mut self) {
        match self.planner_task_field {
            0 => {
                self.planner_task_title.pop();
            }
            1 => {
                self.planner_task_duration.pop();
            }
            _ => {}
        }
    }

    pub(super) fn planner_task_submit(&mut self) {
        let Some(calendar) = self.calendars.get(self.planner_task_calendar_index) else {
            self.set_status("Create a calendar before adding a planner task.", true);
            return;
        };
        let duration_minutes = match self.planner_task_duration.parse() {
            Ok(value) if value > 0 => value,
            _ => {
                self.set_status("Task duration must be a positive number of minutes.", true);
                return;
            }
        };
        let priority = [TaskPriority::Low, TaskPriority::Normal, TaskPriority::High]
            [self.planner_task_priority_index]
            .clone();
        let cognitive_load = [
            CognitiveLoad::Low,
            CognitiveLoad::Medium,
            CognitiveLoad::High,
        ][self.planner_task_cognitive_index]
            .clone();
        self.loading = true;
        self.worker
            .create_planner_task(crate::planner::CreateTaskInput {
                title: self.planner_task_title.clone(),
                duration_minutes,
                target_calendar_id: calendar.id.clone(),
                project_id: None,
                priority,
                cognitive_load,
                earliest_at: None,
                deadline_kind: crate::models::DeadlineKind::None,
                deadline_at: None,
            });
        self.view = View::PlannerInbox;
    }

    pub(super) fn open_planner_settings_form(&mut self) {
        self.planner_settings_field = 0;
        self.worker.load_planner_settings();
        self.view = View::PlannerSettingsForm;
    }

    pub(super) fn planner_settings_next_field(&mut self) {
        self.planner_settings_field = (self.planner_settings_field + 1) % 5;
    }

    pub(super) fn planner_settings_prev_field(&mut self) {
        self.planner_settings_field = self.planner_settings_field.checked_sub(1).unwrap_or(4);
    }

    pub(super) fn planner_settings_input_char(&mut self, character: char) {
        match self.planner_settings_field {
            0 => self.planner_settings_timezone.push(character),
            1 => self.planner_settings_availability.push(character),
            2 => self.planner_settings_horizon_days.push(character),
            3 => self.planner_settings_slot_minutes.push(character),
            4 => self.planner_settings_solve_seconds.push(character),
            _ => {}
        }
    }

    pub(super) fn planner_settings_input_backspace(&mut self) {
        match self.planner_settings_field {
            0 => self.planner_settings_timezone.pop(),
            1 => self.planner_settings_availability.pop(),
            2 => self.planner_settings_horizon_days.pop(),
            3 => self.planner_settings_slot_minutes.pop(),
            4 => self.planner_settings_solve_seconds.pop(),
            _ => None,
        };
    }

    pub(super) fn planner_settings_submit(&mut self) {
        let timezone = match crate::time::normalize_timezone(self.planner_settings_timezone.trim())
        {
            Ok(timezone) => timezone,
            Err(_) => {
                self.set_status(
                    "Timezone must be an IANA name such as Europe/Rome or UTC.",
                    true,
                );
                return;
            }
        };
        let availability = match availability_from_specs(&self.planner_settings_availability) {
            Ok(value) => value,
            Err(message) => {
                self.set_status(message, true);
                return;
            }
        };
        let parse_number = |value: &str, label: &str| {
            value
                .parse::<i64>()
                .map_err(|_| format!("{label} must be a whole number."))
        };
        let horizon_days = match parse_number(&self.planner_settings_horizon_days, "Horizon days") {
            Ok(value) => value,
            Err(message) => {
                self.set_status(message, true);
                return;
            }
        };
        let slot_minutes = match parse_number(&self.planner_settings_slot_minutes, "Slot minutes") {
            Ok(value) => value,
            Err(message) => {
                self.set_status(message, true);
                return;
            }
        };
        let solve_seconds =
            match parse_number(&self.planner_settings_solve_seconds, "Solve seconds") {
                Ok(value) => value,
                Err(message) => {
                    self.set_status(message, true);
                    return;
                }
            };
        self.loading = true;
        self.worker
            .update_planner_settings(crate::planner::SettingsUpdate {
                timezone: Some(timezone),
                availability: Some(availability),
                horizon_days: Some(horizon_days),
                slot_minutes: Some(slot_minutes),
                solve_seconds: Some(solve_seconds),
                ..Default::default()
            });
        self.view = View::PlannerInbox;
    }

    pub(super) fn planner_optimization_prerequisite(&self) -> Option<String> {
        let Some(settings) = self.planner_settings.as_ref() else {
            return Some("Planner settings are loading; wait a moment, then optimize.".into());
        };
        let Some(timezone) = settings.timezone.as_deref() else {
            return Some(
                "Set a planner timezone first: use an IANA name such as Europe/Rome or UTC.".into(),
            );
        };
        if crate::time::normalize_timezone(timezone).is_err() {
            return Some(
                "Correct the planner timezone: use an IANA name such as Europe/Rome or UTC.".into(),
            );
        }
        let availability =
            match serde_json::from_str::<crate::planner::Availability>(&settings.availability_json)
            {
                Ok(value) => value,
                Err(_) => return Some("Correct the weekly availability before optimizing.".into()),
            };
        if let Err(error) = crate::planner::validate_availability(&availability) {
            return Some(format!("Correct weekly availability: {error}"));
        }
        None
    }
}
