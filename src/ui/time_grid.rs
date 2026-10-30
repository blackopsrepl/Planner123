/* The hour grid shared by the week view (seven columns) and the day view (one).

Hour labels and faint rules are drawn first, then the now beam, then every
timed event as a calendar-coloured rail block placed by `layout::layout_day`.
All-day events sit in a strip above the hours instead of covering the day. */

mod all_day;
pub(crate) mod layout;

use chrono::{Duration, Local, NaiveDate, Timelike};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::truncate;

use layout::{layout_day, Placement, ROWS_PER_DAY, ROWS_PER_HOUR};

const TIME_COL_W: u16 = 6;
/// Narrowest lane that still shows a rail and one title character.
const MIN_LANE_W: u16 = 2;

/* The visible slice of the day: the screen area of the hours, and which grid
rows `[first_row, first_row + visible_rows)` are on screen. */
struct Window {
    area: Rect,
    first_row: u32,
    visible_rows: u32,
}

impl Window {
    /* Full-width screen row of grid row `row`, when it is visible. */
    fn row_rect(&self, row: u32) -> Option<Rect> {
        (row >= self.first_row && row < self.first_row + self.visible_rows).then(|| Rect {
            y: self.area.y + (row - self.first_row) as u16,
            height: 1,
            ..self.area
        })
    }
}

/* One day column on screen. */
struct Column {
    x: u16,
    width: u16,
}

pub fn render_time_grid(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    start_date: NaiveDate,
    num_days: u32,
) {
    let day_col_w = area.width.saturating_sub(TIME_COL_W) / num_days as u16;
    let days: Vec<NaiveDate> = (0..num_days)
        .map(|d| start_date + Duration::days(d as i64))
        .collect();
    let events = app.visible_occurrences();

    let mut grid = area;
    if all_day::render(app, frame, area, &days, day_col_w) {
        grid.y += 1;
        grid.height -= 1;
    }

    let first_row = (app.week_scroll.max(0) as u32 * ROWS_PER_HOUR).min(ROWS_PER_DAY - 1);
    let window = Window {
        area: grid,
        first_row,
        visible_rows: (grid.height as u32).min(ROWS_PER_DAY - first_row),
    };
    let today_shown = days.contains(&Local::now().date_naive());
    render_hour_rules(frame, &window, today_shown);
    if today_shown {
        render_now_beam(app, frame, &window);
    }

    for (d, date) in days.iter().enumerate() {
        let column = Column {
            x: grid.x + TIME_COL_W + d as u16 * day_col_w,
            width: day_col_w,
        };
        for placement in layout_day(&events, *date) {
            render_event(app, frame, &placement, &column, &window);
        }
    }
}

fn render_hour_rules(frame: &mut Frame, window: &Window, today_shown: bool) {
    let t = theme();
    let rule = Style::default().fg(Color::Rgb(20, 22, 40));
    let now_hour = Local::now().hour();
    let rule_w = window.area.width.saturating_sub(TIME_COL_W) as usize;
    for hour in 0..24 {
        let Some(rect) = window.row_rect(hour * ROWS_PER_HOUR) else {
            continue;
        };
        let label_style = if today_shown && hour < now_hour {
            t.past_dim()
        } else {
            t.dimmed()
        };
        let line = Line::from(vec![
            Span::styled(format!("{:02}:00 ", hour), label_style),
            Span::styled("\u{2500}".repeat(rule_w), rule),
        ]);
        frame.render_widget(Paragraph::new(line), rect);
    }
}

fn render_now_beam(app: &App, frame: &mut Frame, window: &Window) {
    let now = Local::now();
    let row = (now.hour() * 60 + now.minute()) * ROWS_PER_HOUR / 60;
    let Some(rect) = window.row_rect(row) else {
        return;
    };
    let t = theme();
    let beam_char = match (app.tick_count / 2) % 3 {
        0 => "\u{2501}",
        1 => "\u{2500}",
        _ => "\u{254c}",
    };
    let style = t.now_beam().add_modifier(Modifier::BOLD);
    let line = Line::from(vec![
        Span::styled(format!("{:02}:{:02} ", now.hour(), now.minute()), style),
        Span::styled(
            beam_char.repeat(rect.width.saturating_sub(TIME_COL_W) as usize),
            style,
        ),
    ]);
    frame.render_widget(Paragraph::new(line), rect);
}

fn render_event(
    app: &App,
    frame: &mut Frame,
    placement: &Placement<'_>,
    column: &Column,
    window: &Window,
) {
    let top = placement.top.max(window.first_row);
    let bottom = placement.bottom.min(window.first_row + window.visible_rows);
    if top >= bottom {
        return;
    }
    let y = window.area.y + (top - window.first_row) as u16;
    // Leave the last column of each day blank so adjacent days stay apart.
    let usable = column.width.saturating_sub(1);
    let lanes = placement.lanes.max(1);
    let lane_w = usable / lanes;
    if lane_w < MIN_LANE_W {
        return render_crowded_marker(frame, placement, column.x, usable, y);
    }
    let width = if placement.lane + 1 == lanes {
        usable - placement.lane * lane_w
    } else {
        lane_w
    };
    let area = Rect {
        x: column.x + placement.lane * lane_w,
        y,
        width,
        height: (bottom - top) as u16,
    };

    let t = theme();
    let ev = placement.event;
    let rail = Span::styled(
        "\u{258c}",
        t.event_rail(app.calendar_index_for(&ev.calendar_id)),
    );
    let is_past = ev.end_dt().is_some_and(|end| end < chrono::Utc::now());
    let title_style = if is_past {
        t.past_dim()
    } else {
        t.event_title()
    };
    let text_w = (width as usize).saturating_sub(1);

    let mut lines = vec![Line::from(vec![
        rail.clone(),
        Span::styled(truncate(&ev.title, text_w), title_style),
    ])];
    if area.height >= 2 {
        let second = if ev.project_id.is_some() {
            Span::styled("[P]", t.project_badge())
        } else {
            let start = ev
                .start_local_dt()
                .map(|s| format!("{:02}:{:02}", s.hour(), s.minute()));
            Span::styled(truncate(&start.unwrap_or_default(), text_w), t.dimmed())
        };
        lines.push(Line::from(vec![rail.clone(), second]));
    }
    while lines.len() < area.height as usize {
        lines.push(Line::from(rail.clone()));
    }
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines), area);
    super::success_accent::render(app, frame, area, Some(&ev.id), window.area);
}

/* A cluster too wide for its column: lane 0 draws one `+N` marker (N is the
cluster's lane count) in place of lanes that cannot be shown, so a crowded
slot is never silently empty. */
fn render_crowded_marker(
    frame: &mut Frame,
    placement: &Placement<'_>,
    x: u16,
    usable: u16,
    y: u16,
) {
    if placement.lane != 0 || usable == 0 {
        return;
    }
    let t = theme();
    let marker = truncate(&format!("+{}", placement.lanes), usable as usize);
    let area = Rect {
        x,
        y,
        width: usable,
        height: 1,
    };
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(Span::styled(marker, t.accent_style())), area);
}
