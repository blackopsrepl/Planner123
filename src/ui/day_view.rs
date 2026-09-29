/* Day view — single-day vertical time grid (same engine as week_view, 1 column). Shows more detail per event: description, location, duration.  */
/* Day view — single-day vertical time grid (same engine as week_view, 1 column). Shows more detail per event: description, location, duration.  */

use chrono::Local;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::week_view::render_time_grid;

pub fn render_day(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let today = Local::now().date_naive();
    let date = app.focused_date;

    let is_today = date == today;
    let date_str = if is_today {
        format!(" Today — {} ", date.format("%A, %B %-d, %Y"))
    } else {
        format!(" {} ", date.format("%A, %B %-d, %Y"))
    };

    let block = Block::default()
        .title(date_str)
        .title_style(t.accent_style().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(if is_today {
            t.border_focused()
        } else {
            t.border()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show project context if selected event has a project
    let project_context_height = if let Some(ev) = app.selected_event() {
        if ev.project_id.is_some() {
            1u16
        } else {
            0
        }
    } else {
        0
    };

    let [grid_area, context_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(project_context_height),
    ])
    .areas(inner);

    render_time_grid(app, frame, grid_area, app.focused_date, 1);

    // Inline project context bar at the bottom of the day view
    if project_context_height > 0 {
        if let Some(ev) = app.selected_event() {
            if let Some(proj_id) = &ev.project_id {
                render_project_context(app, frame, context_area, ev, proj_id);
            }
        }
    }
}

fn render_project_context(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    event: &crate::models::Event,
    project_id: &str,
) {
    let t = theme();

    let project = app.projects.iter().find(|p| p.id == project_id);
    let Some(project) = project else {
        return;
    };

    /* Events of this project in the loaded window, and the DAG's next
    actionable one. The TUI has no completion state to feed the DAG, so it
    passes an empty set rather than a fraction it cannot keep. */
    let proj_events: Vec<&crate::models::Event> = app
        .visible_events()
        .into_iter()
        .filter(|e| e.project_id.as_deref() == Some(project_id))
        .collect();

    let total = proj_events.len();

    // Next actionable event in the project DAG
    let all_ids: Vec<&str> = proj_events.iter().map(|e| e.id.as_str()).collect();
    let actionable = app
        .dag
        .next_actionable(all_ids.into_iter(), &std::collections::HashSet::new());
    let next_title = actionable
        .first()
        .and_then(|id| proj_events.iter().find(|e| &e.id == id))
        .map(|e| e.title.as_str())
        .unwrap_or("—");

    // Direct blockers of the current event
    let blockers = app.dag.direct_blockers(&event.id);
    let blocking_titles: Vec<&str> = blockers
        .iter()
        .filter_map(|bid| app.events.iter().find(|e| &e.id == bid))
        .map(|e| e.title.as_str())
        .collect();

    let spans: Vec<Span> = vec![
        Span::styled(" [P] ", t.project_badge()),
        Span::styled(
            format!(" {} ", project.name),
            t.accent_style().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {} events ", total), t.dimmed()),
        Span::styled(" | Next: ", t.status_desc()),
        Span::styled(next_title, t.normal()),
        if !blocking_titles.is_empty() {
            Span::styled(
                format!(" | Blocked by: {}", blocking_titles.join(", ")),
                t.dimmed(),
            )
        } else {
            Span::styled(" | Ready ", t.accent_style())
        },
    ];

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
