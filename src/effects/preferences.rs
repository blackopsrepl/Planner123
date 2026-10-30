use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

pub const NATIVE: &[&str] = &[
    "agenda stars",
    "agenda easing",
    "edge fades",
    "pressed chips",
    "view reveal",
    "proposal reveal",
    "apply sweep",
    "landing ripples",
    "copper",
    "plasma",
    "task flights",
    "sine scroller",
    "ouroboros",
];

/// The preference key that toggles a glyphfx effect. glyphfx's `copper` shares
/// its name with the native copper bars, so an engine name that collides with a
/// native one gets a `glyphfx ` prefix and the two toggle independently.
pub fn engine_toggle(effect: &str) -> String {
    if NATIVE.contains(&effect) {
        format!("glyphfx {effect}")
    } else {
        effect.to_owned()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub enabled: bool,
    pub individual: BTreeMap<String, bool>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            enabled: true,
            individual: BTreeMap::new(),
        }
    }
}
impl Preferences {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("planner123/effects.json")
    }
    pub fn load() -> Self {
        std::fs::read(Self::path())
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default()
    }
    pub fn on(&self, name: &str) -> bool {
        self.enabled && self.individual.get(name).copied().unwrap_or(true)
    }
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        let parent = path.parent().ok_or("Invalid effects configuration path")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let staging = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&staging, data).map_err(|e| e.to_string())?;
        std::fs::rename(staging, path).map_err(|e| e.to_string())
    }
}
