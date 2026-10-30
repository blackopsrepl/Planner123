// Exercises the adapter module on its own, outside the app.
#[path = "../src/glyphfx/mod.rs"]
mod glyphfx;
use glyphfx::{Clip, EFFECTS};
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

fn frames(effect: &str) -> Vec<Buffer> {
    let mut clip = Clip::new(effect, "GLYPH FX\nPlanner", 16, 6).unwrap();
    let mut frames = Vec::new();
    while let Some(frame) = clip.next_frame() {
        assert!(frames.len() < 20_000, "{effect}");
        assert_eq!(frame.area, Rect::new(0, 0, 16, 6));
        assert_eq!(frame.content.len(), 96);
        frames.push(frame);
    }
    assert!(clip.is_finished());
    assert!(clip.next_frame().is_none());
    assert!(!frames.is_empty());
    assert!(frames
        .iter()
        .flat_map(|b| &b.content)
        .any(|c| c.symbol() != " "));
    assert!(frames
        .iter()
        .flat_map(|b| &b.content)
        .any(|c| c.fg != Color::Reset));
    frames
}

#[test]
fn all37_real_styled_buffers_complete_and_interleave() {
    assert_eq!(EFFECTS.len(), 37);
    for &effect in EFFECTS {
        let expected = frames(effect);
        let mut actual = Clip::new(effect, "GLYPH FX\nPlanner", 16, 6).unwrap();
        let mut other = Clip::new("matrix", "OTHER", 20, 8).unwrap();
        for frame in expected {
            assert_eq!(actual.next_frame(), Some(frame), "{effect}");
            other.next_frame();
        }
        assert!(actual.next_frame().is_none());
    }
}

#[test]
fn real_unicode_frames_and_errors() {
    let mut clip = Clip::new("wipe", "界é🙂", 12, 4).unwrap();
    let mut last = None;
    while let Some(frame) = clip.next_frame() {
        last = Some(frame);
    }
    let frame = last.unwrap();
    assert_eq!(frame[(0, 0)].symbol(), "界");
    assert_eq!(frame[(1, 0)].symbol(), " ");
    assert_eq!(frame[(2, 0)].symbol(), "é");
    assert_eq!(frame[(3, 0)].symbol(), "🙂");
    let mut clip = Clip::centered("wipe", "MID", 21, 9).unwrap();
    let mut last = None;
    while let Some(frame) = clip.next_frame() {
        last = Some(frame);
    }
    let frame = last.unwrap();
    let at = (0..9u16)
        .flat_map(|y| (0..21u16).map(move |x| (x, y)))
        .find(|&p| frame[p].symbol() == "M")
        .unwrap();
    assert!(
        (8..=10).contains(&at.0) && (3..=5).contains(&at.1),
        "{at:?}"
    );
    assert!(Clip::new("bad", "X", 12, 4).is_err());
    assert!(Clip::new("wipe", "X\0Y", 12, 4).is_err());
    assert!(Clip::new("wipe", "X", 0, 4).is_err());
    assert!(Clip::new("wipe", "X", u16::MAX, u16::MAX).is_err());
}
