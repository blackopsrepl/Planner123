/* Agenda view — chronological list of upcoming events grouped by day.  */

use chrono::{Local, NaiveDate};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::{format_duration, friendly_date, truncate};

pub fn render_agenda(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let today = Local::now().date_naive();

    let block = Block::default()
        .title(" Agenda ")
        .title_style(t.accent_style().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(t.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner_width = inner.width.saturating_sub(2) as usize;
    let visible_events = app.visible_events();

    // Group events by date, starting from today
    let mut lines: Vec<Line> = Vec::new();

    // Show the next 60 days
    let mut prev_date: Option<NaiveDate> = None;
    let mut event_count = 0;

    for ev in &visible_events {
        let date = if let Some(dt) = ev.start_local_dt() {
            dt.date_naive()
        } else {
            continue;
        };

        // Skip events in the past (before today)
        if date < today {
            continue;
        }

        // Day separator header
        if prev_date != Some(date) {
            // Blank line between days (except first)
            if prev_date.is_some() {
                lines.push(Line::from(""));
            }

            let date_label = friendly_date(date);
            let date_str = format!("  {} — {}  ", date_label, date.format("%a, %b %-d"));

            let date_style = if date == today {
                t.agenda_today()
            } else {
                t.agenda_date_header()
            };

            lines.push(Line::from(Span::styled(date_str, date_style)));
            prev_date = Some(date);
        }

        // Event line: ▌ HH:MM-HH:MM  TITLE  [calendar]  [P]
        let cal_idx = app.calendar_index_for(&ev.calendar_id);
        let rail_style = t.event_rail(cal_idx);

        let time_str = if ev.all_day {
            "all day ".to_string()
        } else {
            let start = ev
                .start_local_dt()
                .map(|d| d.format("%H:%M").to_string())
                .unwrap_or_default();
            let end = ev
                .end_local_dt()
                .map(|d| d.format("%H:%M").to_string())
                .unwrap_or_default();
            format!("{}-{}", start, end)
        };

        let duration_str = ev
            .duration_minutes()
            .map(|m| format!(" ({})", format_duration(m)))
            .unwrap_or_default();

        let max_title_w = inner_width.saturating_sub(time_str.len() + 4);
        let title = truncate(&ev.title, max_title_w);

        let is_selected = app
            .events
            .get(app.selected_event_index)
            .map(|se| se.id == ev.id)
            .unwrap_or(false);

        let title_style = if is_selected {
            t.event_selected()
        } else {
            t.normal()
        };

        let mut spans = vec![
            Span::styled("  \u{258c}", rail_style), // ▌
            Span::styled(format!(" {:9}", time_str), t.dimmed()),
            Span::styled(title, title_style),
            Span::styled(duration_str, t.dimmed()),
        ];

        // Project badge
        if ev.project_id.is_some() {
            if let Some(proj) = ev
                .project_id
                .as_ref()
                .and_then(|pid| app.projects.iter().find(|p| &p.id == pid))
            {
                spans.push(Span::styled(
                    format!(" [{}]", truncate(&proj.name, 8)),
                    t.project_badge(),
                ));
            }
        }

        // Recurring indicator
        if ev.is_recurring() {
            spans.push(Span::styled(" \u{21bb}", t.dimmed())); // ↻
        }

        lines.push(Line::from(spans));
        event_count += 1;
    }

    if event_count == 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", t.normal()),
            Span::styled("No upcoming events.", t.dimmed()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", t.normal()),
            Span::styled("Press ", t.dimmed()),
            Span::styled("c", t.status_key()),
            Span::styled(" to create one.", t.dimmed()),
        ]));
    }

    let para = Paragraph::new(lines).scroll((app.agenda_scroll, 0));
    frame.render_widget(para, inner);
}
