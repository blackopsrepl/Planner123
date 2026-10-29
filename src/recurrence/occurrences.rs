/* Expansion of recurring events into the occurrences they produce.

The planner treats every occurrence of a recurring event as busy time, and
the calendar views draw every occurrence in the window they show; both read
the dates from `occurrence_starts`, so the screen and the solver can never
disagree about when a recurring event takes place. */

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::models::Event;
use crate::time;

/// Upper bound on occurrences expanded for one event inside one window.
pub const OCCURRENCE_LIMIT: u16 = u16::MAX;

/* Start instants of the occurrences of `event` that overlap `[from, to)`.

`Ok(None)` for an event without a recurrence rule. Errors name the event and
say what is wrong with its timezone, rule, or occurrence count. */
pub fn occurrence_starts(
    event: &Event,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Option<Vec<DateTime<Utc>>>, String> {
    let Some(rule) = event.rrule.as_deref() else {
        return Ok(None);
    };
    let (Some(start), Some(end)) = (event.start_dt(), event.end_dt()) else {
        return Ok(Some(Vec::new()));
    };
    let duration = end - start;
    let timezone = event.timezone_tz().ok_or_else(|| {
        format!(
            "event '{}' has an invalid recurrence timezone '{}'",
            event.title, event.timezone
        )
    })?;
    let timezone = rrule::Tz::from(timezone);
    let invalid = |error: &dyn std::fmt::Display| {
        format!(
            "event '{}' has an invalid recurrence rule: {error}",
            event.title
        )
    };
    let result = rule
        .parse::<rrule::RRule<rrule::Unvalidated>>()
        .map_err(|error| invalid(&error))?
        .build(start.with_timezone(&timezone))
        .map_err(|error| invalid(&error))?
        .after((from - duration).with_timezone(&timezone))
        .before(to.with_timezone(&timezone))
        .all(OCCURRENCE_LIMIT);
    if result.limited {
        return Err(format!(
            "event '{}' recurrence exceeds the {}-occurrence safety limit within the window",
            event.title, OCCURRENCE_LIMIT
        ));
    }
    Ok(Some(
        result
            .dates
            .into_iter()
            .map(|occurrence| occurrence.with_timezone(&Utc))
            .filter(|occurrence| *occurrence < to && *occurrence + duration > from)
            .collect(),
    ))
}

/* Every event as it appears on the calendar between the local dates `first`
and `last` inclusive: one-off events as they are, recurring events as one
copy per occurrence, each carrying the series id and its own start and end.

A recurring event whose rule cannot be expanded is shown once, at the time
it is stored, so a malformed rule never hides the event itself. */
pub fn occurrences_between(events: &[Event], first: NaiveDate, last: NaiveDate) -> Vec<Event> {
    let mut shown = Vec::with_capacity(events.len());
    for event in events {
        let Some((from, to)) = local_window(event, first, last) else {
            shown.push(event.clone());
            continue;
        };
        match occurrence_starts(event, from, to) {
            Ok(Some(starts)) => shown.extend(starts.into_iter().map(|start| at(event, start))),
            Ok(None) | Err(_) => shown.push(event.clone()),
        }
    }
    shown.sort_by(|a, b| (&a.start_at, &a.title, &a.id).cmp(&(&b.start_at, &b.title, &b.id)));
    shown
}

/* `[first 00:00, last + 1 00:00)` in the event's own timezone, as UTC. */
fn local_window(
    event: &Event,
    first: NaiveDate,
    last: NaiveDate,
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let timezone = event.timezone_tz()?;
    let midnight = |date: NaiveDate| {
        timezone
            .from_local_datetime(&date.and_time(NaiveTime::MIN))
            .earliest()
            .map(|at| at.with_timezone(&Utc))
    };
    Some((midnight(first)?, midnight(last.succ_opt()?)?))
}

/* A copy of `event` moved to the occurrence starting at `start`. */
fn at(event: &Event, start: DateTime<Utc>) -> Event {
    let duration = event
        .start_dt()
        .zip(event.end_dt())
        .map_or(Duration::zero(), |(s, e)| e - s);
    let timezone = event.timezone_tz().unwrap_or(chrono_tz::UTC);
    let local = |at: DateTime<Utc>| {
        at.with_timezone(&timezone)
            .format(time::STORAGE_FORMAT)
            .to_string()
    };
    let mut occurrence = event.clone();
    occurrence.start_at = local(start);
    occurrence.end_at = local(start + duration);
    occurrence
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weekly_standup() -> Event {
        let mut event = Event::new(
            "cal",
            "Standup",
            "2026-10-05 09:00:00",
            "2026-10-05 09:30:00",
            "Europe/Rome",
        );
        event.rrule = Some("FREQ=WEEKLY;BYDAY=MO,WE".into());
        event
    }

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    #[test]
    fn a_series_started_weeks_earlier_appears_on_every_day_it_repeats() {
        let shown = occurrences_between(&[weekly_standup()], day(2), day(8));
        let starts: Vec<&str> = shown.iter().map(|e| e.start_at.as_str()).collect();
        assert_eq!(starts, ["2026-11-02 09:00:00", "2026-11-04 09:00:00"]);
        assert!(shown.iter().all(|e| e.end_at.ends_with("09:30:00")));
    }

    #[test]
    fn occurrences_keep_their_wall_clock_time_across_dst() {
        // Rome leaves summer time on Oct 25, 2026; the standup stays at 09:00.
        let shown = occurrences_between(
            &[weekly_standup()],
            NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
        );
        assert!(shown.iter().all(|e| e.start_at.ends_with("09:00:00")));
        assert_eq!(shown.len(), 4);
    }

    #[test]
    fn one_off_events_pass_through_unchanged() {
        let event = Event::new(
            "cal",
            "Dentist",
            "2026-11-05 09:30:00",
            "2026-11-05 10:00:00",
            "UTC",
        );
        assert_eq!(
            occurrences_between(std::slice::from_ref(&event), day(2), day(8))[0].start_at,
            event.start_at
        );
    }

    #[test]
    fn a_malformed_rule_still_shows_the_stored_event() {
        let mut event = weekly_standup();
        event.rrule = Some("FREQ=SOMETIMES".into());
        let shown = occurrences_between(&[event.clone()], day(2), day(8));
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].start_at, event.start_at);
    }
}
