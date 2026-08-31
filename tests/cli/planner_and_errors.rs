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
