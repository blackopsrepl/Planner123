use super::*;

#[test]
fn successor_stays_unassigned_when_its_predecessor_has_no_hard_feasible_slot() {
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
                    end: "11:00".into(),
                }],
            )]))),
            ..Default::default()
        },
    )
    .unwrap();

    let mut predecessor = task(calendar_id.clone(), "Impossible predecessor");
    predecessor.deadline_kind = DeadlineKind::Hard;
    predecessor.deadline_at = Some(format!("{target} 08:30:00"));
    let predecessor = create_task(&conn, predecessor).unwrap();
    let successor = create_task(&conn, task(calendar_id.clone(), "Dependent successor")).unwrap();
    add_dependency(&conn, &predecessor.id, &successor.id).unwrap();
    let transitive = create_task(
        &conn,
        task(calendar_id.clone(), "Transitively dependent task"),
    )
    .unwrap();
    add_dependency(&conn, &successor.id, &transitive.id).unwrap();

    let proposal = optimize(&conn, Some(2)).unwrap();
    assert!(proposal.items.iter().all(|item| !item.scheduled));
    let successor = proposal
        .items
        .iter()
        .find(|item| item.task_id == successor.id)
        .unwrap();
    assert_eq!(
        successor.diagnostics.outcome,
        PlannerProposalOutcome::NoHardFeasibleSlot
    );
    let transitive = proposal
        .items
        .iter()
        .find(|item| item.task_id == transitive.id)
        .unwrap();
    assert_eq!(
        transitive.diagnostics.outcome,
        PlannerProposalOutcome::NoHardFeasibleSlot
    );
}
