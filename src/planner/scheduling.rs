use super::*;

/// Local midnight `days` after `start`, converted to UTC. Keeps DST days exact.
pub(super) fn local_horizon_end(
    start: DateTime<Tz>,
    days: i64,
) -> Result<DateTime<Utc>, PlannerError> {
    let days =
        u64::try_from(days).map_err(|_| PlannerError::Validation("invalid horizon".into()))?;
    let end_date = start
        .date_naive()
        .checked_add_days(Days::new(days))
        .ok_or_else(|| PlannerError::Validation("invalid horizon".into()))?;
    start
        .timezone()
        .from_local_datetime(&end_date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .map(|end| end.with_timezone(&Utc))
        .ok_or_else(|| {
            PlannerError::Validation("planner horizon ends at an invalid local time".into())
        })
}

/// Builds the uniform candidate start grid: one `SolverSlot` per `minutes`.
///
/// The grid is intentionally not trimmed by availability or busy time; those
/// rules are scored as constraints.
pub(super) fn make_slots(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    minutes: i64,
) -> Result<Vec<SolverSlot>, PlannerError> {
    if minutes <= 0 || end <= start {
        return Err(PlannerError::Validation("invalid horizon".into()));
    }
    let mut slots = Vec::new();
    let mut slot_start = start;
    let mut index = 0usize;
    while slot_start < end {
        slots.push(SolverSlot {
            id: index,
            start: slot_start,
        });
        slot_start += Duration::minutes(minutes);
        index += 1;
    }
    Ok(slots)
}

/// Maps an availability weekday key to days from Monday.
pub(super) fn weekday_index(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(0),
        "tue" | "tuesday" => Some(1),
        "wed" | "wednesday" => Some(2),
        "thu" | "thursday" => Some(3),
        "fri" | "friday" => Some(4),
        "sat" | "saturday" => Some(5),
        "sun" | "sunday" => Some(6),
        _ => None,
    }
}

/// Half-open interval overlap used by recurrence expansion and tests.
pub(super) fn overlaps(
    a: DateTime<Utc>,
    b: DateTime<Utc>,
    c: DateTime<Utc>,
    d: DateTime<Utc>,
) -> bool {
    a < d && c < b
}
