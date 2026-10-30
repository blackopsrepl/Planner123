mod common;
use planner123::{
    app::App,
    keys::Action,
    models::{CognitiveLoad, DeadlineKind, TaskPriority},
    planner::{self, Availability, CreateTaskInput, SettingsUpdate, TimeWindow},
    ui,
};
use ratatui::{backend::TestBackend, Terminal};
use std::{collections::BTreeMap, time::Duration};
use tokio::runtime::Runtime;

fn prepare() -> (tempfile::TempDir, Runtime, App) {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    std::env::set_var("XDG_DATA_HOME", dir.path());
    std::env::set_var(
        "PLANNER123_TEST_KEYRING_SERVICE",
        format!("effects-apply-{}", std::process::id()),
    );
    let (rt, mut app) = common::seeded_app(&dir, &[]);
    let conn = planner123::db::open().unwrap();
    planner::update_settings(
        &conn,
        SettingsUpdate {
            timezone: Some("UTC".into()),
            solve_seconds: Some(1),
            availability: Some(Availability(BTreeMap::from_iter(
                ["mon", "tue", "wed", "thu", "fri", "sat", "sun"].map(|d| {
                    (
                        d.into(),
                        vec![TimeWindow {
                            start: "00:00".into(),
                            end: "23:59".into(),
                        }],
                    )
                }),
            ))),
            ..Default::default()
        },
    )
    .unwrap();
    planner::create_task(
        &conn,
        CreateTaskInput {
            title: "Unicode café 日本".into(),
            duration_minutes: 30,
            target_calendar_id: app.calendars[0].id.clone(),
            project_id: None,
            priority: TaskPriority::Normal,
            cognitive_load: CognitiveLoad::Low,
            earliest_at: Some(format!(
                "{} 09:00:00",
                chrono::Utc::now().date_naive() + chrono::Duration::days(1)
            )),
            deadline_kind: DeadlineKind::None,
            deadline_at: None,
        },
    )
    .unwrap();
    app.dispatch(Action::PlannerInbox);
    assert!(common::pump_until(
        &mut app,
        &rt,
        Duration::from_secs(10),
        |a| a.planner_settings.is_some() && a.planner_tasks.len() == 1
    ));
    app.dispatch(Action::PlannerOptimize);
    assert!(common::pump_until(
        &mut app,
        &rt,
        Duration::from_secs(20),
        |a| a.planner_proposal.is_some()
    ));
    assert!(app.effects.proposal.is_some());
    assert!(app.effects.applied.is_none());
    assert_eq!(
        app.planner_proposal
            .as_ref()
            .unwrap()
            .items
            .iter()
            .filter(|i| i.scheduled)
            .count(),
        1
    );
    (dir, rt, app)
}
fn screen(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(220, 50)).unwrap();
    terminal.draw(|f| ui::render(app, f)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        let mut x = 0;
        while x < buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            text.push_str(symbol);
            x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
        }
        text.push('\n');
    }
    text
}

#[test]
fn real_proposal_ghosts_only_become_solid_after_commit_and_id_reload() {
    let _guard = common::serial_guard();
    let (_dir, rt, mut app) = prepare();
    let ghost = screen(&mut app);
    assert!(ghost.contains("GHOST / ready"));
    assert!(ghost.contains("┊Unicode café 日本"));
    assert!(app.events.iter().all(|e| e.title != "Unicode café 日本"));
    app.dispatch(Action::PlannerApply);
    assert!(
        app.effects.applied.is_none(),
        "dispatch is not a completed apply"
    );
    assert!(common::pump_until(
        &mut app,
        &rt,
        Duration::from_secs(10),
        |a| a.effects.applied.is_some()
    ));
    assert_eq!(app.effects.applied_ids.len(), 1);
    assert!(app.effects.pending_ids.is_empty());
    assert!(app
        .events
        .iter()
        .any(|e| app.effects.applied_ids.contains(&e.id)));
    assert!(common::pump_until(
        &mut app,
        &rt,
        Duration::from_secs(5),
        |a| a.planner_tasks.is_empty()
    ));
    let solid = screen(&mut app);
    assert!(solid.contains("APPLIED"));
    assert!(
        solid.contains("▌Unicode café 日本"),
        "task label survives leaving inbox"
    );
    let applied = app.effects.applied;
    app.dispatch(Action::ViewWeek);
    app.dispatch(Action::ViewMonth);
    common::pump_until(&mut app, &rt, Duration::from_secs(5), |a| !a.loading);
    assert_eq!(
        app.effects.applied, applied,
        "ordinary refresh must not replay success"
    );
    app.effects.preferences.enabled = false;
    let disabled = screen(&mut app);
    assert!(disabled.contains("Planner123"));
}

#[test]
fn rejected_stale_proposal_never_celebrates_or_leaves_ghosts() {
    let _guard = common::serial_guard();
    let (_dir, rt, mut app) = prepare();
    let conn = planner123::db::open().unwrap();
    planner::update_task(
        &conn,
        &app.planner_tasks[0].id,
        planner::UpdateTaskInput {
            title: Some("Changed after solving".into()),
            ..Default::default()
        },
    )
    .unwrap();
    app.dispatch(Action::PlannerApply);
    assert!(common::pump_until(
        &mut app,
        &rt,
        Duration::from_secs(10),
        |a| a.status_is_error && !a.loading
    ));
    assert!(app.effects.applied.is_none());
    assert!(app.effects.applied_ids.is_empty());
    assert!(app.effects.pending_ids.is_empty());
    assert!(app.planner_proposal.is_none());
}
