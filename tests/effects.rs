mod common;
use planner123::{
    effects::{engine_toggle, Effects, Preferences, NATIVE},
    keys::{Action, View},
    ui,
};
use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    Terminal,
};
use std::time::{Duration, Instant};

fn isolated() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    std::env::set_var("XDG_DATA_HOME", dir.path());
    std::env::set_var(
        "PLANNER123_TEST_KEYRING_SERVICE",
        format!("planner-effects-test-{}", std::process::id()),
    );
    dir
}

#[test]
fn every_native_and_engine_toggle_defaults_on_and_persists() {
    let _lock = common::serial_guard();
    let _dir = isolated();
    let mut pref = Preferences::load();
    assert!(pref.enabled);
    let toggles: Vec<String> = NATIVE
        .iter()
        .map(|n| (*n).to_owned())
        .chain(
            planner123::glyphfx::EFFECTS
                .iter()
                .map(|e| engine_toggle(e)),
        )
        .collect();
    for name in &toggles {
        assert!(pref.on(name), "{name}");
        pref.individual.insert(name.clone(), false);
    }
    pref.save().unwrap();
    let loaded = Preferences::load();
    for name in &toggles {
        assert!(!loaded.on(name), "{name}");
    }
    assert!(loaded.enabled);
    pref.enabled = false;
    pref.save().unwrap();
    assert!(!Preferences::load().enabled);
}

#[test]
fn ambient_layers_keep_glyphs_and_fills_and_tint_evenly() {
    let _lock = common::serial_guard();
    let _dir = isolated();
    let mut fx = Effects::default();
    fx.started = Instant::now() - Duration::from_secs(4);
    fx.reveal = None;
    for view in [View::Month, View::Week, View::Day, View::Agenda] {
        let area = Rect::new(3, 2, 90, 30);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 40));
        buffer.set_string(
            7,
            7,
            "日本語 👩‍💻 café",
            ratatui::style::Style::default().fg(Color::White),
        );
        buffer[(8, 8)]
            .set_symbol(" ")
            .set_bg(Color::Blue)
            .set_style(ratatui::style::Style::default().add_modifier(Modifier::BOLD));
        buffer[(3, 2)].set_symbol("┌");
        let before = buffer.clone();
        planner123::effects::layers::composite(&fx, &view, &mut buffer, area);
        for y in 0..40 {
            for x in 0..100 {
                let (c, now) = (&before[(x, y)], &buffer[(x, y)]);
                // A semantic fill is never touched; a glyph keeps symbol and style
                // while a tint may sit under it.
                if c.bg == Color::Blue {
                    assert_eq!(now, c, "fill {view:?} at {x},{y}");
                } else if c.symbol() != " "
                    || (x > 0
                        && unicode_width::UnicodeWidthStr::width(before[(x - 1, y)].symbol()) > 1)
                {
                    let kept = (now.symbol(), now.fg, now.modifier);
                    assert_eq!(kept, (c.symbol(), c.fg, c.modifier), "{view:?} at {x},{y}");
                }
            }
        }
        if matches!(view, View::Week | View::Day) {
            // Even tint: a glyph shares the background of the blank beside it.
            assert_eq!(buffer[(7, 7)].bg, buffer[(6, 7)].bg, "{view:?} cut-out");
        }
        fx.preferences.enabled = false;
        let mut disabled = before.clone();
        planner123::effects::layers::composite(&fx, &view, &mut disabled, area);
        assert_eq!(before, disabled);
        fx.preferences.enabled = true;
    }
}

#[test]
fn scheduler_stops_hidden_disabled_and_finished_layers() {
    let _lock = common::serial_guard();
    let _dir = isolated();
    let mut fx = Effects::default();
    fx.reveal = None;
    assert!(fx.active(&View::Agenda));
    assert!(fx.active(&View::Month));
    assert!(!fx.active(&View::Help));
    for name in NATIVE {
        fx.preferences.individual.insert((*name).into(), false);
    }
    assert!(!fx.active(&View::Month));
    assert!(!fx.active(&View::Agenda));
    fx.preferences.individual.insert("view reveal".into(), true);
    fx.reveal = Some(Instant::now() - Duration::from_secs(1));
    assert!(!fx.active(&View::Month));
    fx.reveal = Some(Instant::now());
    assert!(fx.active(&View::Month));
    fx.preferences.enabled = false;
    assert!(!fx.active(&View::Month));
}

#[test]
fn actual_palette_registry_feedback_resize_and_credits_controls() {
    let _lock = common::serial_guard();
    let dir = isolated();
    let (_rt, mut app) = common::seeded_app(&dir, &[]);
    app.dispatch(Action::Palette);
    let matches = app.palette_matches();
    assert_eq!(
        matches
            .iter()
            .filter(|c| matches!(c.action, Action::ToggleEffect(_)))
            .count(),
        1 + NATIVE.len() + planner123::glyphfx::EFFECTS.len()
    );
    app.dispatch(Action::PlannerInbox);
    app.dispatch(Action::Palette);
    app.palette_input = "Effects: all".into();
    app.dispatch(Action::PaletteRun);
    assert_eq!(
        app.view,
        View::PlannerInbox,
        "effects toggles must not force a calendar view"
    );
    assert!(!app.effects.preferences.enabled);
    app.dispatch(Action::ToggleEffect("all".into()));
    app.dispatch(Action::ViewAgenda);
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('j'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(planner123::keys::rows(planner123::keys::Context::Agenda)
        .iter()
        .filter_map(|r| r.bar)
        .any(|b| app.effects.pressed(&View::Agenda, b.key, b.label)));
    for (w, h) in [(1, 1), (24, 8), (60, 18), (120, 40), (200, 60)] {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        for view in [
            View::Agenda,
            View::Month,
            View::Week,
            View::Day,
            View::PlannerInbox,
            View::Help,
        ] {
            app.view = view;
            terminal.draw(|frame| ui::render(&mut app, frame)).unwrap();
        }
    }
    app.dispatch(Action::Credits);
    assert!(app.effects.credits);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| ui::render(&mut app, f)).unwrap();
    assert!(app.animation_active());
    assert_eq!(planner123::glyphfx::EFFECTS, ::glyphfx::EFFECTS);
    for index in 0..planner123::glyphfx::EFFECTS.len() {
        if index > 0 {
            app.handle_key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Right,
                crossterm::event::KeyModifiers::NONE,
            ));
        }
        terminal.draw(|f| ui::render(&mut app, f)).unwrap();
        std::thread::sleep(Duration::from_millis(35));
        terminal.draw(|f| ui::render(&mut app, f)).unwrap();
        assert!(
            app.effects.diagnostic.is_none(),
            "gallery effect {index}: {:?}",
            app.effects.diagnostic
        );
    }
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(!app.effects.credits);
    app.dispatch(Action::ToggleEffect("all".into()));
    assert!(!app.animation_active());
    assert!(!Preferences::load().enabled);
}

#[test]
fn easing_settles_at_exact_requested_row_and_resize_cancels() {
    let _lock = common::serial_guard();
    let _dir = isolated();
    let mut fx = Effects::default();
    fx.scroll = (2., 12, Instant::now() - Duration::from_millis(90));
    assert!((3..=12).contains(&fx.scroll_position(12)));
    fx.scroll.2 = Instant::now() - Duration::from_millis(200);
    assert_eq!(fx.scroll_position(12), 12);
    fx.prepare(&View::Month, Rect::new(0, 0, 80, 24));
    fx.prepare(&View::Month, Rect::new(0, 0, 120, 40));
    assert!(fx.reveal.is_none());
}
