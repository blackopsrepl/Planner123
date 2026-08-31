use super::*;

pub(super) fn colors_toml_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".local/share/solverforge/default/theme/colors.toml");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub(super) fn load_theme() -> anyhow::Result<Theme> {
    let path = colors_toml_path().ok_or_else(|| anyhow::anyhow!("colors.toml not found"))?;
    let content = std::fs::read_to_string(&path)?;
    parse_colors_toml(&content)
}

pub fn parse_colors_toml(content: &str) -> anyhow::Result<Theme> {
    let table: HashMap<String, String> = toml::from_str(content)?;
    let get = |key: &str| -> anyhow::Result<Color> {
        let hex = table
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("missing color: {key}"))?;
        parse_hex_color(hex)
    };
    Ok(Theme {
        accent: get("accent")?,
        background: get("background")?,
        foreground: get("foreground")?,
        selection_fg: get("selection_foreground")?,
        selection_bg: get("selection_background")?,
        color0: get("color0")?,
        color1: get("color1")?,
        color3: get("color3")?,
        color4: get("color4")?,
        color5: get("color5")?,
        color6: get("color6")?,
        color7: get("color7")?,
        color8: get("color8")?,
        color9: get("color9")?,
    })
}

pub fn parse_hex_color(hex: &str) -> anyhow::Result<Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        anyhow::bail!("invalid hex color length: {hex}");
    }
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok(Color::Rgb(r, g, b))
}

pub fn fallback_theme() -> Theme {
    Theme {
        accent: Color::Rgb(130, 251, 156),
        background: Color::Rgb(11, 12, 22),
        foreground: Color::Rgb(221, 247, 255),
        selection_fg: Color::Rgb(11, 12, 22),
        selection_bg: Color::Rgb(221, 247, 255),
        color0: Color::Rgb(11, 12, 22),
        color1: Color::Rgb(80, 248, 114),
        color3: Color::Rgb(80, 247, 212),
        color4: Color::Rgb(130, 157, 212),
        color5: Color::Rgb(134, 167, 223),
        color6: Color::Rgb(124, 248, 247),
        color7: Color::Rgb(133, 225, 251),
        color8: Color::Rgb(106, 110, 149),
        color9: Color::Rgb(133, 255, 157),
    }
}
