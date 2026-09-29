use super::*;
impl App {
    /* True for the four calendar surfaces an overlay can be opened from. */
    fn is_calendar_view(view: &View) -> bool {
        matches!(view, View::Month | View::Week | View::Day | View::Agenda)
    }

    pub(super) fn switch_view(&mut self, view: View) {
        if Self::is_calendar_view(&view) {
            self.return_view = view.clone();
        }
        self.view = view;
        self.sidebar_focused = false;
        self.reload_events_if_needed();
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
        self.reload_events_if_needed();
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
        self.reload_events_if_needed();
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

    /* The six-week grid rendered for a month view: Monday on or before the
    first of `month`, through the following 41 days. */
    fn month_grid(year: i32, month: u32) -> (NaiveDate, NaiveDate) {
        let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_default();
        let first_weekday =
            chrono::Datelike::weekday(&first_of_month).num_days_from_monday() as i64;
        let grid_start = first_of_month - Duration::days(first_weekday);
        (grid_start, grid_start + Duration::days(41))
    }

    /* Keep the month display anchored to the focused date.

    Month view draws a fixed six-week grid. Pressing `j`/`k` (unit navigation)
    moves `focused_date` by a week without touching `view_month`, and repeated
    presses can carry the cursor beyond the displayed grid. In that case the
    grid is re-anchored to the focused date so the cursor stays visible. */
    fn sync_month_view_to_focus(&mut self) {
        if self.view != View::Month {
            return;
        }
        let (grid_start, grid_end) = Self::month_grid(self.view_year, self.view_month);
        if !(grid_start..=grid_end).contains(&self.focused_date) {
            self.view_month = self.focused_date.month();
            self.view_year = self.focused_date.year();
        }
    }

    /* The date window the current view needs to render every visible event.

    Month view draws its six-week grid, week view spans Monday–Sunday around the
    focused date, and day view is a single date. Agenda follows the same window
    as the month grid so a week switch never exposes an unloaded range. */
    fn required_event_window(&self) -> (NaiveDate, NaiveDate) {
        match self.view {
            View::Week => {
                let days_from_mon =
                    chrono::Datelike::weekday(&self.focused_date).num_days_from_monday() as i64;
                let start = self.focused_date - Duration::days(days_from_mon);
                (start, start + Duration::days(6))
            }
            View::Day => (self.focused_date, self.focused_date),
            _ => Self::month_grid(self.view_year, self.view_month),
        }
    }

    /* Reload events when the window the current view needs is not fully covered
    by the window the last load populated.

    Comparing the display month against `focused_date` is insufficient: month
    navigation moves both in lockstep, so the check never fires and the grid
    renders a stale (often empty) event set. The authoritative signal is the
    window that was actually loaded. */
    pub(super) fn reload_events_if_needed(&mut self) {
        self.sync_month_view_to_focus();

        let (needed_start, needed_end) = self.required_event_window();
        let covered = match self.event_window {
            Some((loaded_start, loaded_end)) => {
                loaded_start <= needed_start && needed_end <= loaded_end
            }
            None => false,
        };

        if !covered {
            self.reload_events();
        }
    }

    /* Force a reload of the window the current view needs.

    Used after mutations (save, delete, sync, import) that can change the stored
    events even though the visible range did not move. */
    pub(super) fn reload_events(&mut self) {
        self.sync_month_view_to_focus();
        let (start, end) = self.required_event_window();
        self.event_window = Some((start, end));
        self.loading = true;
        self.event_load_seq = self.worker.load_events(start, end);
    }

    pub(super) fn handle_escape(&mut self) {
        match self.view {
            // Closing an overlay returns to the calendar view it was opened
            // from. It has to go through `switch_view`: the overlay may have
            // been opened from a view whose window is narrower than a month
            // (day/week), and returning must re-anchor and load that view's
            // range instead of rendering a stale one.
            View::Help
            | View::EventForm
            | View::IcalImport
            | View::QuickAdd
            | View::GoogleAuth
            | View::GoogleManage
            | View::CalendarList
            | View::PlannerInbox => {
                let return_view = self.return_view.clone();
                self.switch_view(return_view);
            }
            View::PlannerTaskForm | View::PlannerSettingsForm => self.view = View::PlannerInbox,
            _ => {}
        }
    }

    // ── Event form ────────────────────────────────────────────────
}
