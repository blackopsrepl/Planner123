use super::*;

pub(super) fn require_task(conn: &Connection, id: &str) -> Result<PlanningTask, PlannerError> {
    get_task(conn, id)?.ok_or_else(|| PlannerError::NotFound {
        resource: "task",
        id: id.into(),
    })
}
pub(super) fn normalize_timezone(value: &str) -> Result<String, PlannerError> {
    time::normalize_timezone(value).map_err(|_| {
        PlannerError::Validation(
            "planner timezone must be an IANA name such as Europe/Rome or UTC".into(),
        )
    })
}

pub(super) fn canonical_settings_snapshot(
    settings: &PlannerSettings,
) -> Result<String, PlannerError> {
    let availability: Availability =
        serde_json::from_str(&settings.availability_json).map_err(|error| {
            PlannerError::Validation(format!("invalid planner availability: {error}"))
        })?;
    let mut value = serde_json::to_value(settings).map_err(internal)?;
    value["availability_json"] = serde_json::to_value(availability).map_err(internal)?;
    serde_json::to_string(&value).map_err(internal)
}
pub(super) fn normalize_clock(value: &str) -> Result<String, PlannerError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map(|v| v.format("%H:%M").to_string())
        .map_err(|_| PlannerError::Validation(format!("invalid time '{}'; expected HH:MM", value)))
}
pub(super) fn parse_clock(value: &str) -> Result<NaiveTime, PlannerError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| PlannerError::Validation(format!("invalid time '{}'; expected HH:MM", value)))
}
pub fn validate_availability(value: &Availability) -> Result<(), PlannerError> {
    if value.0.is_empty() {
        return Err(PlannerError::Validation(
            "configure at least one weekly availability window".into(),
        ));
    }
    for (day, windows) in &value.0 {
        if weekday_index(day).is_none() {
            return Err(PlannerError::Validation(format!(
                "invalid weekday '{}'",
                day
            )));
        }
        if windows.is_empty() {
            return Err(PlannerError::Validation(format!(
                "weekday '{}' has no windows",
                day
            )));
        }
        for window in windows {
            let start = parse_clock(&window.start)?;
            let end = parse_clock(&window.end)?;
            if end <= start {
                return Err(PlannerError::Validation(format!(
                    "availability on '{}' must end after it starts",
                    day
                )));
            }
        }
    }
    Ok(())
}
pub(super) fn validate_settings(value: &PlannerSettings) -> Result<(), PlannerError> {
    if !(1..=90).contains(&value.horizon_days) {
        return Err(PlannerError::Validation(
            "horizon_days must be between 1 and 90".into(),
        ));
    }
    if !(5..=120).contains(&value.slot_minutes) {
        return Err(PlannerError::Validation(
            "slot_minutes must be between 5 and 120".into(),
        ));
    }
    if !(1..=120).contains(&value.solve_seconds) {
        return Err(PlannerError::Validation(
            "solve_seconds must be between 1 and 120".into(),
        ));
    }
    if [
        value.low_outside_penalty,
        value.medium_outside_penalty,
        value.high_outside_penalty,
        value.excess_high_penalty,
    ]
    .iter()
    .any(|v| *v < 0)
    {
        return Err(PlannerError::Validation(
            "planner weights cannot be negative".into(),
        ));
    }
    if value.priority_low_weight < 1
        || value.priority_low_weight > value.priority_normal_weight
        || value.priority_normal_weight > value.priority_high_weight
    {
        return Err(PlannerError::Validation(
            "priority weights must be positive and ordered low <= normal <= high".into(),
        ));
    }
    if value.high_streak_limit < 1 || value.recovery_minutes < 0 {
        return Err(PlannerError::Validation(
            "high streak limit must be positive and recovery minutes cannot be negative".into(),
        ));
    }
    if value.cognitive_enabled {
        for clock in [
            &value.low_window_start,
            &value.low_window_end,
            &value.medium_window_start,
            &value.medium_window_end,
            &value.high_window_start,
            &value.high_window_end,
        ] {
            parse_clock(clock)?;
        }
    }
    Ok(())
}
