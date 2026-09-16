use super::*;

fn parse(value: &str) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap()
}

#[test]
fn optimize_produces_a_hard_feasible_future_schedule() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);

    let tomorrow = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(1))
        .unwrap();
    event_service::save_event(
        &conn,
        Event::new(
            calendar_id.clone(),
            "Existing block",
            format!("{tomorrow} 10:00:00"),
            format!("{tomorrow} 11:00:00"),
            "UTC",
        ),
        true,
    )
    .unwrap();
    create_task(&conn, task(calendar_id.clone(), "Focus A")).unwrap();
    create_task(&conn, task(calendar_id, "Focus B")).unwrap();

    let proposal = optimize(&conn, Some(3)).unwrap();
    assert_eq!(proposal.items.len(), 2);
    assert!(
        proposal.items.iter().all(|item| item.scheduled),
        "both tasks should be schedulable inside availability"
    );

    let now = Utc::now().naive_utc();
    let existing_start = parse(&format!("{tomorrow} 10:00:00"));
    let existing_end = parse(&format!("{tomorrow} 11:00:00"));

    let mut intervals = Vec::new();
    for item in &proposal.items {
        let start = parse(item.start_at.as_ref().unwrap());
        let end = parse(item.end_at.as_ref().unwrap());

        assert!(start > now, "assignment must not be in the past: {item:?}");
        assert!(start.time() >= chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert!(end.time() <= chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        assert!(start.date() == end.date(), "task must fit inside one day");
        assert!(
            !(start < existing_end && existing_start < end),
            "assignment must not overlap the existing event: {item:?}"
        );
        intervals.push((start, end));
    }

    intervals.sort();
    for pair in intervals.windows(2) {
        assert!(
            pair[0].1 <= pair[1].0,
            "assignments must not overlap each other"
        );
    }
}

#[test]
fn soft_deadline_ontime_placement_wins_over_the_preferred_window() {
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
                    start: "10:00".into(),
                    end: "12:00".into(),
                }],
            )]))),
            cognitive_enabled: Some(true),
            low_window_start: Some("11:00".into()),
            low_window_end: Some("12:00".into()),
            low_outside_penalty: Some(10),
            ..Default::default()
        },
    )
    .unwrap();

    let mut input = task(calendar_id, "Deadline first");
    input.duration_minutes = 30;
    input.cognitive_load = CognitiveLoad::Low;
    input.deadline_kind = DeadlineKind::Soft;
    input.deadline_at = Some(format!("{target} 10:30:00"));
    create_task(&conn, input).unwrap();

    let proposal = optimize(&conn, None).unwrap();
    // 10:00 finishes on time but sits outside the 11:00-12:00 preference
    // window; 11:00 fits the window yet ends an hour late. Lateness is a
    // medium score and must win over the soft preference.
    assert_eq!(
        proposal.items[0].start_at.as_deref(),
        Some(format!("{target} 10:00:00").as_str())
    );
    assert_eq!(
        proposal.items[0].diagnostics.outcome,
        PlannerProposalOutcome::Scheduled
    );
}

#[test]
fn item_penalties_reconcile_additively_with_the_recovery_score() {
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
                    end: "12:00".into(),
                }],
            )]))),
            recovery_minutes: Some(150),
            excess_high_penalty: Some(7),
            ..Default::default()
        },
    )
    .unwrap();

    for title in ["H1", "H2", "H3"] {
        let mut input = task(calendar_id.clone(), title);
        input.duration_minutes = 60;
        input.cognitive_load = CognitiveLoad::High;
        create_task(&conn, input).unwrap();
    }

    // The three-hour window forces the tasks back to back, so the last task
    // violates against two predecessors and the score carries three pair
    // penalties: 7 for H2, 14 for H3.
    let proposal = optimize(&conn, Some(2)).unwrap();
    assert_eq!(
        proposal.proposal.score.as_deref(),
        Some("0hard/0medium/-21soft")
    );
    let fatigue_total: i64 = proposal.items.iter().map(|item| item.fatigue_penalty).sum();
    assert_eq!(fatigue_total, 21);
}

#[test]
fn availability_far_out_in_the_horizon_is_still_reachable() {
    use chrono::Datelike;

    let (_temp, conn, calendar_id) = connection();
    let target = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(3))
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
                    end: "17:00".into(),
                }],
            )]))),
            ..Default::default()
        },
    )
    .unwrap();
    create_task(&conn, task(calendar_id, "Far focus")).unwrap();

    let proposal = optimize(&conn, Some(14)).unwrap();
    assert!(
        proposal.items[0].scheduled,
        "a single weekly window inside the horizon must stay schedulable"
    );
}
