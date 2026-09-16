use super::*;

#[test]
fn inbox_task_keeps_cognitive_load_and_deadline_contract() {
    let (_temp, conn, calendar_id) = connection();
    let mut input = task(calendar_id, "Deep work");
    input.cognitive_load = CognitiveLoad::High;
    input.deadline_kind = DeadlineKind::Hard;
    input.deadline_at = Some("2026-09-01 12:00:00".into());
    let created = create_task(&conn, input).unwrap();
    assert_eq!(created.cognitive_load, CognitiveLoad::High);
    assert_eq!(created.state, PlanningTaskState::Inbox);
    assert_eq!(list_tasks(&conn).unwrap().len(), 1);
}

#[test]
fn applied_tasks_are_not_returned_by_the_inbox_query() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);
    let created = create_task(&conn, task(calendar_id, "Schedule once")).unwrap();
    let proposal = optimize(&conn, None).unwrap();

    apply_proposal(&conn, &proposal.proposal.id).unwrap();

    assert!(list_inbox_tasks(&conn).unwrap().is_empty());
    assert_eq!(
        get_task(&conn, &created.id).unwrap().unwrap().state,
        PlanningTaskState::Applied
    );
    assert_eq!(list_tasks(&conn).unwrap().len(), 1);
}

#[test]
fn local_day_horizons_include_every_dst_instant_and_no_more() {
    let timezone = Tz::from_str("Europe/Rome").unwrap();
    for (date, hours) in [("2026-03-29", 23), ("2026-10-25", 25)] {
        let start = timezone
            .from_local_datetime(
                &chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            )
            .single()
            .unwrap();
        let end = local_horizon_end(start, 1).unwrap();
        let slots = make_slots(start.with_timezone(&Utc), end, 60).unwrap();

        assert_eq!(slots.len(), hours);
        assert_eq!(
            (slots.last().unwrap().start + Duration::hours(1)).with_timezone(&timezone),
            end.with_timezone(&timezone)
        );
        assert_eq!(end.with_timezone(&timezone).hour(), 0);
    }
}

#[test]
fn planner_busy_time_expands_recurrences_through_the_horizon() {
    let (_temp, conn, calendar_id) = connection();
    let mut weekly = Event::new(
        calendar_id.clone(),
        "Weekly planning",
        "2026-03-16 09:00:00",
        "2026-03-16 10:00:00",
        "Europe/Rome",
    );
    weekly.rrule = Some("FREQ=WEEKLY;COUNT=4".into());
    event_service::save_event(&conn, weekly, true).unwrap();

    let mut monthly = Event::new(
        calendar_id,
        "Monthly review",
        "2026-01-15 14:00:00",
        "2026-01-15 15:00:00",
        "UTC",
    );
    monthly.rrule = Some("FREQ=MONTHLY;COUNT=4".into());
    event_service::save_event(&conn, monthly, true).unwrap();

    let horizon_start = time::resolve_utc_datetime("2026-03-01 00:00:00", "UTC").unwrap();
    let horizon_end = time::resolve_utc_datetime("2026-05-01 00:00:00", "UTC").unwrap();
    let starts = busy_occurrences(&conn, horizon_start, horizon_end)
        .unwrap()
        .into_iter()
        .map(|occurrence| occurrence.start.format("%Y-%m-%d %H:%M").to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        starts,
        vec![
            "2026-03-15 14:00",
            "2026-04-15 14:00",
            "2026-03-16 08:00",
            "2026-03-23 08:00",
            "2026-03-30 07:00",
            "2026-04-06 07:00",
        ]
    );
}

