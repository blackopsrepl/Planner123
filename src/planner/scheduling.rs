use super::*;

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

pub(super) fn make_slots(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    minutes: i64,
) -> Result<Vec<Slot>, PlannerError> {
    if minutes <= 0 || end <= start {
        return Err(PlannerError::Validation("invalid horizon".into()));
    }
    let mut slots = Vec::new();
    let mut slot_start = start;
    while slot_start < end {
        slots.push(Slot { start: slot_start });
        slot_start += Duration::minutes(minutes);
    }
    Ok(slots)
}
pub(super) struct Slot {
    pub(super) start: DateTime<Utc>,
}
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
pub(super) fn available_interval(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    availability: &Availability,
    timezone: Tz,
) -> bool {
    let mut current = start;
    while current < end {
        let local = current.with_timezone(&timezone);
        let key = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
            [local.weekday().num_days_from_monday() as usize];
        let Some(windows) = availability
            .0
            .get(key)
            .or_else(|| availability.0.get(&key.to_uppercase()))
        else {
            return false;
        };
        if !windows
            .iter()
            .any(|window| clock_contains(local.time(), &window.start, &window.end))
        {
            return false;
        }
        current += Duration::minutes(1);
    }
    true
}
pub(super) fn clock_contains(value: NaiveTime, start: &str, end: &str) -> bool {
    let Ok(start) = NaiveTime::parse_from_str(start, "%H:%M") else {
        return false;
    };
    let Ok(end) = NaiveTime::parse_from_str(end, "%H:%M") else {
        return false;
    };
    if start == end {
        return true;
    }
    if start < end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}
pub(super) fn overlaps(
    a: DateTime<Utc>,
    b: DateTime<Utc>,
    c: DateTime<Utc>,
    d: DateTime<Utc>,
) -> bool {
    a < d && c < b
}
pub(super) fn cognitive_profile(
    settings: &PlannerSettings,
    load: &CognitiveLoad,
) -> (String, String, i64) {
    match load {
        CognitiveLoad::Low => (
            settings.low_window_start.clone(),
            settings.low_window_end.clone(),
            settings.low_outside_penalty,
        ),
        CognitiveLoad::Medium => (
            settings.medium_window_start.clone(),
            settings.medium_window_end.clone(),
            settings.medium_outside_penalty,
        ),
        CognitiveLoad::High => (
            settings.high_window_start.clone(),
            settings.high_window_end.clone(),
            settings.high_outside_penalty,
        ),
    }
}
pub(super) fn cognitive_cost(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    window_start: &str,
    window_end: &str,
    penalty: i64,
    timezone: Tz,
) -> i64 {
    let mut cursor = start;
    let mut cost = 0;
    while cursor < end {
        if !clock_contains(
            cursor.with_timezone(&timezone).time(),
            window_start,
            window_end,
        ) {
            cost += penalty;
        }
        cursor += Duration::minutes(1);
    }
    cost
}
pub(super) fn fatigue_penalty(
    task: &SolverTask,
    all: &[SolverTask],
    slots: &[Slot],
    settings: &PlannerSettings,
) -> i64 {
    if !task.high {
        return 0;
    }
    let Some(start_idx) = task.start_slot_idx else {
        return 0;
    };
    let Some(slot) = slots.get(start_idx) else {
        return 0;
    };
    let start = slot.start;
    let mut current_preceding = 0;
    for other in all {
        if other.id == task.id || !other.high {
            continue;
        }
        if let Some(end) = other.end() {
            let end = slots[0].start + Duration::minutes(end);
            if end <= start && (start - end).num_minutes() < settings.recovery_minutes {
                current_preceding += 1;
            }
        }
    }
    if task.external_fatigue_cost() > 0 || current_preceding >= settings.high_streak_limit as usize
    {
        settings.excess_high_penalty
    } else {
        0
    }
}

pub(super) fn applied_fatigue_cost(
    high: bool,
    start: DateTime<Utc>,
    applied: &[UtcInterval],
    settings: &PlannerSettings,
) -> i64 {
    if !high {
        return 0;
    }
    let preceding = applied
        .iter()
        .filter(|(_, end)| {
            *end <= start && (start - *end).num_minutes() < settings.recovery_minutes
        })
        .count();
    if preceding >= settings.high_streak_limit as usize {
        settings.excess_high_penalty
    } else {
        0
    }
}
