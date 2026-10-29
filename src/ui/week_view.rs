use chrono::{Datelike, Duration, Local, NaiveDate};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::time_grid::render_time_grid;
use crate::ui::util::WEEKDAY_SHORT;

pub fn render_week(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let block = Block::default()
        .title(" Week ")
        .title_style(t.accent_style().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(t.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let days_from_mon = chrono::Datelike::weekday(&app.focused_date).num_days_from_monday() as i64;
    let week_start = app.focused_date - Duration::days(days_from_mon);

    let [header_area, grid_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

    render_week_header(app, frame, header_area, week_start);
    render_time_grid(app, frame, grid_area, week_start, 7);
}

fn render_week_header(app: &App, frame: &mut Frame, area: Rect, week_start: NaiveDate) {
    let t = theme();
    let today = Local::now().date_naive();
    let time_col_w = 6u16;
    let day_col_w = area.width.saturating_sub(time_col_w) / 7;

    let mut spans: Vec<Span> = vec![Span::styled(format!("{:^6}", ""), t.dimmed())];

    for d in 0..7u32 {
        let date = week_start + Duration::days(d as i64);
        let is_today = date == today;
        let is_selected = date == app.focused_date;
        let weekday_name = WEEKDAY_SHORT[d as usize];
        let label = format!("{} {}", weekday_name, date.day());
        let padded = format!("{:^width$}", label, width = day_col_w as usize);

        let style = if is_today {
            t.today_cell().add_modifier(Modifier::BOLD)
        } else if is_selected {
            t.accent_style().add_modifier(Modifier::BOLD)
        } else if d >= 5 {
            t.weekend()
        } else {
            t.header_label()
        };
        spans.push(Span::styled(padded, style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
