use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

pub fn render_ical_import(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(72, 45, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Import .ics ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cursor = if app.cursor_visible() { "\u{2588}" } else { "" };

    let path_focused = app.ical_import_field_index == 0;
    let calendar_focused = app.ical_import_field_index == 1;

    let path_style = if path_focused {
        t.form_focused()
    } else {
        t.form_value()
    };
    let calendar_style = if calendar_focused {
        t.form_focused()
    } else {
        t.form_value()
    };
    let label_style = |focused: bool| {
        if focused {
            t.form_label().add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        }
    };

    let calendar_value = app
        .calendars
        .get(app.ical_import_calendar_index)
        .map(|calendar| format!("{} (h/l to change)", calendar.name))
        .unwrap_or_else(|| "no calendars".to_string());

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Import VEVENT entries from an .ics file into the selected calendar.",
            t.normal(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if path_focused { "▶ " } else { "  " }, t.accent_style()),
            Span::styled("Path          ", label_style(path_focused)),
            Span::styled(
                format!(
                    "{}{}",
                    app.ical_import_path,
                    if path_focused { cursor } else { "" }
                ),
                path_style,
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(if calendar_focused { "▶ " } else { "  " }, t.accent_style()),
            Span::styled("Calendar      ", label_style(calendar_focused)),
            Span::styled(calendar_value, calendar_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Floating timestamps default to ", t.dimmed()),
            Span::styled(crate::time::local_timezone_name(), t.accent_style()),
            Span::styled(".", t.dimmed()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", t.normal()),
            Span::styled(" Tab ", t.status_key()),
            Span::styled(" next  ", t.status_desc()),
            Span::styled(" Enter ", t.status_key()),
            Span::styled(" import  ", t.status_desc()),
            Span::styled(" Esc ", t.status_key()),
            Span::styled(" cancel", t.status_desc()),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}
