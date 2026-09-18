/* Header bar (top, 1 row) + status bar (bottom, 1 row). Visual twin of solverforge-mail's status_bar.rs.  */
/* Header bar (top, 1 row) + status bar (bottom, 1 row). Visual twin of solverforge-mail's status_bar.rs.  */

use chrono::Local;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::keys::View;
use crate::theme::theme;
use crate::ui::util::month_name;

// Braille spinner (same as solverforge-mail)
const SPINNER: &[&str] = &[
    "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{281e}", "\u{2836}", "\u{2834}", "\u{2824}",
];

/* Render the top header bar (1 row). */
pub fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let view_label = match app.view {
        View::Month => format!("{} {}", month_name(app.view_month), app.view_year),
        View::Week => {
            let week_start = app.focused_date
                - chrono::Duration::days(
                    chrono::Datelike::weekday(&app.focused_date).num_days_from_monday() as i64,
                );
            let week_end = week_start + chrono::Duration::days(6);
            format!(
                "{} \u{2014} {}",
                week_start.format("%b %-d"),
                week_end.format("%b %-d, %Y")
            )
        }
        View::Day => app.focused_date.format("%A, %B %-d, %Y").to_string(),
        View::Agenda => "Agenda".to_string(),
        View::EventForm => "New Event".to_string(),
        View::IcalImport => "Import .ics".to_string(),
        _ => "Planner123".to_string(),
    };

    // Google sync indicator
    let google_badge = if crate::google::auth::GoogleClient::is_configured() {
        " \u{f09b}"
    } else {
        ""
    }; // nf-fa-google (approx)

    let title = format!(
        "  \u{f073}  Planner123{}     {}  ",
        google_badge, view_label,
    );

    let paragraph = Paragraph::new(title).style(t.header());
    frame.render_widget(paragraph, area);
}

/* Render the bottom status bar (1 row). */
pub fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    // Build key hint spans
    let hints = crate::keys::hints(&app.view);
    let mut spans: Vec<Span> = Vec::new();

    for (key, desc) in &hints {
        spans.push(Span::styled(format!(" {} ", key), t.status_key()));
        spans.push(Span::styled(format!(" {} ", desc), t.status_desc()));
        spans.push(Span::styled("  ", t.status_bar()));
    }

    // Right side: status message or spinner
    let right = if app.loading {
        let frame_idx = (app.tick_count as usize) % SPINNER.len();
        Span::styled(format!(" {} ", SPINNER[frame_idx]), t.spinner())
    } else if !app.status_message.is_empty() {
        if app.status_is_error {
            Span::styled(format!(" {} ", app.status_message), t.error())
        } else {
            Span::styled(format!(" {} ", app.status_message), t.accent_style())
        }
    } else {
        // Show today's time
        let now = Local::now().format("%H:%M").to_string();
        let google_calendars = app
            .calendars
            .iter()
            .filter(|calendar| calendar.source == crate::models::CalendarSource::Google)
            .count();
        let failed = app
            .calendar_sync_state
            .values()
            .filter(|state| state.last_sync_error_message.is_some())
            .count();
        let pending_outbox: usize = app
            .calendar_sync_state
            .values()
            .map(|state| state.pending_outbox)
            .sum();
        let pending_conflicts: usize = app
            .calendar_sync_state
            .values()
            .map(|state| state.pending_conflicts)
            .sum();
        let (google_state, style) = match crate::google::auth::auth_status().state {
            crate::google::auth::GoogleAuthState::Disconnected => {
                ("google off".to_string(), t.dimmed())
            }
            crate::google::auth::GoogleAuthState::NeedsReauth => {
                ("google reauth".to_string(), t.error())
            }
            crate::google::auth::GoogleAuthState::Connected if failed > 0 => {
                (format!("google failed:{}", failed), t.error())
            }
            crate::google::auth::GoogleAuthState::Connected if pending_conflicts > 0 => {
                (format!("google conflicts:{}", pending_conflicts), t.error())
            }
            crate::google::auth::GoogleAuthState::Connected if pending_outbox > 0 => (
                format!("google pending:{}", pending_outbox),
                t.accent_style(),
            ),
            crate::google::auth::GoogleAuthState::Connected if google_calendars > 0 => {
                ("google synced".to_string(), t.accent_style())
            }
            crate::google::auth::GoogleAuthState::Connected => {
                ("google connected".to_string(), t.dimmed())
            }
        };
        Span::styled(format!(" {}  {} ", google_state, now), style)
    };

    // Left-align hints, right-align status (simplified: just concat)
    let left_line = Line::from(spans).style(t.status_bar());

    // We render two overlapping paragraphs: left for hints, right for status
    let left_para = Paragraph::new(left_line).style(t.status_bar());
    frame.render_widget(left_para, area);

    // Render right-side status (use a right-aligned paragraph)
    let right_text = right.content.to_string();
    let right_style = right.style;
    let right_width = right_text.chars().count() as u16;
    if right_width < area.width {
        let right_area = Rect {
            x: area.x + area.width - right_width,
            y: area.y,
            width: right_width,
            height: 1,
        };
        let right_para = Paragraph::new(right_text).style(right_style);
        frame.render_widget(right_para, right_area);
    }
}
