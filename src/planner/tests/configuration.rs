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
