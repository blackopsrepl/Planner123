use super::*;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn gallery_never_discards_an_unfinished_effect_on_a_wall_clock_limit() {
    let mut fx = Effects {
        preferences: crate::effects::Preferences::default(),
        credits: true,
        ..Default::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| render(&mut fx, frame, frame.area()))
        .unwrap();
    assert!(!fx.clip.as_ref().unwrap().is_finished());
    let index = fx.gallery_index;
    fx.gallery_started = Instant::now() - Duration::from_secs(9);
    terminal
        .draw(|frame| render(&mut fx, frame, frame.area()))
        .unwrap();
    assert_eq!(fx.gallery_index, index);
    assert!(fx.clip.is_some());
}

#[test]
fn gallery_text_settles_in_the_middle_of_its_canvas() {
    let area = Rect::new(0, 0, 80, 20);
    let mut clip = glyphfx::Clip::centered("wipe", GALLERY_TEXT, area.width, area.height).unwrap();
    let mut last = None;
    while let Some(frame) = clip.next_frame() {
        last = Some(frame);
    }
    let frame = last.unwrap();
    // The engine centres the block; its widest line sets the left edge.
    let widest = "Safe native terminal effects";
    let row = (0..area.height)
        .find(|&y| {
            (0..area.width)
                .map(|x| frame[(x, y)].symbol())
                .collect::<String>()
                .contains(widest)
        })
        .expect("block drawn");
    let line: String = (0..area.width).map(|x| frame[(x, row)].symbol()).collect();
    let column = line.find(widest).unwrap() as u16;
    assert!((8..=12).contains(&row), "row {row}");
    assert!((25..=27).contains(&column), "column {column}");
}
