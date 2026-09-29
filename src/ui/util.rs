/* Layout utility helpers.  */

use ratatui::layout::{Constraint, Layout, Rect};

/* Returns a centered rectangle of `percent_x` × `percent_y` of the given area. */
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

/* Truncate a string to fit within `max_width` terminal columns, appending `…` if truncated. */
pub fn truncate(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let width = unicode_width::UnicodeWidthStr::width(s);
    if width <= max_width {
        s.to_string()
    } else if max_width <= 1 {
        "\u{2026}".to_string()
    } else {
        let mut result = String::new();
        let mut current_width = 0;
        for ch in s.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
            if current_width + ch_width + 1 > max_width {
                break;
            }
            result.push(ch);
            current_width += ch_width;
        }
        result.push('\u{2026}');
        result
    }
}

/* Format a duration in minutes as "Xh Ym" or "Xm". */
pub fn format_duration(minutes: i64) -> String {
    if minutes < 60 {
        format!("{}m", minutes)
    } else {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{}h", h)
        } else {
            format!("{}h {}m", h, m)
        }
    }
}

/* Returns "Today", "Tomorrow", "Yesterday", or formatted date. */
pub fn friendly_date(date: chrono::NaiveDate) -> String {
    let today = chrono::Local::now().date_naive();
    let delta = (date - today).num_days();
    match delta {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        -1 => "Yesterday".to_string(),
        _ => date.format("%A, %b %-d").to_string(),
    }
}

/* Short weekday names for the month grid header. */
pub const WEEKDAY_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/* Month name. */
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "?",
    }
}
