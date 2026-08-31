use super::*;
impl Theme {
    // ── Shared semantic styles (identical to solverforge-mail) ──────────

    pub fn header(&self) -> Style {
        Style::default().fg(self.background).bg(self.accent)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.selection_fg)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_bar(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.color0)
    }

    pub fn status_key(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.color4)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_desc(&self) -> Style {
        Style::default().fg(self.color8)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.color8)
    }

    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn dimmed(&self) -> Style {
        Style::default().fg(self.color8)
    }

    pub fn normal(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    pub fn error(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(224, 108, 117))
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn header_label(&self) -> Style {
        Style::default()
            .fg(self.color4)
            .add_modifier(Modifier::BOLD)
    }

    pub fn popup(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.color0)
    }

    pub fn popup_title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn search_input(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.color0)
    }

    pub fn spinner(&self) -> Style {
        Style::default()
            .fg(self.color6)
            .add_modifier(Modifier::BOLD)
    }

    // ── Calendar-specific styles ─────────────────────────────────────
}
