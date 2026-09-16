#[test]
fn planner_task_cli_captures_cognitive_load_without_touching_events() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);
    let created = cli_command(&temp)
        .args([
            "tasks",
            "create",
            "--title",
            "Deep work",
            "--duration-minutes",
            "90",
            "--target-calendar-id",
            &calendar_id,
            "--priority",
            "high",
            "--cognitive-load",
            "high",
        ])
        .output()
        .unwrap();
    assert!(created.status.success());
    let value = read_json(&created.stdout);
    assert_eq!(value["data"]["cognitive_load"], "high");
    assert_eq!(value["data"]["state"], "inbox");

    let events = cli_command(&temp)
        .args(["events", "list"])
        .output()
        .unwrap();
    assert!(events.status.success());
    assert!(read_json(&events.stdout)["data"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn planner_optimize_reports_typed_outcomes_and_busy_blocker_evidence() {
    let temp = TempDir::new().unwrap();
    let calendar_id = first_calendar_id(&temp);
    let tomorrow = chrono::Utc::now().date_naive().succ_opt().unwrap();
    let tomorrow_date = tomorrow.format("%Y-%m-%d").to_string();
    let weekday = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
        [chrono::Datelike::weekday(&tomorrow).num_days_from_monday() as usize];

    let configured = cli_command(&temp)
        .args([
            "planner",
            "settings",
            "update",
            "--timezone",
            "UTC",
            "--availability",
            &format!("{weekday}=09:00-10:00"),
            "--horizon-days",
            "2",
            "--solve-seconds",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stderr)
    );

    let event = cli_command(&temp)
        .args([
            "events",
            "create",
            "--calendar-id",
            &calendar_id,
            "--title",
            "Blocked window",
            "--start-at",
            &format!("{tomorrow_date} 09:00:00"),
            "--end-at",
            &format!("{tomorrow_date} 10:00:00"),
            "--timezone",
            "UTC",
            "--rrule",
            "FREQ=DAILY;COUNT=2",
        ])
        .output()
        .unwrap();
    assert!(
        event.status.success(),
        "{}",
        String::from_utf8_lossy(&event.stderr)
    );

    let task = cli_command(&temp)
        .args([
            "tasks",
            "create",
            "--title",
            "Blocked task",
            "--duration-minutes",
            "60",
            "--target-calendar-id",
            &calendar_id,
        ])
        .output()
        .unwrap();
    assert!(task.status.success());

    let optimized = cli_command(&temp)
        .args(["planner", "optimize"])
        .output()
        .unwrap();
    assert!(
        optimized.status.success(),
        "{}",
        String::from_utf8_lossy(&optimized.stderr)
    );
    let proposal_id = read_json(&optimized.stdout)["data"]["proposal"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let detail = cli_command(&temp)
        .args(["planner", "proposals", "get", &proposal_id])
        .output()
        .unwrap();
    assert!(detail.status.success());
    let item = &read_json(&detail.stdout)["data"]["items"][0];
    assert_eq!(item["scheduled"], false);
    assert_eq!(item["diagnostics"]["outcome"], "no_hard_feasible_slot");
    assert_eq!(
        item["diagnostics"]["busy_blockers"][0]["event_title"],
        "Blocked window"
    );
    assert_eq!(item["diagnostics"]["busy_blockers"][0]["recurring"], true);
    assert_eq!(item["diagnostics"]["busy_blockers_omitted"], 0);
}
