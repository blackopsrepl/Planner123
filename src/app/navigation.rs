use super::*;
impl App {
    pub(super) fn switch_view(&mut self, view: View) {
        self.view = view;
        self.sidebar_focused = false;
    }

    pub(super) fn prev_period(&mut self) {
        match self.view {
            View::Month => self.shift_month(-1),
            View::Week => self.focused_date -= Duration::weeks(1),
            View::Day => self.focused_date -= Duration::days(1),
            _ => {}
        }
        self.reload_events_if_needed();
    }

    pub(super) fn next_period(&mut self) {
        match self.view {
            View::Month => self.shift_month(1),
            View::Week => self.focused_date += Duration::weeks(1),
            View::Day => self.focused_date += Duration::days(1),
            _ => {}
        }
        self.reload_events_if_needed();
    }

    pub(super) fn prev_unit(&mut self) {
        match self.view {
            View::Month => self.focused_date -= Duration::weeks(1),
            View::Week | View::Day => {
                if self.selected_event_index > 0 {
                    self.selected_event_index -= 1;
                } else if self.week_scroll > 0 {
                    self.week_scroll -= 1;
                }
            }
            View::Agenda => {
                self.agenda_scroll = self.agenda_scroll.saturating_sub(1);
            }
            View::PlannerInbox => {
                self.planner_selected_index = self.planner_selected_index.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub(super) fn next_unit(&mut self) {
        match self.view {
            View::Month => self.focused_date += Duration::weeks(1),
            View::Week | View::Day => {
                let count = self.visible_events().len();
                if self.selected_event_index + 1 < count {
                    self.selected_event_index += 1;
                } else if self.week_scroll < 18 {
                    self.week_scroll += 1;
                }
            }
            View::Agenda => {
                self.agenda_scroll = self.agenda_scroll.saturating_add(1);
            }
            View::PlannerInbox if self.planner_selected_index + 1 < self.planner_tasks.len() => {
                self.planner_selected_index += 1;
            }
            _ => {}
        }
    }

    pub(super) fn move_day(&mut self, delta: i64) {
        self.focused_date += Duration::days(delta);
        self.reload_events_if_needed();
    }

    pub(super) fn jump_today(&mut self) {
        let today = Local::now().date_naive();
        self.focused_date = today;
        self.view_month = today.month();
        self.view_year = today.year();
        self.reload_events_if_needed();
    }

    pub(super) fn shift_month(&mut self, delta: i32) {
        let mut month = self.view_month as i32 + delta;
        let mut year = self.view_year;
        while month < 1 {
            month += 12;
            year -= 1;
        }
        while month > 12 {
            month -= 12;
            year += 1;
        }
        self.view_month = month as u32;
        self.view_year = year;

        // Keep focused_date in sync
        let day = self
            .focused_date
            .day()
            .min(days_in_month(year, month as u32));
        self.focused_date =
            NaiveDate::from_ymd_opt(year, month as u32, day).unwrap_or(self.focused_date);
    }

    pub(super) fn reload_events_if_needed(&mut self) {
        // Check if focused_date is outside the currently loaded window
        if self.focused_date.month() != self.view_month
            || self.focused_date.year() != self.view_year
        {
            self.view_month = self.focused_date.month();
            self.view_year = self.focused_date.year();
            self.loading = true;
            self.worker.load_events(self.view_year, self.view_month);
        }
    }

    pub(super) fn handle_escape(&mut self) {
        match self.view {
            View::Help
            | View::EventForm
            | View::IcalImport
            | View::QuickAdd
            | View::GoogleAuth
            | View::GoogleManage => {
                self.view = View::Month;
            }
            View::PlannerTaskForm | View::PlannerSettingsForm => self.view = View::PlannerInbox,
            View::PlannerInbox => self.view = View::Month,
            View::CalendarList => {
                self.sidebar_focused = false;
                self.view = View::Month;
            }
            _ => {}
        }
    }

    // ── Event form ────────────────────────────────────────────────
}
