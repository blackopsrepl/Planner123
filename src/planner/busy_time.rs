use super::*;

/// One expanded occurrence of an existing calendar event.
#[derive(Debug)]
pub(super) struct BusyOccurrence {
    pub(super) start: DateTime<Utc>,
    pub(super) end: DateTime<Utc>,
    pub(super) event_id: String,
    pub(super) event_title: String,
    pub(super) calendar_id: String,
    pub(super) recurring: bool,
}

/// Expands every active event, including recurrences, into horizon occurrences.
pub(super) fn busy_intervals(
    conn: &Connection,
    horizon_start: DateTime<Utc>,
    horizon_end: DateTime<Utc>,
) -> Result<Vec<BusyOccurrence>, PlannerError> {
    db::load_events(conn)
        .map_err(internal)?
        .into_iter()
        .try_fold(Vec::new(), |mut intervals, event| {
            intervals.extend(event_busy_intervals(&event, horizon_start, horizon_end)?);
            Ok(intervals)
        })
}

/// Expands one event into its busy occurrences inside the horizon.
pub(super) fn event_busy_intervals(
    event: &Event,
    horizon_start: DateTime<Utc>,
    horizon_end: DateTime<Utc>,
) -> Result<Vec<BusyOccurrence>, PlannerError> {
    let (start, end) = match event.start_dt().zip(event.end_dt()) {
        Some(interval) => interval,
        None => return Ok(Vec::new()),
    };
    let duration = end - start;
    let starts = match event.rrule.as_deref() {
        Some(rule) => expand_recurrence(event, rule, start, duration, horizon_start, horizon_end)?,
        None => vec![start],
    };

    Ok(starts
        .into_iter()
        .map(|occurrence_start| BusyOccurrence {
            start: occurrence_start,
            end: occurrence_start + duration,
            event_id: event.id.clone(),
            event_title: event.title.clone(),
            calendar_id: event.calendar_id.clone(),
            recurring: event.rrule.is_some(),
        })
        .filter(|occurrence| overlaps(occurrence.start, occurrence.end, horizon_start, horizon_end))
        .collect())
}

fn expand_recurrence(
    event: &Event,
    rule: &str,
    start: DateTime<Utc>,
    duration: chrono::Duration,
    horizon_start: DateTime<Utc>,
    horizon_end: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, PlannerError> {
    let timezone = event.timezone_tz().ok_or_else(|| {
        PlannerError::Validation(format!(
            "event '{}' has an invalid recurrence timezone '{}'",
            event.title, event.timezone
        ))
    })?;
    let recurrence_start = start.with_timezone(&rrule::Tz::from(timezone));
    let rule = rule
        .parse::<rrule::RRule<rrule::Unvalidated>>()
        .map_err(|error| {
            PlannerError::Validation(format!(
                "event '{}' has an invalid recurrence rule: {error}",
                event.title
            ))
        })?;
    let result = rule
        .build(recurrence_start)
        .map_err(|error| {
            PlannerError::Validation(format!(
                "event '{}' has an invalid recurrence rule: {error}",
                event.title
            ))
        })?
        .after((horizon_start - duration).with_timezone(&rrule::Tz::from(timezone)))
        .before(horizon_end.with_timezone(&rrule::Tz::from(timezone)))
        .all(u16::MAX);
    if result.limited {
        return Err(PlannerError::Validation(format!(
            "event '{}' recurrence exceeds the {}-occurrence safety limit within the planning horizon",
            event.title,
            u16::MAX
        )));
    }
    Ok(result
        .dates
        .into_iter()
        .map(|occurrence| occurrence.with_timezone(&Utc))
        .collect())
}

/// Applied proposals as blocked intervals, tagged with high-load state and the
/// inbox successors that must start after them.
pub(super) fn applied_busy(
    conn: &Connection,
    tasks: &[PlanningTask],
    dependencies: &[(String, String)],
) -> Result<Vec<SolverBusy>, PlannerError> {
    let index_of: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect();
    let mut successors: HashMap<String, Vec<usize>> = HashMap::new();
    for (from, to) in dependencies {
        if let Some(&index) = index_of.get(to.as_str()) {
            successors.entry(from.clone()).or_default().push(index);
        }
    }

    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.cognitive_load, e.id, e.start_at, e.end_at, e.timezone, e.title, e.calendar_id
             FROM planning_tasks t
             JOIN planning_task_events l ON l.task_id = t.id
             JOIN events e ON e.id = l.event_id
             WHERE e.deleted_at IS NULL",
        )
        .map_err(internal)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;

    let mut busy = Vec::new();
    for (task_id, load, event_id, start_at, end_at, timezone, title, calendar_id) in rows {
        let (start, end) = match (
            time::resolve_utc_datetime(&start_at, &timezone),
            time::resolve_utc_datetime(&end_at, &timezone),
        ) {
            (Ok(start), Ok(end)) => (start, end),
            _ => continue,
        };
        busy.push(SolverBusy {
            id: format!("applied:{event_id}"),
            start,
            end,
            high: load == "high",
            successors: successors.get(&task_id).cloned().unwrap_or_default(),
            event_id,
            event_title: title,
            calendar_id,
            recurring: false,
        });
    }
    Ok(busy)
}

/// Maps each successor task id to the inbox indexes it must start after.
pub(super) fn dependency_map(
    conn: &Connection,
    tasks: &[PlanningTask],
) -> Result<HashMap<String, Vec<usize>>, PlannerError> {
    let lookup: HashMap<_, _> = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.clone(), index))
        .collect();
    let mut out = HashMap::new();
    for (from, to) in list_dependencies(conn)? {
        if let (Some(&from), Some(_)) = (lookup.get(&from), lookup.get(&to)) {
            out.entry(to).or_insert_with(Vec::new).push(from);
        }
    }
    Ok(out)
}
