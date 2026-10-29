use super::*;

/// One expanded occurrence of an existing calendar event.
#[derive(Debug)]
pub(super) struct BusyOccurrence {
    pub(super) event_id: String,
    pub(super) event_title: String,
    pub(super) calendar_id: String,
    pub(super) recurring: bool,
    pub(super) start: DateTime<Utc>,
    pub(super) end: DateTime<Utc>,
}

/// Expands every active event, including recurrences, into horizon occurrences.
pub(super) fn busy_occurrences(
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
    let starts = crate::recurrence::occurrence_starts(event, horizon_start, horizon_end)
        .map_err(PlannerError::Validation)?
        .unwrap_or_else(|| vec![start]);
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

/// Persisted evidence naming the calendar occurrence that blocked a slot.
pub(super) fn busy_blocker(
    occurrence: &SolverBusy,
    timezone: Tz,
) -> crate::models::PlannerBusyBlocker {
    crate::models::PlannerBusyBlocker {
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

/// Applied task blocks with the inbox successors that must start after them.
///
/// A dependency whose predecessor is no longer in the inbox is only satisfied
/// by an active applied event. When that event is gone the dependency cannot
/// be enforced, so loading fails instead of silently dropping the constraint.
pub(super) fn applied_blocks(
    conn: &Connection,
    tasks: &[PlanningTask],
    dependencies: &[(String, String)],
) -> Result<Vec<SolverAppliedBlock>, PlannerError> {
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
            "SELECT t.id, t.cognitive_load, e.id, e.start_at, e.end_at, e.timezone
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
            ))
        })
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;

    let applied_ids: std::collections::HashSet<&str> =
        rows.iter().map(|(id, ..)| id.as_str()).collect();
    let mut blocks = Vec::new();
    for (task_id, load, event_id, start_at, end_at, timezone) in &rows {
        let start = time::resolve_utc_datetime(start_at, timezone).map_err(validation)?;
        let end = time::resolve_utc_datetime(end_at, timezone).map_err(validation)?;
        blocks.push(SolverAppliedBlock {
            id: format!("applied:{event_id}"),
            start,
            end,
            high: load == "high",
            successors: successors.get(task_id).cloned().unwrap_or_default(),
        });
    }
    for (from, to) in dependencies {
        let enforces_inbox_order =
            index_of.contains_key(to.as_str()) && !index_of.contains_key(from.as_str());
        if enforces_inbox_order && !applied_ids.contains(from.as_str()) {
            return Err(PlannerError::Conflict(format!(
                "dependency predecessor '{from}' has no active event, so its inbox successors cannot be scheduled; return it to the inbox or restore its event"
            )));
        }
    }
    Ok(blocks)
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
