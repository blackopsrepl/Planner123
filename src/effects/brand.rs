use crate::{effects::Effects, glyphfx, keys::View, theme::theme};
use ratatui::{layout::Rect, Frame};
use std::time::{Duration, Instant};

/// Text effects own only the brand, never titles, errors, dates or key hints.
/// On cancellation the canonical header has already been rendered this frame.
pub fn render(fx: &mut Effects, view: &View, frame: &mut Frame, header: Rect) {
    if fx.credits || header.width < 16 || header.height == 0 {
        return;
    }
    let effect = if matches!(view, View::PlannerInbox)
        && fx.on("proposal reveal")
        && Effects::age(fx.proposal) < 0.5
    {
        "print"
    } else if fx.on("apply sweep") && Effects::age(fx.applied) < 1.1 {
        "highlight"
    } else if fx.on("view reveal") && Effects::age(fx.reveal) < 0.32 {
        match view {
            View::Agenda => "slide",
            View::Week | View::Day => "middleout",
            _ => "wipe",
        }
    } else {
        fx.clip = None;
        fx.clip_frame = None;
        return;
    };
    if !fx.on(effect) {
        return;
    }
    let area = Rect::new(header.x + 5, header.y, 10, 1);
    if fx.clip_size != area {
        fx.clip = None;
        fx.clip_frame = None;
        fx.clip_size = area;
    }
    if fx.clip.is_none() && fx.clip_frame.is_none() {
        match glyphfx::Clip::new(effect, "Planner123", area.width, area.height) {
            Ok(clip) => fx.clip = Some(clip),
            Err(e) => {
                fx.diagnostic = Some(format!("glyphfx {effect}: {e}"));
                fx.clip_frame = Some(ratatui::buffer::Buffer::empty(Rect::new(
                    0,
                    0,
                    area.width,
                    area.height,
                )));
                return;
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
        // Keep header background; text animation may not repaint neighbouring chrome.
        for x in 0..area.width.min(buffer.area.width) {
            let src = &buffer[(buffer.area.x + x, buffer.area.y)];
            if src.symbol() != " " {
                frame.buffer_mut()[(area.x + x, area.y)]
                    .set_symbol(src.symbol())
                    .set_style(theme().header());
            }
        }
    }
}
