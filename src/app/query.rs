use super::*;
impl App {
    pub fn selected_event(&self) -> Option<&Event> {
        self.visible_events()
            .get(self.selected_event_index)
            .copied()
    }

    /// Events visible in the current view window, filtered by calendar visibility.
    pub fn visible_events(&self) -> Vec<&Event> {
        let visible_cal_ids: std::collections::HashSet<&str> = self
            .calendars
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id.as_str())
            .collect();

        self.events
            .iter()
            .filter(|e| visible_cal_ids.contains(e.calendar_id.as_str()) && e.deleted_at.is_none())
            .collect()
    }

    /// Events for a specific date.
    pub fn events_on_date(&self, date: NaiveDate) -> Vec<&Event> {
        self.visible_events()
            .into_iter()
            .filter(|e| e.occurs_on(date))
            .collect()
    }

    // ── Calendar color lookup ──────────────────────────────────────

    /// Find the 0-based index of a calendar in the calendars list (for color lookup).
    pub fn calendar_index_for(&self, calendar_id: &str) -> usize {
        self.calendars
            .iter()
            .position(|c| c.id == calendar_id)
            .unwrap_or(0)
    }

    // ── Status ────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_message = msg.into();
        self.status_is_error = is_error;
    }

    /// True if the cursor should be visible (500ms on, 500ms off at 250ms tick rate).
    pub fn cursor_visible(&self) -> bool {
        self.tick_count % 4 < 2
    }

    // ── Scroll helpers ────────────────────────────────────────────

    pub(super) fn scroll_up(&mut self) {
        match self.view {
            View::Help => self.help_scroll = self.help_scroll.saturating_sub(1),
            View::Agenda => self.agenda_scroll = self.agenda_scroll.saturating_sub(1),
            View::Week | View::Day if self.week_scroll > 0 => self.week_scroll -= 1,
            _ => {}
        }
    }

    pub(super) fn scroll_down(&mut self) {
        match self.view {
            View::Help => self.help_scroll = self.help_scroll.saturating_add(1),
            View::Agenda => self.agenda_scroll = self.agenda_scroll.saturating_add(1),
            View::Week | View::Day if self.week_scroll < 20 => self.week_scroll += 1,
            _ => {}
        }
    }

    pub(super) fn scroll_page(&mut self, delta: i16) {
        match self.view {
            View::Help => {
                if delta > 0 {
                    self.help_scroll = self.help_scroll.saturating_add(delta as u16);
                } else {
                    self.help_scroll = self.help_scroll.saturating_sub((-delta) as u16);
                }
            }
            View::Agenda => {
                if delta > 0 {
                    self.agenda_scroll = self.agenda_scroll.saturating_add(delta as u16);
                } else {
                    self.agenda_scroll = self.agenda_scroll.saturating_sub((-delta) as u16);
                }
            }
            _ => {}
        }
    }
}
