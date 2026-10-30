use super::Effects;
use crate::{keys::View, theme::theme};
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

// Decorations never own a glyph or a semantic fill. Canonical UI is drawn first;
// only truly empty, unstyled cells are eligible, before popups are composited.
pub fn composite(fx: &Effects, view: &View, buffer: &mut Buffer, area: Rect) {
    if !fx.preferences.enabled
        || !matches!(view, View::Month | View::Week | View::Day | View::Agenda)
    {
        return;
    }
    let ambient = match view {
        View::Month => fx.on("plasma"),
        View::Week | View::Day => fx.on("copper"),
        View::Agenda => fx.on("agenda stars"),
        _ => false,
    };
    if !(ambient || fx.on("view reveal") && Effects::age(fx.reveal) < 0.32) {
        return;
    }
    let t = theme();
    let time = fx.started.elapsed().as_secs_f32();
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    for y in inner.y..inner.bottom() {
        for x in inner.x..inner.right() {
            if x > buffer.area.x
                && unicode_width::UnicodeWidthStr::width(buffer[(x - 1, y)].symbol()) > 1
            {
                continue;
            }
            let cell = &mut buffer[(x, y)];
            let empty = cell.symbol() == " "
                && (cell.bg == Color::Reset || cell.bg == t.background)
                && cell.modifier.is_empty();
            if !empty {
                continue;
            }
            let wave = match view {
                View::Month if fx.on("plasma") => Some(
                    ((x as f32 * 0.09 + time * 0.4).sin() + (y as f32 * 0.3 - time * 0.5).sin())
                        * 0.5,
                ),
                View::Week | View::Day if fx.on("copper") => {
                    Some((y as f32 * 0.4 - time * 0.7).sin())
                }
                _ => None,
            };
            if let Some(w) = wave {
                cell.set_bg(blend(t.background, t.accent, 0.025 + (w + 1.) * 0.025));
            }
            if matches!(view, View::Agenda) && fx.on("agenda stars") {
                // Three speeds give parallax; bounded hash avoids a viewport-sized particle store.
                for layer in 0..3u32 {
                    let offset = (time * (0.35 + layer as f32 * 0.5)) as u32;
                    let hash = (x as u32 + offset).wrapping_mul(73856093)
                        ^ (y as u32).wrapping_mul(19349663)
                        ^ layer.wrapping_mul(83492791);
                    if hash.is_multiple_of(337) {
                        cell.set_symbol(if layer == 0 { "·" } else { "⋅" });
                        let edge = y == inner.y || y + 1 == inner.bottom();
                        cell.set_fg(blend(
                            t.background,
                            t.accent,
                            if edge && fx.on("edge fades") {
                                0.15
                            } else {
                                0.38
                            },
                        ));
                    }
                }
            }
        }
    }
    if fx.on("view reveal") && Effects::age(fx.reveal) < 0.32 {
        // Style-only wipe: no intermediate frame ever hides working calendar data.
        let p = Effects::age(fx.reveal) / 0.32;
        let x = inner.x + (inner.width as f32 * p) as u16;
        if x < inner.right() {
            for y in inner.y..inner.bottom() {
                let c = &mut buffer[(x, y)];
                if c.symbol() == " " && (c.bg == t.background || c.bg == Color::Reset) {
                    c.set_bg(blend(t.background, t.accent, 0.16));
                }
            }
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
