/* Multi-field event creation and editing form.  */

use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, FormField};
use crate::recurrence::RecurrencePreset;
use crate::theme::theme;
use crate::ui::util::centered_rect;

pub fn render_event_form(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(72, 80, frame.area());

    frame.render_widget(Clear, area);

    let title = if app.form_is_new {
        " New Event "
    } else {
        " Edit Event "
    };
    let block = Block::default()
        .title(title)
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    let fields = &app.form_fields;

    // Build all field lines
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("")); // top padding

    for (i, field) in fields.iter().enumerate() {
        let is_focused = i == app.form_field_index;
        let label_style = if is_focused {
            t.form_label().add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        };
        let value_style = if is_focused {
            t.form_focused()
        } else {
            t.form_value()
        };
        let cursor = if is_focused && app.cursor_visible() {
            "\u{2588}"
        } else {
            ""
        }; // █

        let value = field_value(app, field, cursor);

        let indicator = if is_focused { "▶ " } else { "  " };
        let indicator_style = if is_focused {
            t.accent_style()
        } else {
            t.dimmed()
        };

        lines.push(Line::from(vec![
            Span::styled(indicator, indicator_style),
            Span::styled(format!("{:<14}", field.label()), label_style),
            Span::styled("  ", t.normal()),
            Span::styled(value, value_style),
        ]));

        if i + 1 < fields.len() {
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));
    // Footer hints
    lines.push(Line::from(vec![
        Span::styled("  ", t.normal()),
        Span::styled(" Tab ", t.status_key()),
        Span::styled(" next  ", t.status_desc()),
        Span::styled(" Enter ", t.status_key()),
        Span::styled(" save  ", t.status_desc()),
        Span::styled(" Esc ", t.status_key()),
        Span::styled(" cancel", t.status_desc()),
    ]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn field_value(app: &App, field: &FormField, cursor: &str) -> String {
    match field {
        FormField::Title => format!("{}{}", app.form_title, cursor),
        FormField::Date => format!("{}{}", app.form_date, cursor),
        FormField::StartTime => format!("{}{}", app.form_start_time, cursor),
        FormField::EndTime => format!("{}{}", app.form_end_time, cursor),
        FormField::Timezone => format!("{}{}", app.form_timezone, cursor),
        FormField::Location => {
            if app.form_location.is_empty() && cursor.is_empty() {
                "(optional)".to_string()
            } else {
                format!("{}{}", app.form_location, cursor)
            }
        }
        FormField::Description => {
            if app.form_description.is_empty() && cursor.is_empty() {
                "(optional)".to_string()
            } else {
                format!("{}{}", app.form_description, cursor)
            }
        }
        FormField::Recurrence => {
            let presets = RecurrencePreset::all();
            let label = presets
                .get(app.form_recurrence_index)
                .map(|p| p.label())
                .unwrap_or("—");
            format!("{} (h/l to change)", label)
        }
        FormField::Reminder => {
            if app.form_reminder.is_empty() {
                "none".to_string()
            } else {
                format!("{} min before{}", app.form_reminder, cursor)
            }
        }
        FormField::Calendar => app
            .calendars
            .get(app.form_calendar_index)
            .map(|c| {
                let source = match c.source {
                    crate::models::CalendarSource::Local => "local".to_string(),
                    crate::models::CalendarSource::Google => {
                        if app
                            .calendar_sync_state
                            .get(&c.id)
                            .map(|state| !state.writable)
                            .unwrap_or(false)
                        {
                            "google read-only".to_string()
                        } else {
                            "google writable".to_string()
                        }
                    }
                };
                format!("{} [{}] (h/l)", c.name, source)
            })
            .unwrap_or_else(|| "no calendars".to_string()),
        FormField::Project => {
            if app.form_project_index == 0 {
                "— (none)  (h/l to change)".to_string()
            } else {
                app.projects
                    .get(app.form_project_index - 1)
                    .map(|p| format!("{} (h/l)", p.name))
                    .unwrap_or_else(|| "—".to_string())
            }
        }
        FormField::AllDay => {
            if app.form_all_day {
                "[\u{2588}] Yes (Space to toggle)".to_string()
            } else {
                "[ ] No  (Space to toggle)".to_string()
            }
        }
    }
}
