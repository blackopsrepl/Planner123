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
