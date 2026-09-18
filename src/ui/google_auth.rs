/* Google OAuth2 setup wizard popup.  */

use ratatui::{
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

pub fn render_google_auth(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(65, 55, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Google Calendar Setup ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cursor = if app.cursor_visible() {
        "\u{2588}"
    } else {
        " "
    };

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Create Google OAuth credentials using the Desktop app type.",
            t.normal(),
        )]),
        Line::from(vec![Span::styled(
            "  Planner123 opens the system browser and stores the refresh token in the OS keyring.",
            t.normal(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. Go to ", t.dimmed()),
            Span::styled("console.cloud.google.com", t.accent_style()),
        ]),
        Line::from(vec![Span::styled(
            "  2. Create a project → Enable Calendar API",
            t.dimmed(),
        )]),
        Line::from(vec![Span::styled(
            "  3. Create OAuth2 credentials (Desktop app)",
            t.dimmed(),
        )]),
        Line::from(vec![Span::styled(
            "  4. Scope used here: full Google Calendar access",
            t.dimmed(),
        )]),
        Line::from(""),
    ];

    // Client ID field
    let id_style = if app.google_auth_field == 0 {
        t.border_focused()
    } else {
        t.dimmed()
    };
    let id_cursor = if app.google_auth_field == 0 {
        cursor
    } else {
        ""
    };
    lines.push(Line::from(vec![
        Span::styled(
            if app.google_auth_field == 0 {
                "▶ "
            } else {
                "  "
            },
            t.accent_style(),
        ),
        Span::styled("Client ID:      ", t.form_label()),
        Span::styled(
            format!("{}{}", app.google_auth_client_id, id_cursor),
            id_style,
        ),
    ]));
    lines.push(Line::from(""));

    // Client Secret field
    let sec_style = if app.google_auth_field == 1 {
        t.border_focused()
    } else {
        t.dimmed()
    };
    let sec_cursor = if app.google_auth_field == 1 {
        cursor
    } else {
        ""
    };
    let secret_display: String = "*".repeat(app.google_auth_client_secret.len());
    lines.push(Line::from(vec![
        Span::styled(
            if app.google_auth_field == 1 {
                "▶ "
            } else {
                "  "
            },
            t.accent_style(),
        ),
        Span::styled("Client Secret:  ", t.form_label()),
        Span::styled(format!("{}{}", secret_display, sec_cursor), sec_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  ", t.normal()),
        Span::styled(" Tab ", t.status_key()),
        Span::styled(" next field  ", t.status_desc()),
        Span::styled(" Enter ", t.status_key()),
        Span::styled(" authorize  ", t.status_desc()),
        Span::styled(" Esc ", t.status_key()),
        Span::styled(" cancel", t.status_desc()),
    ]));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}
