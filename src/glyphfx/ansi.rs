use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use vte::{Params, Parser, Perform};

pub(super) fn decode(frame: &str, width: u16, height: u16) -> Buffer {
    let mut screen = Screen {
        buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        x: 0,
        y: 0,
        previous: None,
        style: Style::default(),
    };
    Parser::new().advance(&mut screen, frame.as_bytes());
    screen.buffer
}

struct Screen {
    buffer: Buffer,
    x: u16,
    y: u16,
    previous: Option<(u16, u16)>,
    style: Style,
}

impl Perform for Screen {
    fn print(&mut self, ch: char) {
        // Join combining marks, variation selectors, flags and ZWJ sequences,
        // even when an SGR boundary splits a grapheme in the engine output.
        if let Some(pos) = self.previous {
            let symbol = format!("{}{ch}", self.buffer[pos].symbol());
            if symbol.graphemes(true).count() == 1 {
                let width = UnicodeWidthStr::width(symbol.as_str()) as u16;
                self.x = pos.0.saturating_add(width);
                if u32::from(pos.0) + u32::from(width) <= u32::from(self.buffer.area.width) {
                    self.buffer[pos].set_symbol(&symbol);
                    let style = self.buffer[pos].style();
                    for x in pos.0 + 1..self.x {
                        self.buffer[(x, pos.1)].set_symbol(" ").set_style(style);
                    }
                } else {
                    self.buffer[pos].reset();
                    self.previous = None;
                }
                return;
            }
        }
        let width = ch.width().unwrap_or(0) as u16;
        if width == 0 {
            return;
        }
        let area = self.buffer.area;
        if self.y < area.height && u32::from(self.x) + u32::from(width) <= u32::from(area.width) {
            let pos = (self.x, self.y);
            self.buffer[pos]
                .set_symbol(&ch.to_string())
                .set_style(self.style);
            self.previous = Some(pos);
            for x in self.x + 1..self.x + width {
                self.buffer[(x, self.y)]
                    .set_symbol(" ")
                    .set_style(self.style);
            }
        } else {
            self.previous = None;
        }
        // Never wrap a long engine row into the following row.
        self.x = self.x.saturating_add(width);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.x = 0;
                self.y = self.y.saturating_add(1);
            }
            b'\r' => self.x = 0,
            b'\t' => self.x = (self.x / 8 + 1).saturating_mul(8),
            _ => {}
        }
        self.previous = None;
    }

    fn csi_dispatch(&mut self, params: &Params, _: &[u8], ignore: bool, action: char) {
        if ignore || action != 'm' {
            return;
        }
        let mut values = Vec::new();
        for p in params.iter() {
            // ISO colon RGB permits an optional (empty/zero) color-space id.
            if p.len() == 6 && matches!(p[0], 38 | 48) && p[1] == 2 {
                values.extend_from_slice(&p[..2]);
                values.extend_from_slice(&p[3..]);
            } else {
                values.extend_from_slice(p);
            }
        }
        let mut i = 0;
        while i < values.len() {
            let n = values[i];
            let modifier = match n {
                1 => Some(Modifier::BOLD),
                2 => Some(Modifier::DIM),
                3 => Some(Modifier::ITALIC),
                4 | 21 => Some(Modifier::UNDERLINED),
                5 => Some(Modifier::SLOW_BLINK),
                6 => Some(Modifier::RAPID_BLINK),
                7 => Some(Modifier::REVERSED),
                8 => Some(Modifier::HIDDEN),
                9 => Some(Modifier::CROSSED_OUT),
                _ => None,
            };
            if let Some(m) = modifier {
                self.style = self.style.add_modifier(m);
            }
            match n {
                0 => self.style = Style::default(),
                22 => self.style = self.style.remove_modifier(Modifier::BOLD | Modifier::DIM),
                23 => self.style = self.style.remove_modifier(Modifier::ITALIC),
                24 => self.style = self.style.remove_modifier(Modifier::UNDERLINED),
                25 => {
                    self.style = self
                        .style
                        .remove_modifier(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK)
                }
                27 => self.style = self.style.remove_modifier(Modifier::REVERSED),
                28 => self.style = self.style.remove_modifier(Modifier::HIDDEN),
                29 => self.style = self.style.remove_modifier(Modifier::CROSSED_OUT),
                30..=37 => self.style.fg = Some(basic(n - 30)),
                40..=47 => self.style.bg = Some(basic(n - 40)),
                90..=97 => self.style.fg = Some(basic(n - 90 + 8)),
                100..=107 => self.style.bg = Some(basic(n - 100 + 8)),
                39 => self.style.fg = Some(Color::Reset),
                49 => self.style.bg = Some(Color::Reset),
                38 | 48 => {
                    let tail = &values[i + 1..];
                    let parsed = match tail {
                        [5, v, ..] if *v <= 255 => Some((Color::Indexed(*v as u8), 2)),
                        [2, r, g, b, ..] if *r <= 255 && *g <= 255 && *b <= 255 => {
                            Some((Color::Rgb(*r as u8, *g as u8, *b as u8), 4))
                        }
                        _ => None,
                    };
                    if let Some((color, consumed)) = parsed {
                        if n == 38 {
                            self.style.fg = Some(color);
                        } else {
                            self.style.bg = Some(color);
                        }
                        i += consumed;
                    } else {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

fn basic(index: u16) -> Color {
    [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ][index as usize]
}
