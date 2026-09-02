use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::{app::App, theme, ui::util::centered_rect};

pub fn render_inbox(app: &App, frame: &mut Frame, area: Rect) {
    let [tasks_area, proposal_area] =
        Layout::vertical([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(area);
    let [task_list_area, task_detail_area] =
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(tasks_area);
    let tasks: Vec<ListItem> = if app.planner_tasks.is_empty() {
        vec![ListItem::new("No inbox tasks. Press n to add one.")]
    } else {
        app.planner_tasks
            .iter()
            .enumerate()
            .map(|(index, task)| {
                let marker = if index == app.planner_selected_index {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(
                        &task.title,
                        if index == app.planner_selected_index {
                            Style::default().add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::raw(format!(
                        "  {}m  {:?}  {:?}",
                        task.duration_minutes, task.priority, task.cognitive_load
                    )),
                ]))
            })
            .collect()
    };
    frame.render_widget(
        List::new(tasks).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Planner Inbox "),
        ),
        task_list_area,
    );
    let task_detail = match app.planner_tasks.get(app.planner_selected_index) {
        Some(task) => {
            let calendar = app
                .calendars
                .iter()
                .find(|calendar| calendar.id == task.target_calendar_id)
                .map(|calendar| calendar.name.as_str())
                .unwrap_or("unknown calendar");
            vec![
                Line::from(format!(
                    "{}  ({} minutes)",
                    task.title, task.duration_minutes
                )),
                Line::from(format!("Calendar: {calendar}")),
                Line::from(format!(
                    "Priority: {:?}    Cognitive load: {:?}",
                    task.priority, task.cognitive_load
                )),
                Line::from(format!(
                    "Earliest: {}    Deadline: {:?} {}",
                    task.earliest_at.as_deref().unwrap_or("none"),
                    task.deadline_kind,
                    task.deadline_at.as_deref().unwrap_or("")
                )),
                Line::from(format!("Task ID: {}", task.id)),
            ]
        }
        None => vec![Line::from(
            "Select an inbox task with j/k to inspect its scheduling details.",
        )],
    };
    frame.render_widget(
        Paragraph::new(task_detail).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Selected Task "),
        ),
        task_detail_area,
    );
    let proposal = match &app.planner_proposal {
        Some(detail) => {
            let scheduled = detail.items.iter().filter(|item| item.scheduled).count();
            let lines = detail
                .items
                .iter()
                .take(5)
                .map(|item| {
                    let when = item.start_at.as_deref().unwrap_or("unscheduled");
                    Line::from(format!(
                        "{}  {}",
                        when,
                        item.explanation.as_deref().unwrap_or("scheduled")
                    ))
                })
                .collect::<Vec<_>>();
            let mut text = vec![Line::from(format!(
                "Proposal {}: {scheduled}/{} scheduled",
                &detail.proposal.id[..8],
                detail.items.len()
            ))];
            text.push(Line::from(if detail.proposal.status == "ready" {
                "Press a to apply this reviewed proposal."
            } else {
                "This proposal has already been applied."
            }));
            text.extend(lines);
            text
        }
        None => vec![Line::from(
            "No proposal yet. Press s to configure Planner Settings, then press o.",
        )],
    };
    frame.render_widget(
        Paragraph::new(proposal).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" SolverForge Proposal "),
        ),
        proposal_area,
    );
}

pub fn render_settings_form(app: &App, frame: &mut Frame) {
    let area = centered_rect(90, 82, frame.area());
    frame.render_widget(Clear, area);
    let rows = [
        format!("Timezone (IANA name): {}", app.planner_settings_timezone),
        format!(
            "Weekly availability (weekly working windows): {}",
            app.planner_settings_availability
        ),
        format!("Horizon days: {}", app.planner_settings_horizon_days),
        format!("Slot minutes: {}", app.planner_settings_slot_minutes),
        format!("Solve seconds: {}", app.planner_settings_solve_seconds),
    ];
    let mut lines = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let prefix = if index == app.planner_settings_field {
                "> "
            } else {
                "  "
            };
            Line::from(Span::styled(
                format!("{prefix}{row}"),
                if index == app.planner_settings_field {
                    theme::theme().selected()
                } else {
                    Style::default()
                },
            ))
        })
        .collect::<Vec<_>>();
    let local_timezone = crate::time::local_timezone_name();
    lines.extend([
        Line::from(""),
        Line::from("Required: an IANA timezone name, for example Europe/Rome or UTC."),
        Line::from(format!("Detected system timezone: {local_timezone}")),
        Line::from("Availability syntax: day=HH:MM-HH:MM; separate windows with commas."),
        Line::from(
            "Use mon through sun (or full names such as Monday). Example: mon=09:00-17:00, tue=09:00-17:00.",
        ),
        Line::from(
            "Use a day more than once for split shifts; each window must end after it starts.",
        ),
        Line::from("Enter validates and saves all settings. Esc cancels."),
    ]);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Planner Settings "),
        ),
        area,
    );
}

pub fn render_task_form(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 46, frame.area());
    frame.render_widget(Clear, area);
    let calendar = app
        .calendars
        .get(app.planner_task_calendar_index)
        .map(|calendar| calendar.name.as_str())
        .unwrap_or("none");
    let priorities = ["low", "normal", "high"];
    let loads = ["low", "medium", "high"];
    let rows = [
        format!("Title: {}", app.planner_task_title),
        format!("Duration minutes: {}", app.planner_task_duration),
        format!("Target calendar: {calendar}  (type any key to cycle)"),
        format!(
            "Priority: {}  (type any key to cycle)",
            priorities[app.planner_task_priority_index]
        ),
        format!(
            "Cognitive load: {}  (type any key to cycle)",
            loads[app.planner_task_cognitive_index]
        ),
    ];
    let lines = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let prefix = if index == app.planner_task_field {
                "> "
            } else {
                "  "
            };
            Line::from(Span::styled(
                format!("{prefix}{row}"),
                if index == app.planner_task_field {
                    theme::theme().selected()
                } else {
                    Style::default()
                },
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" New Planner Task — Tab fields, Enter save "),
        ),
        area,
    );
}
