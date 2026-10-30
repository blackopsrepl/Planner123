use super::Effects;
use crate::{keys::View, theme::theme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};
use unicode_width::UnicodeWidthStr;

// Canonical UI is drawn first; decorations never replace a glyph or a semantic
// fill. A background tint covers every plain cell, glyphs included: tinting only
// blank cells leaves each letter, border and hour line in a dark cut-out.
pub fn composite(fx: &Effects, view: &View, buffer: &mut Buffer, area: Rect) {
    if !fx.preferences.enabled
        || !matches!(view, View::Month | View::Week | View::Day | View::Agenda)
    {
        return;
    }
    let tint = match view {
        View::Month => fx.on("plasma"),
        View::Week | View::Day => fx.on("copper"),
        _ => false,
    };
    let stars = matches!(view, View::Agenda) && fx.on("agenda stars");
    let reveal = fx.on("view reveal") && Effects::age(fx.reveal) < 0.32;
    if !(tint || stars || reveal) {
        return;
    }
    let t = theme();
    let time = fx.started.elapsed().as_secs_f32();
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let plain = |c: &ratatui::buffer::Cell| {
        (c.bg == Color::Reset || c.bg == t.background) && !c.modifier.contains(Modifier::REVERSED)
    };
    // Stars are placed against the canonical frame, so one star never blocks the
    // cell its neighbour fades into.
    let mut lit = Vec::new();
    for y in inner.y..inner.bottom() {
        for x in inner.x..inner.right() {
            if tint && plain(&buffer[(x, y)]) {
                let w = if matches!(view, View::Month) {
                    ((x as f32 * 0.09 + time * 0.4).sin() + (y as f32 * 0.3 - time * 0.5).sin())
                        * 0.5
                } else {
                    (y as f32 * 0.4 - time * 0.7).sin()
                };
                buffer[(x, y)].set_bg(blend(t.background, t.accent, 0.025 + (w + 1.) * 0.025));
            }
            if stars {
                lit.extend(star(
                    buffer,
                    inner,
                    x,
                    y,
                    fx.started.elapsed().as_secs_f64(),
                ));
            }
        }
    }
    for (x, y, light, layer) in lit {
        let edge = (y == inner.y || y + 1 == inner.bottom()) && fx.on("edge fades");
        let peak = if edge { 0.15 } else { 0.38 };
        let cell = &mut buffer[(x, y)];
        cell.set_symbol(if layer == 0 { "·" } else { "⋅" });
        cell.set_fg(blend(t.background, t.accent, peak * light));
    }
    if reveal {
        // Style-only wipe: no intermediate frame ever hides working calendar data.
        let x = inner.x + (inner.width as f32 * Effects::age(fx.reveal) / 0.32) as u16;
        if x < inner.right() {
            for y in inner.y..inner.bottom() {
                if plain(&buffer[(x, y)]) {
                    buffer[(x, y)].set_bg(blend(t.background, t.accent, 0.16));
                }
            }
        }
    }
}

/// A cell a star may occupy: empty, not the tail of a wide glyph, and at least
/// two empty cells clear of any text, so stars never sit between or beside words.
fn blank(buffer: &Buffer, inner: Rect, x: u16, y: u16) -> bool {
    let empty = |x: u16| {
        x < inner.x
            || x >= inner.right()
            || (buffer[(x, y)].symbol() == " "
                && (x == buffer.area.x || buffer[(x - 1, y)].symbol().width() <= 1))
    };
    (x.saturating_sub(2)..=x.saturating_add(2)).all(empty)
}

/// Depth of the three parallax layers. As in the Planner06 starfield, a star
/// drifts left at 14 cells a second times its depth, and nearer stars are brighter.
const DEPTH: [f32; 3] = [0.25, 0.625, 1.0];

/// A star sits at a fractional column and is shared between the two cells it
/// straddles, so it glides instead of jumping a whole cell at a time. Travel is
/// f64: in f32 the sub-cell fraction is lost after about a day of uptime.
fn star(buffer: &Buffer, inner: Rect, x: u16, y: u16, time: f64) -> Option<(u16, u16, f32, u32)> {
    let mut best = (0f32, 0u32);
    for layer in 0..3u32 {
        let travel = time * 14.0 * DEPTH[layer as usize] as f64;
        let (whole, frac) = (travel.floor() as u64 as u32, travel.fract() as f32);
        let lit = |k: u32| {
            ((k.wrapping_mul(73856093))
                ^ (y as u32).wrapping_mul(19349663)
                ^ layer.wrapping_mul(83492791))
            .is_multiple_of(337)
        };
        let k = (x as u32).wrapping_add(whole);
        let light = ((1. - frac) * lit(k) as u8 as f32
            + frac * lit(k.wrapping_add(1)) as u8 as f32)
            * (0.4 + 0.6 * DEPTH[layer as usize]);
        if light > best.0 {
            best = (light, layer);
        }
    }
    (best.0 >= 0.08 && blank(buffer, inner, x, y)).then_some((x, y, best.0, best.1))
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
