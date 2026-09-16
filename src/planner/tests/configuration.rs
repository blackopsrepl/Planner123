use super::*;

#[test]
fn settings_start_neutral_and_require_availability_for_a_solve() {
    let (_temp, conn, _) = connection();
    let settings = super::settings(&conn).unwrap();
    assert!(!settings.cognitive_enabled);
    assert_eq!(settings.horizon_days, 14);
    assert!(validate_availability(
        &serde_json::from_str::<Availability>(&settings.availability_json).unwrap()
    )
    .is_err());
}

#[test]
fn planner_timezone_requires_an_iana_name() {
    let (_temp, conn, _) = connection();
    let error = update_settings(
        &conn,
        SettingsUpdate {
            timezone: Some("Rome".into()),
            ..Default::default()
        },
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "planner timezone must be an IANA name such as Europe/Rome or UTC"
    );
}

#[test]
fn availability_requires_known_days_and_forward_time_windows() {
    let invalid_day = Availability(BTreeMap::from([(
        "mo".into(),
        vec![TimeWindow {
            start: "09:00".into(),
            end: "17:00".into(),
        }],
    )]));
    assert_eq!(
        validate_availability(&invalid_day).unwrap_err().to_string(),
        "invalid weekday 'mo'"
    );

    let backwards_window = Availability(BTreeMap::from([(
        "mon".into(),
        vec![TimeWindow {
            start: "17:00".into(),
            end: "09:00".into(),
        }],
    )]));
    assert_eq!(
        validate_availability(&backwards_window)
            .unwrap_err()
            .to_string(),
        "availability on 'mon' must end after it starts"
    );
}

#[test]
fn priority_weights_must_be_positive_and_ordered() {
    let (_temp, conn, _) = connection();
    for update in [
        SettingsUpdate {
            priority_low_weight: Some(0),
            ..Default::default()
        },
        SettingsUpdate {
            priority_low_weight: Some(10),
            priority_normal_weight: Some(5),
            ..Default::default()
        },
    ] {
        assert_eq!(
            update_settings(&conn, update).unwrap_err().to_string(),
            "priority weights must be positive and ordered low <= normal <= high"
        );
    }
}

#[test]
fn task_duration_is_bounded_to_a_supported_same_day_interval() {
    let (_temp, conn, calendar_id) = connection();
    let mut input = task(calendar_id, "Too long");
    input.duration_minutes = 1440;
    assert_eq!(
        create_task(&conn, input).unwrap_err().to_string(),
        "duration_minutes must be between 1 and 1439"
    );
}

#[test]
fn optimizer_rejects_soft_score_overflow_before_solving() {
    let (_temp, conn, calendar_id) = connection();
    configure_utc_workweek(&conn);
    update_settings(
        &conn,
        SettingsUpdate {
            cognitive_enabled: Some(true),
            high_outside_penalty: Some(i64::MAX),
            ..Default::default()
        },
    )
    .unwrap();
    let mut input = task(calendar_id, "Overflowing score");
    input.duration_minutes = 1439;
    input.cognitive_load = CognitiveLoad::High;
    create_task(&conn, input).unwrap();

    assert_eq!(
        optimize(&conn, None).unwrap_err().to_string(),
        "planner cognitive and recovery weights exceed the supported score range"
    );
}
