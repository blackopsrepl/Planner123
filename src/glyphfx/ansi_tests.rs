use super::ansi::decode;
use ratatui::style::{Color, Modifier};

#[test]
fn foreground_background_all_modifiers_and_resets() {
    let b = decode(
        "\x1b[1;2;3;4;5;6;7;8;9;38;2;12;34;56;48;5;123mA\x1b[0mB",
        5,
        2,
    );
    let a = &b[(0, 0)];
    assert_eq!(a.fg, Color::Rgb(12, 34, 56));
    assert_eq!(a.bg, Color::Indexed(123));
    assert_eq!(a.modifier, Modifier::all());
    assert_eq!(b[(1, 0)].fg, Color::Reset);
    assert_eq!(b[(1, 0)].bg, Color::Reset);
    assert_eq!(b[(1, 0)].modifier, Modifier::empty());
    let b = decode("\x1b[91;104;1;3mX\x1b[22;23;39;49mY", 5, 1);
    assert_eq!(b[(0, 0)].fg, Color::LightRed);
    assert_eq!(b[(0, 0)].bg, Color::LightBlue);
    assert_eq!(b[(1, 0)].modifier, Modifier::empty());
    assert_eq!(b[(1, 0)].fg, Color::Reset);
    assert_eq!(b[(1, 0)].bg, Color::Reset);
    let colon = decode("\x1b[38:2::11:22:33;48:5:42mX", 1, 1);
    assert_eq!(colon[(0, 0)].fg, Color::Rgb(11, 22, 33));
    assert_eq!(colon[(0, 0)].bg, Color::Indexed(42));
}

#[test]
fn unicode_combining_width_clipping_rows_and_no_stale_cells() {
    let b = decode("界e\u{301}🙂Z\nnext", 6, 3);
    assert_eq!(b[(0, 0)].symbol(), "界");
    assert_eq!(b[(1, 0)].symbol(), " ");
    assert_eq!(b[(2, 0)].symbol(), "e\u{301}");
    assert_eq!(b[(3, 0)].symbol(), "🙂");
    assert_eq!(b[(5, 0)].symbol(), "Z");
    assert_eq!(b[(0, 1)].symbol(), "n");
    assert_eq!(b[(0, 2)].symbol(), " ");
    let clipped = decode("ABCDE界\nX\nnot visible", 6, 2);
    assert_eq!(clipped[(5, 0)].symbol(), " ");
    assert_eq!(clipped[(0, 1)].symbol(), "X");
    let joined = decode("👩\x1b[0m\u{200d}💻🇮🇹X", 6, 1);
    assert_eq!(joined[(0, 0)].symbol(), "👩\u{200d}💻");
    assert_eq!(joined[(2, 0)].symbol(), "🇮🇹");
    assert_eq!(joined[(4, 0)].symbol(), "X");
    let empty = decode("", 6, 2);
    assert!(empty
        .content
        .iter()
        .all(|c| c.symbol() == " " && c.fg == Color::Reset));
}

#[test]
fn malformed_and_non_sgr_escape_sequences_do_not_leak() {
    let maximum = decode(&"X".repeat(65_536), u16::MAX, 1);
    assert_eq!(maximum[(u16::MAX - 1, 0)].symbol(), "X");
    let b = decode("\x1b[38;2;999mX\x1b[0m\x1b[2JY\x1b]0;title\x07Z", 8, 1);
    assert_eq!(b[(0, 0)].symbol(), "X");
    assert_eq!(b[(1, 0)].symbol(), "Y");
    assert_eq!(b[(2, 0)].symbol(), "Z");
    assert_eq!(b[(0, 0)].fg, Color::Reset);
}
