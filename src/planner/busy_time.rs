use super::*;

#[derive(Debug)]
pub(super) struct BusyOccurrence {
    pub(super) start: DateTime<Utc>,
    pub(super) end: DateTime<Utc>,
    event_id: String,
    event_title: String,
    calendar_id: String,
    recurring: bool,
}

pub(super) type UtcInterval = (DateTime<Utc>, DateTime<Utc>);

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
    let starts = if let Some(rule) = event.rrule.as_deref() {
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
        result
            .dates
            .into_iter()
            .map(|occurrence| occurrence.with_timezone(&Utc))
            .collect()
    } else {
        vec![start]
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

pub(super) fn busy_blocker(occurrence: &BusyOccurrence, timezone: Tz) -> PlannerBusyBlocker {
    PlannerBusyBlocker {
        event_id: occurrence.event_id.clone(),
        event_title: occurrence.event_title.clone(),
        calendar_id: occurrence.calendar_id.clone(),
        start_at: occurrence
            .start
            .with_timezone(&timezone)
            .format(time::STORAGE_FORMAT)
            .to_string(),
        end_at: occurrence
            .end
            .with_timezone(&timezone)
            .format(time::STORAGE_FORMAT)
            .to_string(),
        recurring: occurrence.recurring,
    }
}
pub(super) fn applied_high_intervals(conn: &Connection) -> Result<Vec<UtcInterval>, PlannerError> {
    let mut stmt=conn.prepare("SELECT e.start_at,e.end_at,e.timezone FROM planning_tasks t JOIN planning_task_events l ON l.task_id=t.id JOIN events e ON e.id=l.event_id WHERE t.cognitive_load='high' AND e.deleted_at IS NULL").map_err(internal)?;
    let rows = stmt
        .query_map([], |row| {
            let start: String = row.get(0)?;
            let end: String = row.get(1)?;
            let timezone: String = row.get(2)?;
            let start = time::resolve_utc_datetime(&start, &timezone)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let end = time::resolve_utc_datetime(&end, &timezone)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok((start, end))
        })
        .map_err(internal)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)
}
pub(super) fn dependency_map(
    conn: &Connection,
    tasks: &[PlanningTask],
) -> Result<HashMap<String, Vec<usize>>, PlannerError> {
    let lookup: HashMap<_, _> = tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| (task.id.clone(), idx))
        .collect();
    let mut out = HashMap::new();
    for (from, to) in list_dependencies(conn)? {
        if let (Some(&from), Some(_)) = (lookup.get(&from), lookup.get(&to)) {
            out.entry(to).or_insert_with(Vec::new).push(from);
        }
    }
    Ok(out)
}

pub(super) fn applied_predecessor_ends(
    conn: &Connection,
    inbox_tasks: &[PlanningTask],
    timezone: &str,
) -> Result<HashMap<String, DateTime<Utc>>, PlannerError> {
    let inbox: std::collections::HashSet<_> =
        inbox_tasks.iter().map(|task| task.id.as_str()).collect();
    let mut ends: HashMap<String, DateTime<Utc>> = HashMap::new();
    for (from, to) in list_dependencies(conn)? {
        if !inbox.contains(to.as_str()) || inbox.contains(from.as_str()) {
            continue;
        }
        let predecessor = require_task(conn, &from)?;
        let event: Option<(String, String)> = conn
            .query_row(
                "SELECT e.end_at,e.timezone FROM planning_task_events l JOIN events e ON e.id=l.event_id WHERE l.task_id=?1 AND e.deleted_at IS NULL",
                [&from],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?;
        let Some((end, event_timezone)) = event else {
            return Err(PlannerError::Conflict(format!(
                "dependency predecessor '{}' is not backed by an active event",
                predecessor.title
            )));
        };
        let end = time::resolve_utc_datetime(&end, &event_timezone)
            .or_else(|_| time::resolve_utc_datetime(&end, timezone))
            .map_err(validation)?;
        ends.entry(to)
            .and_modify(|current| *current = (*current).max(end))
            .or_insert(end);
    }
    Ok(ends)
}
