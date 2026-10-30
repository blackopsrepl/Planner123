use super::time_grid::layout::layout_day;
use crate::{app::App, effects::Effects, models::Event, theme::theme};
use chrono::{Duration, Timelike};
use ratatui::{
    layout::Rect,
    style::Modifier,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// A real, read-only time grid: proposed items and existing occurrences share
/// the same overlap-lane allocator as the working calendar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Proposal preview · dashed = not applied · A applies ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(detail) = &app.planner_proposal else {
        frame.render_widget(
            Paragraph::new("No proposal yet. Press s for settings, then o to optimize."),
            inner,
        );
        return;
    };
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let ready = detail.proposal.status == "ready" && detail.applicability.can_apply;
    let selected = app
        .planner_tasks
        .get(app.planner_selected_index)
        .and_then(|t| detail.items.iter().find(|i| i.task_id == t.id))
        .or_else(|| detail.items.get(app.planner_selected_index))
        .or_else(|| detail.items.first());
    let applied = detail.proposal.status == "applied" && !app.effects.applied_ids.is_empty();
    let state = if applied {
        "APPLIED"
    } else if ready {
        "GHOST / ready"
    } else {
        "not applicable / awaiting reload"
    };
    let scheduled = detail.items.iter().filter(|i| i.scheduled).count();
    frame.render_widget(
        Paragraph::new(format!(
            "{state} · {scheduled}/{} scheduled · {}",
            detail.items.len(),
            detail.proposal.timezone
        ))
        .style(t.accent_style()),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    if let Some(item) = selected.filter(|_| inner.height > 1) {
        let text = format!(
            "{} · cognitive {} / fatigue {} · {}",
            item.start_at.as_deref().unwrap_or("Unscheduled"),
            item.cognitive_penalty,
            item.fatigue_penalty,
            item.explanation.as_deref().unwrap_or("scheduled")
        );
        frame.render_widget(
            Paragraph::new(text).style(t.dimmed()),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
    }
    let mut ghosts = Vec::new();
    for item in detail.items.iter().filter(|i| i.scheduled) {
        let (Some(start), Some(end)) = (&item.start_at, &item.end_at) else {
            continue;
        };
        let task = app
            .planner_tasks
            .iter()
            .find(|task| task.id == item.task_id);
        let cached = app.effects.task_labels.get(&item.task_id);
        let title = task
            .map(|t| t.title.as_str())
            .or_else(|| cached.map(|t| t.0.as_str()))
            .unwrap_or("Scheduled task");
        let calendar = task
            .map(|t| t.target_calendar_id.as_str())
            .or_else(|| cached.map(|t| t.1.as_str()))
            .unwrap_or("");
        let mut event = Event::new(calendar, title, start, end, &detail.proposal.timezone);
        event.id = app
            .effects
            .task_events
            .get(&item.task_id)
            .cloned()
            .unwrap_or_else(|| item.id.clone());
        ghosts.push(event);
    }
    let anchor = selected
        .and_then(|i| {
            ghosts
                .iter()
                .find(|e| e.id == *app.effects.task_events.get(&i.task_id).unwrap_or(&i.id))
        })
        .and_then(|e| e.start_local_dt());
    let Some(first) = anchor.or_else(|| ghosts.iter().filter_map(|e| e.start_local_dt()).min())
    else {
        return;
    };
    let start = first.date_naive();
    let first_row = (first.hour() * 2 + first.minute() / 30).saturating_sub(4);
    let grid = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(3),
    );
    if grid.width < 9 || grid.height < 2 {
        return;
    }
    let days = (grid.width.saturating_sub(6) / 14).clamp(1, 7);
    let col_w = grid.width.saturating_sub(6) / days;
    let timezone = crate::time::parse_timezone(&detail.proposal.timezone).ok();
    let busy: Vec<Event> = app
        .occurrences
        .iter()
        .filter(|e| !app.effects.applied_ids.contains(&e.id))
        .map(|event| {
            let mut copy = event.clone();
            if let (Some(zone), Some(start), Some(end)) =
                (timezone, event.start_dt(), event.end_dt())
            {
                copy.start_at = start
                    .with_timezone(&zone)
                    .format(crate::time::STORAGE_FORMAT)
                    .to_string();
                copy.end_at = end
                    .with_timezone(&zone)
                    .format(crate::time::STORAGE_FORMAT)
                    .to_string();
                copy.timezone = detail.proposal.timezone.clone();
            }
            copy
        })
        .collect();
    let mut refs: Vec<&Event> = busy.iter().collect();
    refs.extend(ghosts.iter());
    for d in 0..days {
        let date = start + Duration::days(d as i64);
        let col_x = grid.x + 6 + d * col_w;
        frame.render_widget(
            Paragraph::new(date.format("%a %d %b").to_string()).style(t.dimmed()),
            Rect::new(col_x, grid.y, col_w, 1),
        );
        for placement in layout_day(&refs, date) {
            if placement.top < first_row
                || placement.top >= first_row + grid.height.saturating_sub(1) as u32
            {
                continue;
            }
            let lane_w = col_w.saturating_sub(1) / placement.lanes.max(1);
            if lane_w < 2 {
                if placement.lane == 0 {
                    frame.render_widget(
                        Paragraph::new(format!("+{} overlap", placement.lanes))
                            .style(t.accent_style()),
                        Rect::new(
                            col_x,
                            grid.y + 1 + (placement.top - first_row) as u16,
                            col_w.saturating_sub(1),
                            1,
                        ),
                    );
                }
                continue;
            }
            let target = Rect::new(
                col_x + lane_w * placement.lane,
                grid.y + 1 + (placement.top - first_row) as u16,
                lane_w,
                (placement.bottom - placement.top)
                    .min((grid.height - 1) as u32 - (placement.top - first_row))
                    as u16,
            );
            let proposed = ghosts.iter().any(|g| g.id == placement.event.id);
            let style = if proposed {
                t.accent_style()
            } else {
                t.dimmed()
            };
            let landed = app.effects.applied_ids.contains(&placement.event.id);
            let marker = if proposed && !landed { "┊" } else { "▌" };
            let label = format!("{marker}{}", placement.event.title);
            // The rail spans the whole interval, as in the week grid.
            let mut lines = vec![ratatui::text::Line::from(label.clone())];
            lines.resize(target.height.max(1) as usize, marker.into());
            frame.render_widget(Paragraph::new(lines).style(style), target);
            if proposed && ready && app.effects.on("task flights") {
                let p = (Effects::age(app.effects.proposal) / 0.85).min(1.);
                if p < 1. {
                    let eased = 1. - (1. - p).powi(3);
                    let source_y = inner.y + 2;
                    let source_x = target.x.saturating_sub(8).max(inner.x);
                    let x = source_x + ((target.x - source_x) as f32 * eased).round() as u16;
                    let y =
                        source_y + ((target.y - source_y) as f32 * eased * eased).round() as u16;
                    // The preview's empty flight gutter is decorative; the real inbox stays intact.
                    let clear =
                        (x..x + target.width).all(|x| frame.buffer_mut()[(x, y)].symbol() == " ");
                    if clear {
                        frame.render_widget(
                            Paragraph::new(label).style(style.add_modifier(Modifier::BOLD)),
                            Rect::new(x, y, target.width, 1),
                        );
                    }
                }
            }
            if proposed && landed {
                super::success_accent::render(app, frame, target, None, grid);
            }
        }
    }
    for row in 0..grid.height.saturating_sub(1) {
        let minute = (first_row + row as u32) * 30;
        if minute < 1440 {
            frame.render_widget(
                Paragraph::new(format!("{:02}:{:02}", minute / 60, minute % 60)).style(t.dimmed()),
                Rect::new(grid.x, grid.y + 1 + row, 5, 1),
            );
        }
    }
    if app.effects.on("proposal reveal") && Effects::age(app.effects.proposal) < 0.5 {
        let x = inner.x + (inner.width as f32 * Effects::age(app.effects.proposal) / 0.5) as u16;
        if x < inner.right() {
            frame.buffer_mut()[(x, inner.y)].set_style(t.selected());
        }
    }
    // Retain explicit unscheduled evidence instead of pretending every task has a slot.
    if let Some(item) = detail.items.iter().find(|i| !i.scheduled) {
        let text = format!(
            "Unscheduled: {}",
            item.explanation.as_deref().unwrap_or("no feasible slot")
        );
        frame.render_widget(
            Paragraph::new(text).style(t.dimmed()),
            Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
        );
    }
}
