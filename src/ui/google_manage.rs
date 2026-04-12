use ratatui::{
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

pub fn render_google_manage(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(74, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Google ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let auth_status = crate::google::auth::auth_status();
    let state_label = match auth_status.state {
        crate::google::auth::GoogleAuthState::Connected => "connected",
        crate::google::auth::GoogleAuthState::NeedsReauth => "needs reauth",
        crate::google::auth::GoogleAuthState::Disconnected => "disconnected",
    };
    let imported_count = app
        .calendars
        .iter()
        .filter(|calendar| calendar.source == crate::models::CalendarSource::Google)
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

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status: ", t.form_label()),
            Span::styled(state_label, t.accent_style()),
        ]),
        Line::from(vec![
            Span::styled("  Imported calendars: ", t.form_label()),
            Span::styled(imported_count.to_string(), t.normal()),
        ]),
        Line::from(vec![
            Span::styled("  Pending outbound: ", t.form_label()),
            Span::styled(pending_outbox.to_string(), t.normal()),
            Span::styled("    Conflicts: ", t.form_label()),
            Span::styled(pending_conflicts.to_string(), t.normal()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  r refresh   i import   l login/reconnect   o logout   s sync",
            t.dimmed(),
        )]),
        Line::from(""),
    ];

    let imported_google: Vec<_> = app
        .calendars
        .iter()
        .filter(|calendar| calendar.source == crate::models::CalendarSource::Google)
        .collect();

    if imported_google.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No Google calendars imported yet.",
            t.dimmed(),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  Imported calendars",
            t.accent_style(),
        )]));
        lines.push(Line::from(""));

        for calendar in imported_google {
            let state = app.calendar_sync_state.get(&calendar.id);
            let mut badges = Vec::new();
            if state.map(|state| state.writable).unwrap_or(true) {
                badges.push("writable".to_string());
            } else {
                badges.push("read-only".to_string());
            }
            if let Some(access_role) = state.and_then(|state| state.google_access_role.as_deref()) {
                badges.push(access_role.to_string());
            }
            if let Some(state) = state {
                if state.pending_outbox > 0 {
                    badges.push(format!("pending {}", state.pending_outbox));
                }
                if state.pending_conflicts > 0 {
                    badges.push(format!("conflicts {}", state.pending_conflicts));
                }
                if let Some(last_synced_at) = state.last_synced_at.as_deref() {
                    badges.push(format!("synced {}", last_synced_at));
                }
                if state.last_sync_error_message.is_some() {
                    badges.push("error".to_string());
                }
            }
            lines.push(Line::from(vec![
                Span::styled("  ", t.normal()),
                Span::styled(
                    format!("{} [{}]", calendar.name, badges.join(", ")),
                    t.normal(),
                ),
            ]));
            if let Some(error) = state.and_then(|state| state.last_sync_error_message.as_deref()) {
                lines.push(Line::from(vec![
                    Span::styled("    ", t.normal()),
                    Span::styled(error.to_string(), t.error()),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    if app.google_discovered_calendars.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No Google calendars loaded yet. Press r to discover them.",
            t.dimmed(),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  Discoverable calendars",
            t.accent_style(),
        )]));
        lines.push(Line::from(""));

        for (index, calendar) in app.google_discovered_calendars.iter().enumerate() {
            let is_selected = index == app.google_discovery_index;
            let imported = app
                .calendars
                .iter()
                .any(|local| local.google_id.as_deref() == Some(calendar.google_id.as_str()));
            let mut badges = Vec::new();
            if calendar.primary {
                badges.push("primary");
            }
            if imported {
                badges.push("imported");
            }
            if !calendar.writable {
                badges.push("read-only");
            }
            if let Some(access_role) = calendar.access_role.as_deref() {
                badges.push(access_role);
            }

            let label = if badges.is_empty() {
                calendar.name.clone()
            } else {
                format!("{} [{}]", calendar.name, badges.join(", "))
            };
            lines.push(Line::from(vec![
                Span::styled(if is_selected { "▶ " } else { "  " }, t.accent_style()),
                Span::styled(
                    label,
                    if is_selected {
                        t.form_focused()
                    } else {
                        t.normal()
                    },
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}
