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

/* Render the bottom status bar (1 row).

The row is shared: the status block owns a bounded share on the right and the
key chips take what is left. Chips are dropped from the tail of their priority
order rather than clipped, so the bar never shows half a hint, and the chip
that opens help is reserved before anything else. */
pub fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

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

    /* The status block is capped so a long error message cannot take the bar
    from the key hints, and the hints are budgeted against what it actually
    uses. */
    let right_text = crate::ui::util::truncate(
        right.content.as_ref(),
        status_block_limit(area.width) as usize,
    );
    let right_style = right.style;
    let right_width = right_text.chars().count() as u16;

    let budget = area.width.saturating_sub(right_width).saturating_sub(1);
    let mut spans: Vec<Span> = Vec::new();
    match &app.pending_confirm {
        /* While a confirmation is armed the only meaningful keys are the answer,
        so the hints step aside for the question. */
        Some(confirm) => {
            spans.push(Span::styled(format!(" {}  ", confirm.prompt), t.error()));
            spans.push(Span::styled("y confirm", t.status_key()));
            spans.push(Span::styled("   any other key cancels ", t.status_desc()));
        }
        None => {
            for (key, desc) in crate::keys::hints_within(&app.view, budget) {
                spans.push(Span::styled(format!(" {} ", key), t.status_key()));
                spans.push(Span::styled(format!(" {} ", desc), t.status_desc()));
                spans.push(Span::styled("  ", t.status_bar()));
            }
        }
    }

    let left_line = Line::from(spans).style(t.status_bar());
    let left_para = Paragraph::new(left_line).style(t.status_bar());
    frame.render_widget(left_para, area);

    if right_width <= area.width {
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

/* The widest the status block may be: a third of the bar, never more than the
bar minus the room a single hint needs. */
fn status_block_limit(width: u16) -> u16 {
    (width / 3).max(16).min(width.saturating_sub(8))
}
