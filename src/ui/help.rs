/* Help popup overlay — keybinding reference.  */

use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

pub fn render_help(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(70, 85, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let lines = build_help_lines();
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn heading(title: &str) -> Line<'static> {
    let t = theme();
    Line::from(Span::styled(
        title.to_string(),
        t.accent_style()
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

fn binding(key: &str, desc: &str) -> Line<'static> {
    let t = theme();
    Line::from(vec![
        Span::styled(format!("  {:<20}", key), t.header_label()),
        Span::styled(desc.to_string(), t.normal()),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn build_help_lines() -> Vec<Line<'static>> {
    vec![
        blank(),
        heading("GLOBAL"),
        binding("q / Ctrl+C", "Quit"),
        binding("?", "Toggle this help"),
        binding("1", "Month view"),
        binding("2", "Week view"),
        binding("3", "Day view"),
        binding("4", "Agenda view"),
        binding("Tab", "Focus calendar sidebar"),
        binding("G", "Open Google management"),
        binding("S", "Sync with Google Calendar"),
        binding("i", "Open .ics import"),
        binding("x", "Export .ics file"),
        blank(),
        heading("MONTH VIEW"),
        binding("h / l", "Previous / next day"),
        binding("H / L", "Previous / next month"),
        binding("j / k", "Row down / up (7 days)"),
        binding("n", "Jump to today"),
        binding("c", "Create event on selected day"),
        binding("e", "Edit selected event"),
        binding("d", "Delete selected event"),
        binding("Enter", "Switch to day view"),
        binding("/", "Quick-add event"),
        blank(),
        heading("WEEK / DAY VIEW"),
        binding("h / l", "Previous / next week (day)"),
        binding("j / k", "Next / previous event"),
        binding("n", "Jump to current time"),
        binding("c", "Create event"),
        binding("e", "Edit selected event"),
        binding("d", "Delete event"),
        binding("PgUp / PgDn", "Scroll time grid"),
        blank(),
        heading("AGENDA VIEW"),
        binding("j / k", "Scroll"),
        binding("n", "Jump to today"),
        binding("c", "Create event"),
        binding("e", "Edit / Enter detail"),
        binding("d", "Delete event"),
        blank(),
        heading("CALENDAR SIDEBAR (Tab)"),
        binding("j / k", "Navigate calendars"),
        binding("Space", "Toggle calendar visibility"),
        binding("Tab / Esc", "Return to main view"),
        blank(),
        heading("EVENT FORM"),
        binding("Tab / ↓", "Next field"),
        binding("Shift+Tab / ↑", "Previous field"),
        binding("h / l (on selects)", "Previous / next option"),
        binding("Space (on All day)", "Toggle"),
        binding("Enter", "Save event"),
        binding("Esc", "Cancel"),
        blank(),
        heading("QUICK-ADD BAR"),
        binding("Type title", "Event title (uses focused date)"),
        binding("Enter", "Create event"),
        binding("Esc", "Cancel"),
        blank(),
        heading("GOOGLE MANAGEMENT"),
        binding("j / k", "Move through discoverable calendars"),
        binding("r", "Refresh calendar discovery"),
        binding("i / Enter", "Import selected Google calendar"),
        binding("l", "Login / reconnect"),
        binding("o", "Logout"),
        binding("s", "Sync now"),
        binding("Esc", "Close"),
        blank(),
        heading("ICAL IMPORT"),
        binding("Type path", "Path to the .ics file"),
        binding("h / l", "Change target calendar"),
        binding("Enter", "Import"),
        binding("Esc", "Cancel"),
        blank(),
        heading("PLANNER"),
        binding("p", "Open Planner Inbox"),
        binding(
            "Timezone",
            "IANA name: Europe/Rome or UTC (detected local shown)",
        ),
        binding(
            "Availability",
            "day=HH:MM-HH:MM, comma-separated; e.g. mon=09:00-17:00 (full days accepted)",
        ),
        binding("n", "Create planner task"),
        binding("s", "Configure planner settings"),
        binding("o", "Optimize inbox into a proposal"),
        binding("a", "Apply the reviewed proposal"),
        binding("j / k", "Select planner task"),
        blank(),
        heading("HELP"),
        binding("j / k", "Scroll"),
        binding("PgUp / PgDn", "Scroll page"),
        binding("Esc / ?", "Close help"),
        blank(),
    ]
}
