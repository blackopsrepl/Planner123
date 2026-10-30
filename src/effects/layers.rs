use super::Effects;
use crate::{keys::View, theme::theme};
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

// Calendar views carry no ambient background: a decorated canvas costs
// readability and keeps the app redrawing. The only layer here is the short
// entry wipe, and it only ever styles empty cells, never a glyph or a fill.
pub fn composite(fx: &Effects, view: &View, buffer: &mut Buffer, area: Rect) {
    if !fx.on("view reveal")
        || Effects::age(fx.reveal) >= 0.32
        || !matches!(view, View::Month | View::Week | View::Day | View::Agenda)
    {
        return;
    }
    let t = theme();
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let x = inner.x + (inner.width as f32 * Effects::age(fx.reveal) / 0.32) as u16;
    if x >= inner.right() {
        return;
    }
    for y in inner.y..inner.bottom() {
        if x > buffer.area.x
            && unicode_width::UnicodeWidthStr::width(buffer[(x - 1, y)].symbol()) > 1
        {
            continue;
        }
        let c = &mut buffer[(x, y)];
        if c.symbol() == " "
            && c.modifier.is_empty()
            && (c.bg == t.background || c.bg == Color::Reset)
        {
            c.set_bg(blend(t.background, t.accent, 0.16));
        }
    }
}

pub fn blend(base: Color, accent: Color, amount: f32) -> Color {
    let rgb = |c| match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        _ => (32, 32, 40),
    };
    let (r, g, b) = rgb(base);
    let (a, c, d) = rgb(accent);
    let mix = |v: u8, w: u8| (v as f32 + (w as f32 - v as f32) * amount) as u8;
    Color::Rgb(mix(r, a), mix(g, c), mix(b, d))
}
