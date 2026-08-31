use super::*;
impl Theme {
    // Color for a calendar by index (0-based). Cycles through the palette.
    // Returns the Color (not a Style) so callers can build fg/bg variants.
    pub fn calendar_color(&self, index: usize) -> Color {
        match index % CALENDAR_COLORS {
            0 => self.color1, // bright green
            1 => self.color3, // cyan-green
            2 => self.color6, // bright cyan
            3 => self.color7, // light cyan
            4 => self.color4, // muted blue
            5 => self.color5, // light blue
            6 => self.color9, // bright green variant
            _ => self.accent, // mint green
        }
    }

    // The pulsing "Now" beam line — accent color, bold.
    pub fn now_beam(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    // Dimmed style for past time slots in week/day view.
    pub fn past_dim(&self) -> Style {
        Style::default().fg(self.color8)
    }

    // The colored left-rail of an event ribbon (▌ character).
    pub fn event_rail(&self, cal_index: usize) -> Style {
        Style::default()
            .fg(self.calendar_color(cal_index))
            .add_modifier(Modifier::BOLD)
    }

    // Event title text in time-grid views.
    pub fn event_title(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    // Event title when the event is selected.
    pub fn event_selected(&self) -> Style {
        Style::default()
            .fg(self.selection_fg)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    // Today's date number highlight in the month grid.
    pub fn today_cell(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    // Weekend day numbers (slightly dimmed).
    pub fn weekend(&self) -> Style {
        Style::default().fg(self.color8)
    }

    // Day from adjacent month (very dimmed).
    pub fn adjacent_month(&self) -> Style {
        Style::default().fg(Color::Rgb(60, 63, 90))
    }

    // Project badge text: `[P]` inline in status bar.
    pub fn project_badge(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.color3)
            .add_modifier(Modifier::BOLD)
    }

    // Progress bar filled segment.
    pub fn progress_filled(&self) -> Style {
        Style::default().fg(self.accent)
    }

    // Progress bar empty segment.
    pub fn progress_empty(&self) -> Style {
        Style::default().fg(self.color8)
    }

    // Quick-add bar label badge.
    pub fn quick_add_label(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.color3)
            .add_modifier(Modifier::BOLD)
    }

    // Form field label.
    pub fn form_label(&self) -> Style {
        Style::default()
            .fg(self.color4)
            .add_modifier(Modifier::BOLD)
    }

    // Form field value (editable).
    pub fn form_value(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    // Form field when focused (accent border).
    pub fn form_focused(&self) -> Style {
        Style::default()
            .fg(self.foreground)
            .bg(Color::Rgb(20, 22, 40))
    }

    // Agenda date section header.
    pub fn agenda_date_header(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    // "TODAY" label in agenda.
    pub fn agenda_today(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }
}