#[test]
fn optimizer_does_not_schedule_over_a_future_recurring_occurrence() {
    let (_temp, conn, calendar_id) = connection();
    let occurrence_date = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(1))
        .unwrap();
    let master_date = occurrence_date.checked_sub_days(Days::new(7)).unwrap();
    let weekday = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
        [occurrence_date.weekday().num_days_from_monday() as usize];
    update_settings(
        &conn,
        SettingsUpdate {
            timezone: Some("UTC".into()),
            availability: Some(Availability(BTreeMap::from([(
                weekday.into(),
                vec![TimeWindow {
                    start: "09:00".into(),
                    end: "10:00".into(),
                }],
            )]))),
            ..Default::default()
        },
    )
    .unwrap();

    let mut event = Event::new(
        calendar_id.clone(),
        "Recurring conflict",
        format!("{master_date} 09:00:00"),
        format!("{master_date} 10:00:00"),
        "UTC",
    );
    event.rrule = Some("FREQ=WEEKLY;COUNT=2".into());
    event_service::save_event(&conn, event, true).unwrap();
    create_task(&conn, task(calendar_id.clone(), "Must not conflict")).unwrap();

    let proposal = optimize(&conn, Some(2)).unwrap();
    assert!(!proposal.items[0].scheduled);
    assert_eq!(
        proposal.items[0].diagnostics.outcome,
        PlannerProposalOutcome::NoHardFeasibleSlot
    );
    assert!(proposal.items[0].explanation.is_some());
    let blocker = &proposal.items[0].diagnostics.busy_blockers[0];
    assert!(blocker.recurring);
    assert_eq!(blocker.event_title, "Recurring conflict");
    assert_eq!(blocker.calendar_id, calendar_id);
}

#[test]
fn unselected_but_individually_feasible_tasks_report_feasible_but_not_selected() {
    let (_temp, conn, calendar_id) = connection();
    let target = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(1))
        .unwrap();
    let weekday = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
        [target.weekday().num_days_from_monday() as usize];
    update_settings(
        &conn,
        SettingsUpdate {
            timezone: Some("UTC".into()),
            availability: Some(Availability(BTreeMap::from([(
                weekday.into(),
                vec![TimeWindow {
                    start: "09:00".into(),
                    end: "10:00".into(),
                }],
            )]))),
            ..Default::default()
        },
    )
    .unwrap();
    create_task(&conn, task(calendar_id.clone(), "Winner")).unwrap();
    create_task(&conn, task(calendar_id, "Runner up")).unwrap();

    let proposal = optimize(&conn, Some(2)).unwrap();
    let outcomes: Vec<_> = proposal
        .items
        .iter()
        .map(|item| item.diagnostics.outcome.clone())
        .collect();
    assert!(outcomes.contains(&PlannerProposalOutcome::Scheduled));
    assert!(outcomes.contains(&PlannerProposalOutcome::FeasibleButNotSelected));
    let loser = proposal
        .items
        .iter()
        .find(|item| item.diagnostics.outcome == PlannerProposalOutcome::FeasibleButNotSelected)
        .unwrap();
    assert!(!loser.scheduled);
    assert!(loser.diagnostics.busy_blockers.is_empty());
}

#[test]
fn blocker_evidence_is_bounded_and_reports_the_exact_omitted_count() {
    let (_temp, conn, calendar_id) = connection();
    let target = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(1))
        .unwrap();
    let weekday = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
        [target.weekday().num_days_from_monday() as usize];
    update_settings(
        &conn,
        SettingsUpdate {
            timezone: Some("UTC".into()),
            availability: Some(Availability(BTreeMap::from([(
                weekday.into(),
                vec![TimeWindow {
                    start: "09:00".into(),
                    end: "13:00".into(),
                }],
            )]))),
            ..Default::default()
        },
    )
    .unwrap();

    // Six 40-minute events exactly tile the four-hour availability window, so
    // no placement escapes them and all six become blocker evidence.
    for index in 0..6 {
        let start_minute = 540 + index * 40;
        let end_minute = start_minute + 40;
        event_service::save_event(
            &conn,
            Event::new(
                calendar_id.clone(),
                format!("Block {index}"),
                format!(
                    "{target} {:02}:{:02}:00",
                    start_minute / 60,
                    start_minute % 60
                ),
                format!("{target} {:02}:{:02}:00", end_minute / 60, end_minute % 60),
                "UTC",
            ),
            true,
        )
        .unwrap();
    }
    create_task(&conn, task(calendar_id, "Fully blocked")).unwrap();

    let proposal = optimize(&conn, Some(2)).unwrap();
    let diagnostics = &proposal.items[0].diagnostics;
    assert_eq!(
        diagnostics.outcome,
        PlannerProposalOutcome::NoHardFeasibleSlot
    );
    assert_eq!(diagnostics.busy_blockers.len(), 5);
    assert_eq!(diagnostics.busy_blockers_omitted, 1);
    let starts: Vec<_> = diagnostics
        .busy_blockers
        .iter()
        .map(|blocker| blocker.start_at.clone())
        .collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(
        starts, sorted,
        "blockers must be ordered by occurrence start"
    );
}
