/* The all-day strip above the hours: one row, the first all-day event of each
day with `+N` for the rest. Drawn only when the shown days have any. */

use chrono::NaiveDate;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::TIME_COL_W;
use crate::app::App;
use crate::theme::theme;
use crate::ui::util::truncate;

/* Draws the strip on the first row of `area`; returns whether it took the row. */
pub(super) fn render(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    days: &[NaiveDate],
    day_col_w: u16,
) -> bool {
    let events = app.visible_occurrences();
    let per_day: Vec<Vec<_>> = days
        .iter()
        .map(|date| {
            events
                .iter()
                .filter(|e| e.all_day && e.occurs_on(*date))
                .collect()
        })
        .collect();
    if area.height <= 1 || per_day.iter().all(Vec::is_empty) {
        return false;
    }

    let t = theme();
    let width = day_col_w as usize;
    let mut spans = vec![Span::styled(
        format!("{:<w$}", "all", w = TIME_COL_W as usize),
        t.dimmed(),
    )];
    for day in per_day {
        let Some(first) = day.first() else {
            spans.push(Span::raw(" ".repeat(width)));
            continue;
        };
        let more = match day.len() {
            1 => String::new(),
            n => format!(" +{}", n - 1),
        };
        let title = truncate(&first.title, width.saturating_sub(1 + more.width()));
        let pad = width.saturating_sub(1 + title.width() + more.width());
        let rail = t.event_rail(app.calendar_index_for(&first.calendar_id));
        spans.push(Span::styled("\u{258c}", rail));
        spans.push(Span::styled(title, t.event_title()));
        spans.push(Span::styled(more, t.dimmed()));
        spans.push(Span::raw(" ".repeat(pad)));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect { height: 1, ..area },
    );
    true
}
