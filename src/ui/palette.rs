/* Command palette overlay and the go-to-date prompt.

The palette is a filter line over the command list; the date prompt borrows the
status row, exactly like the quick-add bar it replaces. */

use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/* Rows of matches shown at once. */
const VISIBLE: usize = 10;

pub fn render_palette(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(60, 55, frame.area());

    frame.render_widget(Clear, area);

    let matches = app.palette_matches();

    /* Keep the selection in view without rebuilding the whole list. */
    let first = app.palette_index.saturating_sub(VISIBLE - 1);
    let items: Vec<ListItem> = matches
        .iter()
        .skip(first)
        .take(VISIBLE)
        .enumerate()
        .map(|(offset, command)| {
            let index = first + offset;
            let selected = index == app.palette_index;
            let marker = if selected { "> " } else { "  " };
            let style = if selected {
                t.selected().add_modifier(Modifier::BOLD)
            } else {
                t.normal()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", command.label),
                style,
            )))
        })
        .collect();

    let prompt = Line::from(vec![
        Span::styled(" > ", t.accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(app.palette_input.clone(), t.normal()),
        Span::styled("▏", t.dimmed()),
    ]);

    let counter = if matches.is_empty() {
        " no command matches ".to_string()
    } else {
        format!(" {}/{} ", app.palette_index + 1, matches.len())
    };

    let block = Block::default()
        .title(" Command ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let [prompt_area, counter_area, list_area] = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Fill(1),
    ])
    .areas(inner);

    frame.render_widget(Paragraph::new(prompt), prompt_area);
    frame.render_widget(
        Paragraph::new(Span::styled(counter, t.dimmed())),
        counter_area,
    );
    frame.render_widget(List::new(items), list_area);
}

/* One line at the bottom of the screen: " Go to date: +21  (YYYY-MM-DD or ±N) ". */
pub fn render_date_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let line = Line::from(vec![
        Span::styled(" Go to date: ", t.status_key()),
        Span::styled(app.date_jump_input.clone(), t.normal()),
        Span::styled("▏", t.dimmed()),
        Span::styled("   YYYY-MM-DD or ±N days   ", t.status_desc()),
        Span::styled("Enter go  Esc cancel ", t.status_desc()),
    ]);
    frame.render_widget(Paragraph::new(line).style(t.status_bar()), area);
}
