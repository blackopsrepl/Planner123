use super::Effects;
use crate::{glyphfx, theme::theme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "credits_tests.rs"]
mod tests;

pub fn render(fx: &mut Effects, frame: &mut Frame, area: Rect) {
    let t = theme();
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Planner123 · Credits / glyphfx gallery · Esc closes ")
        .style(t.popup());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let time = if fx.preferences.enabled {
        fx.started.elapsed().as_secs_f32()
    } else {
        0.
    };
    let logo_height = inner.height.saturating_sub(5).min(9);
    let logo = Rect::new(inner.x, inner.y, inner.width, logo_height);
    if fx.on("ouroboros") {
        ouroboros(frame.buffer_mut(), logo, time);
    } else {
        frame.render_widget(Paragraph::new("Planner123 · SolverForge · glyphfx"), logo);
    }
    let scroller = Rect::new(
        inner.x,
        logo.bottom(),
        inner.width,
        3.min(inner.bottom().saturating_sub(logo.bottom())),
    );
    let message = "    PLANNER123  ·  REAL CALENDARS, REAL PLANS  ·  SOLVERFORGE  ·  GLYPHFX / MIT  ·  THANK YOU FOR USING FREE SOFTWARE    ";
    if fx.on("sine scroller") {
        let chars: Vec<char> = message.chars().collect();
        for x in 0..scroller.width {
            let ch = chars[(x as usize + (time * 8.) as usize) % chars.len()];
            let y = ((x as f32 * 0.16 + time * 2.).sin() + 1.).round() as u16;
            if y < scroller.height {
                frame.buffer_mut()[(scroller.x + x, scroller.y + y)]
                    .set_char(ch)
                    .set_style(t.accent_style());
            }
        }
    } else {
        frame.render_widget(Paragraph::new(message), scroller);
    }
    let top = scroller.bottom();
    if top >= inner.bottom() {
        return;
    }
    let enabled: Vec<&str> = glyphfx::EFFECTS
        .iter()
        .copied()
        .filter(|e| fx.on(e))
        .collect();
    let gallery = Rect::new(
        inner.x,
        top + 1,
        inner.width,
        inner.bottom().saturating_sub(top + 1),
    );
    if enabled.is_empty() {
        fx.clip = None;
        fx.clip_frame = None;
        frame.render_widget(
            Paragraph::new("glyphfx gallery paused — effects are disabled"),
            Rect::new(inner.x, top, inner.width, 1),
        );
        return;
    }
    fx.gallery_index %= enabled.len();
    let effect = enabled[fx.gallery_index];
    let heading = format!(
        "glyphfx: {effect} ({}/{}) · ←/→ browse · automatic playback",
        fx.gallery_index + 1,
        enabled.len()
    );
    frame.render_widget(
        Paragraph::new(heading).style(t.accent_style()),
        Rect::new(inner.x, top, inner.width, 1),
    );
    if gallery.width == 0 || gallery.height == 0 {
        return;
    }
    if fx.clip_size != gallery {
        fx.clip = None;
        fx.clip_frame = None;
        fx.clip_size = gallery;
    }
    if fx.clip.is_none() && fx.clip_frame.is_none() {
        fx.gallery_started = Instant::now();
        match glyphfx::Clip::centered(effect, GALLERY_TEXT, gallery.width, gallery.height) {
            Ok(clip) => {
                fx.clip = Some(clip);
                fx.diagnostic = None;
            }
            Err(e) => {
                fx.diagnostic = Some(format!("glyphfx {effect}: {e}"));
                fx.clip_frame = Some(Buffer::empty(Rect::new(
                    0,
                    0,
                    gallery.width,
                    gallery.height,
                )));
            }
        }
    }
    if fx.clip_step.elapsed() >= Duration::from_millis(33) {
        fx.clip_step = Instant::now();
        if let Some(clip) = fx.clip.as_mut() {
            if let Some(buffer) = clip.next_frame() {
                fx.clip_frame = Some(buffer);
            }
        }
    }
    if let Some(buffer) = fx.clip_frame.as_ref() {
        copy(buffer, frame.buffer_mut(), gallery);
    } else {
        frame.render_widget(
            Paragraph::new("Planner123 · SolverForge · glyphfx"),
            gallery,
        );
    }
    if let Some(message) = &fx.diagnostic {
        frame.render_widget(Paragraph::new(message.as_str()).style(t.error()), gallery);
    }
    let elapsed = fx.gallery_started.elapsed().as_secs_f32();
    if elapsed > 3. && fx.clip.as_ref().is_none_or(|c| c.is_finished()) {
        fx.gallery_index = (fx.gallery_index + 1) % enabled.len();
        fx.clip = None;
        fx.clip_frame = None;
        fx.gallery_started = Instant::now();
    }
}

const GALLERY_TEXT: &str = "Planner123\nSolverForge · glyphfx\nSafe native terminal effects";

pub fn copy(source: &Buffer, dest: &mut Buffer, target: Rect) {
    let width = source.area.width.min(target.width);
    let height = source.area.height.min(target.height);
    for y in 0..height {
        for x in 0..width {
            dest[(target.x + x, target.y + y)] =
                source[(source.area.x + x, source.area.y + y)].clone();
        }
    }
}

fn ouroboros(buffer: &mut Buffer, area: Rect, time: f32) {
    if area.width < 4 || area.height < 2 {
        return;
    }
    let mut dots = vec![0u8; area.width as usize * area.height as usize];
    let w = area.width as f32 * 2.;
    let h = area.height as f32 * 4.;
    let bits = [[0, 3], [1, 4], [2, 5], [6, 7]];
    for i in 0..720 {
        let theta = i as f32 / 720. * std::f32::consts::TAU + time * 0.45;
        let radius = 0.78 + 0.04 * (theta * 8. - time * 2.).sin();
        let x = ((w - 1.) * (0.5 + 0.48 * radius * theta.cos())) as usize;
        let y = ((h - 1.) * (0.5 + 0.48 * radius * theta.sin())) as usize;
        if x < w as usize && y < h as usize {
            dots[y / 4 * area.width as usize + x / 2] |= 1 << bits[y % 4][x % 2];
        }
    }
    for y in 0..area.height {
        for x in 0..area.width {
            let dot = dots[y as usize * area.width as usize + x as usize];
            if dot != 0 {
                buffer[(area.x + x, area.y + y)]
                    .set_char(char::from_u32(0x2800 + dot as u32).unwrap_or(' '))
                    .set_style(Style::default().fg(theme().accent));
            }
        }
    }
    let label = "∞ Planner123";
    buffer.set_stringn(
        area.x + area.width.saturating_sub(12) / 2,
        area.y + area.height / 2,
        label,
        area.width as usize,
        theme().accent_style(),
    );
}
