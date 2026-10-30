use crate::keys::{Action, View};
use ratatui::{buffer::Buffer, layout::Rect};
use std::time::{Duration, Instant};
pub mod brand;
pub mod credits;
pub mod layers;
mod preferences;
pub use preferences::{engine_toggle, Preferences, NATIVE};

pub struct Effects {
    pub preferences: Preferences,
    pub credits: bool,
    pub started: Instant,
    pub reveal: Option<Instant>,
    pub proposal: Option<Instant>,
    pub applied: Option<Instant>,
    pub pending_ids: Vec<String>,
    pub task_labels: std::collections::HashMap<String, (String, String)>,
    pub task_events: std::collections::HashMap<String, String>,
    pub applied_ids: Vec<String>,
    pub chip: Option<(Action, Instant)>,
    pub scroll: (f32, u16, Instant),
    pub gallery_index: usize,
    pub diagnostic: Option<String>,
    pub(crate) clip: Option<crate::glyphfx::Clip>,
    pub(crate) clip_frame: Option<Buffer>,
    pub(crate) clip_size: Rect,
    pub(crate) clip_step: Instant,
    pub(crate) gallery_started: Instant,
    last_view: Option<View>,
    size: Rect,
}
impl Default for Effects {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            preferences: Preferences::load(),
            credits: false,
            started: now,
            reveal: Some(now),
            proposal: None,
            applied: None,
            pending_ids: vec![],
            task_labels: Default::default(),
            task_events: Default::default(),
            applied_ids: vec![],
            chip: None,
            scroll: (0., 0, now),
            gallery_index: 0,
            diagnostic: None,
            clip: None,
            clip_frame: None,
            clip_size: Rect::default(),
            clip_step: now,
            gallery_started: now,
            last_view: None,
            size: Rect::default(),
        }
    }
}
impl Effects {
    pub fn on(&self, name: &str) -> bool {
        self.preferences.on(name)
    }
    /// Whether the glyphfx effect named `effect` is enabled.
    pub fn engine_on(&self, effect: &str) -> bool {
        self.on(&engine_toggle(effect))
    }
    pub fn age(mark: Option<Instant>) -> f32 {
        mark.map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(f32::INFINITY)
    }
    pub fn prepare(&mut self, view: &View, size: Rect) {
        if self.size != Rect::default() && self.size != size {
            self.cancel();
        } else if self.last_view.as_ref().is_some_and(|v| v != view) {
            // A second navigation during a wipe settles, rather than restarting it.
            self.reveal = if Self::age(self.reveal) < 0.32 {
                None
            } else {
                Some(Instant::now())
            };
            self.clip = None;
            self.clip_frame = None;
        }
        self.size = size;
        self.last_view = Some(view.clone());
    }
    pub fn cancel(&mut self) {
        self.reveal = None;
        self.proposal = None;
        self.applied = None;
        self.chip = None;
        self.clip = None;
        self.clip_frame = None;
        self.scroll.0 = self.scroll.1 as f32;
    }
    pub fn active(&self, view: &View) -> bool {
        if !self.preferences.enabled {
            return false;
        }
        if self.credits {
            return self.on("sine scroller")
                || self.on("ouroboros")
                || crate::glyphfx::EFFECTS.iter().any(|e| self.engine_on(e));
        }
        let calendar = matches!(view, View::Month | View::Week | View::Day | View::Agenda);
        (matches!(view, View::Agenda)
            && (self.on("agenda stars")
                || (self.on("agenda easing") && self.scroll.2.elapsed().as_secs_f32() < 0.18)))
            || (matches!(view, View::Month) && self.on("plasma"))
            || (matches!(view, View::Week | View::Day) && self.on("copper"))
            || (calendar && self.on("view reveal") && Self::age(self.reveal) < 0.32)
            || (self.on("pressed chips")
                && self
                    .chip
                    .as_ref()
                    .is_some_and(|(_, t)| t.elapsed().as_secs_f32() < 0.18))
            || (matches!(view, View::PlannerInbox)
                && ((self.on("proposal reveal") && Self::age(self.proposal) < 0.5)
                    || (self.on("task flights") && Self::age(self.proposal) < 1.0)))
            || ((calendar || matches!(view, View::PlannerInbox))
                && (self.on("apply sweep") || self.on("landing ripples"))
                && Self::age(self.applied) < 1.1)
    }
    pub fn scroll_position(&self, target: u16) -> u16 {
        if !self.on("agenda easing") || target != self.scroll.1 {
            return target;
        }
        let p = (self.scroll.2.elapsed().as_secs_f32() / 0.18).min(1.);
        (self.scroll.0 + (target as f32 - self.scroll.0) * (1. - (1. - p).powi(3))).round() as u16
    }
    pub fn scroll_to(&mut self, previous: u16, target: u16) {
        let from = self.scroll_position(previous) as f32;
        self.scroll = (from, target, Instant::now());
    }
    pub fn pressed(&self, view: &View, key: &str, desc: &str) -> bool {
        self.on("pressed chips")
            && self.chip.as_ref().is_some_and(|(action, at)| {
                at.elapsed() < Duration::from_millis(180)
                    && crate::keys::action_hint(crate::keys::Context::of(view), action)
                        .is_some_and(|(k, d)| k == key && d == desc)
            })
    }
}
