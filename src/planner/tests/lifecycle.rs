use super::*;

#[test]
fn proposal_becomes_stale_when_settings_change() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);
    create_task(&conn, task(calendar_id, "Settings-sensitive task")).unwrap();
    let proposal = optimize(&conn, Some(2)).unwrap();

    update_settings(
        &conn,
        SettingsUpdate {
            slot_minutes: Some(30),
            ..Default::default()
        },
    )
    .unwrap();

    let applicability = proposal_applicability(&conn, &proposal.proposal.id).unwrap();
    assert!(!applicability.can_apply);
    assert!(applicability
        .reasons
        .iter()
        .any(|reason| reason == "planner_settings_changed"));
}

#[test]
fn horizon_override_proposal_remains_ready_and_can_be_applied() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);
    create_task(&conn, task(calendar_id, "Short-horizon task")).unwrap();

    let proposal = optimize(&conn, Some(2)).unwrap();
    assert_eq!(proposal.proposal.horizon_days, 2);
    let applicability = proposal_applicability(&conn, &proposal.proposal.id).unwrap();
    assert!(applicability.can_apply, "{:?}", applicability.reasons);
    apply_proposal(&conn, &proposal.proposal.id).unwrap();
}

#[test]
fn proposal_becomes_stale_when_dependencies_change() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);
    let first = create_task(&conn, task(calendar_id.clone(), "First")).unwrap();
    let second = create_task(&conn, task(calendar_id, "Second")).unwrap();
    let proposal = optimize(&conn, Some(2)).unwrap();

    add_dependency(&conn, &first.id, &second.id).unwrap();

    let applicability = proposal_applicability(&conn, &proposal.proposal.id).unwrap();
    assert!(!applicability.can_apply);
    assert!(applicability
        .reasons
        .iter()
        .any(|reason| reason == "planner_dependencies_changed"));
}

#[test]
fn planner_rejects_invalid_recurrence_instead_of_ignoring_busy_time() {
    let (_temp, conn, calendar_id) = connection();
    let mut event = Event::new(
        calendar_id,
        "Broken recurrence",
        "2026-03-16 09:00:00",
        "2026-03-16 10:00:00",
        "UTC",
    );
    event.rrule = Some("FREQ=NOT_A_FREQUENCY".into());
    event_service::save_event(&conn, event, true).unwrap();

    let error = busy_occurrences(
        &conn,
        time::resolve_utc_datetime("2026-03-01 00:00:00", "UTC").unwrap(),
        time::resolve_utc_datetime("2026-04-01 00:00:00", "UTC").unwrap(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("invalid recurrence rule"));
}

#[test]
fn task_dependencies_reject_cycles() {
    let (_temp, conn, calendar_id) = connection();
    let first = create_task(&conn, task(calendar_id.clone(), "First")).unwrap();
    let second = create_task(&conn, task(calendar_id, "Second")).unwrap();
    add_dependency(&conn, &first.id, &second.id).unwrap();
    assert!(add_dependency(&conn, &second.id, &first.id).is_err());
}

#[test]
fn optimize_conflicts_when_an_applied_predecessor_loses_its_event() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);
    let predecessor = create_task(&conn, task(calendar_id.clone(), "Applied predecessor")).unwrap();
    let first_proposal = optimize(&conn, None).unwrap();
    apply_proposal(&conn, &first_proposal.proposal.id).unwrap();

    let successor = create_task(&conn, task(calendar_id, "Successor")).unwrap();
    add_dependency(&conn, &predecessor.id, &successor.id).unwrap();
    let event_id: String = conn
        .query_row(
            "SELECT event_id FROM planning_task_events WHERE task_id=?1",
            [&predecessor.id],
            |row| row.get(0),
        )
        .unwrap();
    event_service::delete_event(&conn, &event_id).unwrap();

    let error = optimize(&conn, None).unwrap_err();
    assert!(matches!(error, PlannerError::Conflict(_)));
    assert!(error.to_string().contains("no active event"));
}

#[test]
fn deleted_linked_event_moves_task_to_missing_event_and_can_return_to_inbox() {
    let (_temp, conn, calendar_id) = connection();
    let created = create_task(&conn, task(calendar_id.clone(), "Recover me")).unwrap();
    let event = event_service::save_event(
        &conn,
        Event::new(
            calendar_id,
            "Recover me",
            "2026-09-01 09:00:00",
            "2026-09-01 10:00:00",
            "UTC",
        ),
        true,
    )
    .unwrap();
    conn.execute(
            "INSERT INTO planner_proposals (id,status,horizon_start,horizon_days,timezone,snapshot_json,created_at) VALUES ('proposal','applied','2026-09-01 00:00:00',1,'UTC','{}','2026-09-01 00:00:00')",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT INTO planning_task_events (task_id,event_id,proposal_id) VALUES (?1,?2,'proposal')",
        params![created.id, event.id],
    )
    .unwrap();
    conn.execute(
        "UPDATE planning_tasks SET state='applied' WHERE id=?1",
        [&created.id],
    )
    .unwrap();

    event_service::delete_event(&conn, &event.id).unwrap();
    assert_eq!(
        require_task(&conn, &created.id).unwrap().state,
        PlanningTaskState::MissingEvent
    );

    assert_eq!(
        return_to_inbox(&conn, &created.id).unwrap().state,
        PlanningTaskState::Inbox
    );
    assert!(
        conn.query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM planning_task_events WHERE task_id=?1",
            [&created.id],
            |row| row.get(0),
        )
        .unwrap()
            == 0
    );
}
