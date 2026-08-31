use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};

/* The resolved color palette used across all UI modules. */
#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub background: Color,
    pub foreground: Color,
    pub selection_fg: Color,
    pub selection_bg: Color,
    pub color0: Color,
    pub color1: Color,
    pub color3: Color,
    pub color4: Color,
    pub color5: Color,
    pub color6: Color,
    pub color7: Color,
    pub color8: Color,
    pub color9: Color,
}

/* Calendar color palette: 8 distinct calendar colors cycling through the theme. */
/* Index 0-7 map to color1..color7 + accent. */
pub const CALENDAR_COLORS: usize = 8;

// ── Singleton ────────────────────────────────────────────────────────
static THEME: OnceLock<Theme> = OnceLock::new();

/* Returns the global theme, loading it once on first access. */
pub fn theme() -> &'static Theme {
    THEME.get_or_init(|| load_theme().unwrap_or_else(|_| fallback_theme()))
}

// ── Loading ──────────────────────────────────────────────────────────

mod calendar;
mod loading;
mod styles;

use loading::{fallback_theme, load_theme};
