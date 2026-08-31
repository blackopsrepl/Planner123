use super::*;

pub fn settings(conn: &Connection) -> Result<PlannerSettings, PlannerError> {
    conn.query_row(
        "SELECT timezone, availability_json, horizon_days, slot_minutes, solve_seconds,
                priority_low_weight, priority_normal_weight, priority_high_weight,
                cognitive_enabled, low_window_start, low_window_end, low_outside_penalty,
                medium_window_start, medium_window_end, medium_outside_penalty,
                high_window_start, high_window_end, high_outside_penalty,
                high_streak_limit, recovery_minutes, excess_high_penalty
         FROM planner_settings WHERE id = 1",
        [],
        settings_from_row,
    )
    .map_err(internal)
}

/* Render planner settings without leaking SQLite's JSON storage representation. */
pub fn settings_json(conn: &Connection) -> Result<serde_json::Value, PlannerError> {
    let settings = settings(conn)?;
    let availability: Availability =
        serde_json::from_str(&settings.availability_json).map_err(|error| {
            PlannerError::Validation(format!("invalid planner availability: {error}"))
        })?;
    let mut value = serde_json::to_value(settings).map_err(internal)?;
    let object = value.as_object_mut().ok_or_else(|| {
        PlannerError::Internal("planner settings did not serialize as an object".into())
    })?;
    object.remove("availability_json");
    object.insert(
        "availability".into(),
        serde_json::to_value(availability).map_err(internal)?,
    );
    Ok(value)
}

pub fn update_settings(
    conn: &Connection,
    update: SettingsUpdate,
) -> Result<PlannerSettings, PlannerError> {
    let mut current = settings(conn)?;
    if let Some(value) = update.timezone {
        current.timezone = Some(normalize_timezone(&value)?);
    }
    if let Some(value) = update.availability {
        validate_availability(&value)?;
        current.availability_json = serde_json::to_string(&value).map_err(internal)?;
    }
    macro_rules! replace {
        ($field:ident) => {
            if let Some(value) = update.$field {
                current.$field = value;
            }
        };
    }
    replace!(horizon_days);
    replace!(slot_minutes);
    replace!(solve_seconds);
    replace!(priority_low_weight);
    replace!(priority_normal_weight);
    replace!(priority_high_weight);
    replace!(cognitive_enabled);
    replace!(low_outside_penalty);
    replace!(medium_outside_penalty);
    replace!(high_outside_penalty);
    replace!(high_streak_limit);
    replace!(recovery_minutes);
    replace!(excess_high_penalty);
    if let Some(value) = update.low_window_start {
        current.low_window_start = normalize_clock(&value)?;
    }
    if let Some(value) = update.low_window_end {
        current.low_window_end = normalize_clock(&value)?;
    }
    if let Some(value) = update.medium_window_start {
        current.medium_window_start = normalize_clock(&value)?;
    }
    if let Some(value) = update.medium_window_end {
        current.medium_window_end = normalize_clock(&value)?;
    }
    if let Some(value) = update.high_window_start {
        current.high_window_start = normalize_clock(&value)?;
    }
    if let Some(value) = update.high_window_end {
        current.high_window_end = normalize_clock(&value)?;
    }
    validate_settings(&current)?;
    conn.execute(
        "UPDATE planner_settings SET timezone=?1, availability_json=?2, horizon_days=?3, slot_minutes=?4,
             solve_seconds=?5, priority_low_weight=?6, priority_normal_weight=?7, priority_high_weight=?8,
             cognitive_enabled=?9, low_window_start=?10, low_window_end=?11, low_outside_penalty=?12,
             medium_window_start=?13, medium_window_end=?14, medium_outside_penalty=?15,
             high_window_start=?16, high_window_end=?17, high_outside_penalty=?18,
             high_streak_limit=?19, recovery_minutes=?20, excess_high_penalty=?21,
             updated_at=strftime('%Y-%m-%d %H:%M:%S', 'now') WHERE id=1",
        params![current.timezone, current.availability_json, current.horizon_days, current.slot_minutes,
            current.solve_seconds, current.priority_low_weight, current.priority_normal_weight,
            current.priority_high_weight, current.cognitive_enabled as i64, current.low_window_start,
            current.low_window_end, current.low_outside_penalty, current.medium_window_start,
            current.medium_window_end, current.medium_outside_penalty, current.high_window_start,
            current.high_window_end, current.high_outside_penalty, current.high_streak_limit,
            current.recovery_minutes, current.excess_high_penalty],
    ).map_err(internal)?;
    settings(conn)
}
