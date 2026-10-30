/* Month grid view — 7-column calendar grid with colored dots, today highlight, adjacent month dimming, and expandable selected-day cell showing event titles.  */
/* Month grid view — 7-column calendar grid with colored dots, today highlight, adjacent month dimming, and expandable selected-day cell showing event titles.  */

use chrono::{Datelike, Duration, Local, NaiveDate};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::{truncate, WEEKDAY_SHORT};

pub fn render_month(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let block = Block::default()
        .title(format!(
            " {} {} ",
            crate::ui::util::month_name(app.view_month),
            app.view_year
        ))
        .title_style(t.accent_style().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(t.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Header row: Mon Tue Wed Thu Fri Sat Sun
    let header_height = 1u16;
    let [header_area, grid_area] =
        Layout::vertical([Constraint::Length(header_height), Constraint::Fill(1)]).areas(inner);

    render_weekday_header(frame, header_area);
    render_grid(app, frame, grid_area);
}

fn render_weekday_header(frame: &mut Frame, area: Rect) {
    let t = theme();
    if area.width < 7 {
        return;
    }
    let col_w = area.width / 7;

    let spans: Vec<Span> = WEEKDAY_SHORT
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i >= 5 {
                t.weekend()
            } else {
                t.header_label()
            };
            Span::styled(format!("{:^width$}", name, width = col_w as usize), style)
        })
        .collect();

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

struct DayCellFlags {
    is_current_month: bool,
    is_today: bool,
    is_selected: bool,
    is_weekend: bool,
}

fn render_grid(app: &App, frame: &mut Frame, area: Rect) {
    let today = Local::now().date_naive();

    // First day of the displayed month
    let first_of_month = NaiveDate::from_ymd_opt(app.view_year, app.view_month, 1).unwrap_or(today);

    // Monday-based: find the first cell date
    let first_weekday = chrono::Datelike::weekday(&first_of_month).num_days_from_monday() as i64;
    let grid_start = first_of_month - Duration::days(first_weekday);

    // 6 rows × 7 cols
    let num_rows: u16 = 6;
    let row_height = area.height / num_rows;
    let col_width = area.width / 7;
    if row_height == 0 || col_width == 0 {
        return;
    }

    for row in 0..num_rows {
        for col in 0u16..7 {
            let day_offset = row as i64 * 7 + col as i64;
            let date = grid_start + Duration::days(day_offset);
            let flags = DayCellFlags {
                is_current_month: date.month() == app.view_month && date.year() == app.view_year,
                is_today: date == today,
                is_selected: date == app.focused_date,
                is_weekend: col >= 5,
            };

            let cell_rect = Rect {
                x: area.x + col * col_width,
                y: area.y + row * row_height,
                width: col_width,
                height: row_height,
            };

            render_day_cell(app, frame, cell_rect, date, &flags);
        }
    }
}

fn render_day_cell(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    date: NaiveDate,
    flags: &DayCellFlags,
) {
    let DayCellFlags {
        is_current_month,
        is_today,
        is_selected,
        is_weekend,
    } = *flags;
    let t = theme();
    let events = app.events_on_date(date);

    // Day number style
    let day_style = if !is_current_month {
        t.adjacent_month()
    } else if is_today {
        t.today_cell()
    } else if is_weekend {
        t.weekend()
    } else {
        t.normal()
    };

    // Border style for selected day
    let border_style = if is_selected {
        t.border_focused()
    } else {
        Style::default().fg(Color::Rgb(30, 32, 50)) // very dark, almost invisible grid lines
    };

    // Day cell block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Day number (top-left of cell)
    let day_num = format!("{:2}", date.day());
    let day_num_style = if is_today {
        t.today_cell().add_modifier(Modifier::BOLD)
    } else {
        day_style
    };

    // First line: day number
    let mut lines: Vec<Line> = Vec::new();
    let day_line = Line::from(Span::styled(day_num, day_num_style));
    lines.push(day_line);

    // If selected, show event titles; otherwise show colored dots
    if is_selected && inner.height > 2 && !events.is_empty() {
        // Expanded: show first (height-1) event titles
        for ev in events
            .iter()
            .take((inner.height as usize).saturating_sub(1))
        {
            let cal_idx = app.calendar_index_for(&ev.calendar_id);
            let cal_color = t.calendar_color(cal_idx);
            let title = truncate(&ev.title, inner.width.saturating_sub(3) as usize);
            lines.push(Line::from(vec![
                Span::styled("\u{2590}", Style::default().fg(cal_color)), // ▐ rail
                Span::styled(" ", t.normal()),
                Span::styled(title, t.event_title()),
            ]));
        }
    } else if !events.is_empty() && inner.height > 1 {
        // Compact: colored dots (one per distinct calendar that has events)
        let mut dot_line_spans: Vec<Span> = vec![Span::styled(" ", t.normal())];
        let mut seen_cals = std::collections::HashSet::new();
        let mut dot_count = 0;

        for ev in &events {
            if seen_cals.insert(&ev.calendar_id) {
                let cal_idx = app.calendar_index_for(&ev.calendar_id);
                let cal_color = t.calendar_color(cal_idx);
                dot_line_spans.push(Span::styled(
                    "\u{25cf}", // ●
                    Style::default().fg(cal_color).add_modifier(Modifier::BOLD),
                ));
                dot_count += 1;
            }
            if dot_count >= (inner.width as usize / 2).max(1) {
                break;
            }
        }

        // Event count if more than shown
        let extra = events.len().saturating_sub(dot_count);
        if extra > 0 {
            dot_line_spans.push(Span::styled(format!("+{}", extra), t.dimmed()));
        }

        lines.push(Line::from(dot_line_spans));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
    if let Some(event) = events
        .iter()
        .find(|e| app.effects.applied_ids.contains(&e.id))
    {
        let target = Rect::new(
            inner.x,
            inner.y + 1,
            inner.width,
            inner.height.saturating_sub(1),
        );
        super::success_accent::render(app, frame, target, Some(&event.id), inner);
    }
}
