//! Safe, local-buffer adapter for glyphfx. The engine owns all native code.
#![forbid(unsafe_code)]
mod ansi;
#[cfg(test)]
mod ansi_tests;

pub use ::glyphfx::EFFECTS;
use ratatui::buffer::Buffer;

pub struct Clip {
    engine: ::glyphfx::Clip,
    width: u16,
    height: u16,
}

impl Clip {
    /// Text anchored top-left, as the engine lays it out by default.
    pub fn new(effect: &str, text: &str, width: u16, height: u16) -> Result<Self, String> {
        Self::open(effect, text, width, height, ::glyphfx::Anchor::NorthWest)
    }

    /// Text settles in the middle of the canvas.
    pub fn centered(effect: &str, text: &str, width: u16, height: u16) -> Result<Self, String> {
        Self::open(effect, text, width, height, ::glyphfx::Anchor::Center)
    }

    fn open(
        effect: &str,
        text: &str,
        width: u16,
        height: u16,
        anchor: ::glyphfx::Anchor,
    ) -> Result<Self, String> {
        Ok(Self {
            engine: ::glyphfx::Clip::with_options(effect, text, width, height, 1, anchor)?,
            width,
            height,
        })
    }

    pub fn next_frame(&mut self) -> Option<Buffer> {
        self.engine
            .next_frame()
            .map(|frame| ansi::decode(&frame, self.width, self.height))
    }

    pub fn is_finished(&self) -> bool {
        self.engine.is_finished()
    }
}
