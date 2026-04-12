/* Sidebar: calendar list with colored bullets + project progress bars.  */

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::{progress_bar, truncate};

pub fn render_calendar_list(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.sidebar_focused;

    // Split sidebar vertically: calendars on top, projects below
    let project_count = app.projects.len();
    let projects_height = if project_count > 0 {
        (project_count + 4).min(area.height as usize / 2) as u16
    } else {
        0
    };
    let calendars_height = area.height.saturating_sub(projects_height);

    let [cals_area, projs_area] = Layout::vertical([
        Constraint::Length(calendars_height),
        Constraint::Length(projects_height),
    ])
    .areas(area);

    render_calendars(app, frame, cals_area, focused);

    if project_count > 0 {
        render_projects(app, frame, projs_area);
    }
}

fn render_calendars(app: &App, frame: &mut Frame, area: Rect, focused: bool) {
    let t = theme();
    let inner_width = area.width.saturating_sub(4) as usize;

    let items: Vec<ListItem> = app
        .calendars
        .iter()
        .enumerate()
        .map(|(i, cal)| {
            let cal_color = t.calendar_color(i);
            let dot = if cal.visible {
                "\u{25cf} "
            } else {
                "\u{25cb} "
            }; // ● or ○
            let name = truncate(&cal.name, inner_width.saturating_sub(4));
            let source_icon = match cal.source {
                crate::models::CalendarSource::Google => {
                    if app
                        .calendar_sync_state
                        .get(&cal.id)
                        .map(|state| !state.writable)
                        .unwrap_or(false)
                    {
                        " G lock"
                    } else {
                        " G"
                    }
                }
                crate::models::CalendarSource::Local => "",
            };

            let line = Line::from(vec![
                Span::styled(
                    dot,
                    ratatui::style::Style::default()
                        .fg(cal_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}{}", name, source_icon),
                    if focused && i == app.calendar_list_index {
                        t.selected()
                    } else {
                        t.normal()
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    if focused {
        state.select(Some(app.calendar_list_index));
    }

    let block = Block::default()
        .title(" Calendars ")
        .title_style(if focused {
            t.accent_style().add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        })
        .borders(Borders::ALL)
        .border_style(if focused {
            t.border_focused()
        } else {
            t.border()
        });

    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_projects(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let inner_width = area.width.saturating_sub(4) as usize;

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("", t.dimmed()))); // padding

    for proj in &app.projects {
        // Count events in this project
        let proj_events: Vec<&crate::models::Event> = app
            .visible_events()
            .into_iter()
            .filter(|e| e.project_id.as_deref() == Some(&proj.id))
            .collect();

        let total = proj_events.len();
        let completed = app
            .completed_event_ids
            .iter()
            .filter(|id| proj_events.iter().any(|e| &e.id == *id))
            .count();

        let fraction = if total > 0 {
            completed as f64 / total as f64
        } else {
            0.0
        };
        let bar_width = inner_width.saturating_sub(2).min(8);
        let (filled, empty) = progress_bar(fraction, bar_width);

        // Project name
        let name = truncate(&proj.name, inner_width.saturating_sub(2));
        lines.push(Line::from(vec![
            Span::styled("\u{25b8} ", t.accent_style()), // ▸
            Span::styled(name, t.normal()),
        ]));

        // Progress bar + count
        lines.push(Line::from(vec![
            Span::styled("  ", t.normal()),
            Span::styled(filled, t.progress_filled()),
            Span::styled(empty, t.progress_empty()),
            Span::styled(format!(" {}/{}", completed, total), t.dimmed()),
        ]));
    }

    let block = Block::default()
        .title(" Projects ")
        .title_style(t.dimmed())
        .borders(Borders::ALL)
        .border_style(t.border());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
