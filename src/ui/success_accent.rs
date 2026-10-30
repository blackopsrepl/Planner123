use crate::{
    app::App,
    effects::{layers::blend, Effects},
    theme::theme,
};
use ratatui::{layout::Rect, Frame};

pub fn render(app: &App, frame: &mut Frame, area: Rect, id: Option<&str>, clip: Rect) {
    if id.is_some_and(|id| !app.effects.applied_ids.iter().any(|i| i == id)) {
        return;
    }
    let age = Effects::age(app.effects.applied);
    if age >= 1.1 || area.width == 0 || area.height == 0 {
        return;
    }
    let t = theme();
    if app.effects.on("apply sweep") {
        let x = area.x + (area.width as f32 * (age / 1.1)) as u16;
        if x < area.right() {
            for y in area.y..area.bottom() {
                let c = &mut frame.buffer_mut()[(x, y)];
                // Foreground and Unicode cell occupancy remain canonical.
                if c.bg != t.selection_bg
                    && !c.modifier.contains(ratatui::style::Modifier::REVERSED)
                {
                    c.set_bg(blend(t.background, t.accent, 0.24));
                }
            }
        }
    }
    if app.effects.on("landing ripples") {
        let radius = (age * 5.) as u16;
        let bounds = clip.intersection(frame.area());
        for dy in -1i32..=1 {
            for dx in [-1i32, 1] {
                let x = area.x as i32 + dx * radius as i32;
                let y = area.y as i32 + dy * radius as i32 / 2;
                if x < bounds.x as i32
                    || y < bounds.y as i32
                    || x >= bounds.right() as i32
                    || y >= bounds.bottom() as i32
                {
                    continue;
                }
                if x > bounds.x as i32
                    && unicode_width::UnicodeWidthStr::width(
                        frame.buffer_mut()[((x - 1) as u16, y as u16)].symbol(),
                    ) > 1
                {
                    continue;
                }
                let c = &mut frame.buffer_mut()[(x as u16, y as u16)];
                if c.symbol() == " "
                    && c.modifier.is_empty()
                    && (c.bg == ratatui::style::Color::Reset || c.bg == t.background)
                {
                    c.set_symbol("·")
                        .set_fg(blend(t.background, t.accent, (1. - age / 1.1) * 0.5));
                }
            }
        }
    }
}
