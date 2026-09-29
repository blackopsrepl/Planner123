/* Help popup overlay — keybinding reference, generated from the keymap registry.

The overlay opens with the section for the surface you are looking from, so
what you can press here is on screen without scrolling. The rest of the map
follows for reference. `/` filters it by typing, which is what makes a map this
size a lookup instead of a scroll. */

use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::keys::{self, View};
use crate::theme::theme;
use crate::ui::util::centered_rect;

pub fn render_help(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(70, 85, frame.area());

    frame.render_widget(Clear, area);

    let title = if app.help_filtering {
        format!(" Keybindings — filter: {}▏ ", app.help_query)
    } else if app.help_query.is_empty() {
        " Keybindings ".to_string()
    } else {
        format!(" Keybindings — /{} ", app.help_query)
    };

    let block = Block::default()
        .title(title)
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let lines = help_lines(&app.view, &app.return_view, &app.help_query);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0));

    frame.render_widget(paragraph, area);
}

/* One heading and its key lines per section, the current surface first, filtered
by whatever has been typed after `/`. */
fn help_lines(view: &View, came_from: &View, query: &str) -> Vec<Line<'static>> {
    let current = keys::section_of(view).filter(|section| *section != keys::Section::Help);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for (section, title, rows) in keys::help_sections_for(view, came_from, query) {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(heading(title, Some(section) == current));
        for (key_name, desc) in rows {
            lines.push(binding(key_name, desc));
        }
    }

    if lines.is_empty() {
        let t = theme();
        lines.push(Line::from(Span::styled(
            format!("  no key matches \u{201c}{query}\u{201d}"),
            t.dimmed(),
        )));
    }

    lines
}

/* The surface you are on is the one you are reading for, so it gets the accent
and a marker; the other sections stay quiet until you scroll to them. */
fn heading(title: &str, is_current: bool) -> Line<'static> {
    let t = theme();
    if is_current {
        return Line::from(Span::styled(
            format!("\u{25b8} {title}"),
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ));
    }
    Line::from(Span::styled(
        format!("  {title}"),
        t.header_label().add_modifier(Modifier::UNDERLINED),
    ))
}

fn binding(key_name: &str, desc: &str) -> Line<'static> {
    let t = theme();
    Line::from(vec![
        Span::styled(format!("  {:<20}", key_name), t.header_label()),
        Span::styled(desc.to_string(), t.normal()),
    ])
}
