/* Pure placement of timed events inside one day column of the time grid.

The grid draws `ROWS_PER_HOUR` text rows per hour, so each row stands for
`60 / ROWS_PER_HOUR` minutes. An event covers every row its local interval
touches, clipped to the day it is drawn on, and always at least one row so
its title is visible. Events whose row spans intersect are drawn side by
side in lanes: two blocks that share a drawn row never share a cell, and an
event is never skipped because another one already covers its hour. */

use chrono::{Duration, NaiveDate, NaiveDateTime};

use crate::models::Event;

pub const ROWS_PER_HOUR: u32 = 2;
const MINUTES_PER_ROW: i64 = 60 / ROWS_PER_HOUR as i64;
pub const ROWS_PER_DAY: u32 = 24 * ROWS_PER_HOUR;

#[derive(Debug, Clone)]
pub struct Placement<'a> {
    pub event: &'a Event,
    /// First row from local midnight, inclusive.
    pub top: u32,
    /// Row after the last one covered.
    pub bottom: u32,
    pub lane: u16,
    /// Lane count of the overlap cluster this event belongs to.
    pub lanes: u16,
}

/* Row span `[top, bottom)` of `event` on `date`, or `None` when the event is
all-day, unparseable, or does not touch the day. */
pub fn row_span(event: &Event, date: NaiveDate) -> Option<(u32, u32)> {
    if event.all_day {
        return None;
    }
    let start = event.start_local_dt()?.naive_local();
    let end = event.end_local_dt()?.naive_local().max(start);
    let day_start = date.and_hms_opt(0, 0, 0)?;
    let day_end = day_start + Duration::days(1);
    let touches = start < day_end && (end > day_start || (start == end && start >= day_start));
    if !touches {
        return None;
    }
    let minutes = |at: NaiveDateTime| (at.clamp(day_start, day_end) - day_start).num_minutes();
    let top = (minutes(start) / MINUTES_PER_ROW) as u32;
    let bottom = (minutes(end) + MINUTES_PER_ROW - 1) / MINUTES_PER_ROW;
    let top = top.min(ROWS_PER_DAY - 1);
    Some((top, (bottom as u32).clamp(top + 1, ROWS_PER_DAY)))
}

/* Places every timed event of `date`, assigning lanes per overlap cluster. */
pub fn layout_day<'a>(events: &[&'a Event], date: NaiveDate) -> Vec<Placement<'a>> {
    let mut spans: Vec<(&Event, u32, u32)> = events
        .iter()
        .filter_map(|event| row_span(event, date).map(|(top, bottom)| (*event, top, bottom)))
        .collect();
    spans.sort_by(|a, b| {
        (a.1, std::cmp::Reverse(a.2), &a.0.start_at, &a.0.id).cmp(&(
            b.1,
            std::cmp::Reverse(b.2),
            &b.0.start_at,
            &b.0.id,
        ))
    });

    let mut placed: Vec<Placement<'a>> = Vec::with_capacity(spans.len());
    let mut cluster_start = 0;
    let mut cluster_bottom = 0;
    // Row after which each lane of the current cluster is free again.
    let mut lane_free_at: Vec<u32> = Vec::new();
    for (event, top, bottom) in spans {
        if top >= cluster_bottom {
            close_cluster(&mut placed[cluster_start..], lane_free_at.len());
            cluster_start = placed.len();
            lane_free_at.clear();
        }
        let lane = match lane_free_at.iter().position(|free| *free <= top) {
            Some(lane) => {
                lane_free_at[lane] = bottom;
                lane
            }
            None => {
                lane_free_at.push(bottom);
                lane_free_at.len() - 1
            }
        };
        cluster_bottom = cluster_bottom.max(bottom);
        placed.push(Placement {
            event,
            top,
            bottom,
            lane: lane as u16,
            lanes: 0,
        });
    }
    close_cluster(&mut placed[cluster_start..], lane_free_at.len());
    placed
}

fn close_cluster(cluster: &mut [Placement<'_>], lanes: usize) {
    for placement in cluster {
        placement.lanes = lanes as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, start: &str, end: &str) -> Event {
        let mut event = Event::new("cal", id, start, end, "UTC");
        event.id = id.into();
        event
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
    }

    fn spans(events: &[Event]) -> Vec<(String, u32, u32, u16, u16)> {
        let refs: Vec<&Event> = events.iter().collect();
        layout_day(&refs, day())
            .into_iter()
            .map(|p| (p.event.id.clone(), p.top, p.bottom, p.lane, p.lanes))
            .collect()
    }

    /* No two placements may share a drawn cell: same lane implies disjoint rows. */
    fn assert_disjoint(events: &[Event]) {
        let refs: Vec<&Event> = events.iter().collect();
        let placed = layout_day(&refs, day());
        assert_eq!(placed.len(), events.len(), "every timed event is placed");
        for (i, a) in placed.iter().enumerate() {
            assert!(a.lane < a.lanes);
            for b in &placed[i + 1..] {
                let rows_meet = a.top < b.bottom && b.top < a.bottom;
                assert!(
                    !(rows_meet && a.lane == b.lane),
                    "{a:?} collides with {b:?}"
                );
                if rows_meet {
                    assert_eq!(a.lanes, b.lanes, "one cluster, one lane count");
                }
            }
        }
    }

    #[test]
    fn half_hour_start_lands_on_its_own_row() {
        let events = [event(
            "dentist",
            "2026-11-02 09:30:00",
            "2026-11-02 10:00:00",
        )];
        assert_eq!(spans(&events), vec![("dentist".into(), 19, 20, 0, 1)]);
    }

    #[test]
    fn back_to_back_events_stack_instead_of_sharing_rows() {
        let events = [
            event("dentist", "2026-11-02 09:30:00", "2026-11-02 10:00:00"),
            event("demo", "2026-11-02 10:00:00", "2026-11-02 11:30:00"),
        ];
        assert_eq!(
            spans(&events),
            vec![
                ("dentist".into(), 19, 20, 0, 1),
                ("demo".into(), 20, 23, 0, 1)
            ]
        );
    }

    #[test]
    fn an_event_inside_a_longer_one_gets_its_own_lane() {
        let events = [
            event("long", "2026-11-02 09:00:00", "2026-11-02 11:00:00"),
            event("inside", "2026-11-02 10:00:00", "2026-11-02 10:30:00"),
        ];
        assert_eq!(
            spans(&events),
            vec![
                ("long".into(), 18, 22, 0, 2),
                ("inside".into(), 20, 21, 1, 2)
            ]
        );
    }

    #[test]
    fn a_lane_is_reused_once_its_event_ends() {
        let events = [
            event("a", "2026-11-02 13:00:00", "2026-11-02 15:00:00"),
            event("b", "2026-11-02 13:30:00", "2026-11-02 14:00:00"),
            event("c", "2026-11-02 14:00:00", "2026-11-02 16:00:00"),
        ];
        assert_eq!(
            spans(&events),
            vec![
                ("a".into(), 26, 30, 0, 2),
                ("b".into(), 27, 28, 1, 2),
                ("c".into(), 28, 32, 1, 2)
            ]
        );
        assert_disjoint(&events);
    }

    #[test]
    fn events_sharing_a_row_without_overlapping_in_time_are_split() {
        let events = [
            event("a", "2026-11-02 14:00:00", "2026-11-02 14:15:00"),
            event("b", "2026-11-02 14:15:00", "2026-11-02 14:30:00"),
        ];
        assert_disjoint(&events);
        assert_eq!(spans(&events)[1].3, 1);
    }

    #[test]
    fn multi_day_events_are_clipped_to_each_day() {
        let event = event("trip", "2026-11-01 22:00:00", "2026-11-03 01:00:00");
        assert_eq!(row_span(&event, day()), Some((0, ROWS_PER_DAY)));
        let next = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        assert_eq!(row_span(&event, next), Some((0, 2)));
        let before = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        assert_eq!(row_span(&event, before), Some((44, ROWS_PER_DAY)));
    }

    #[test]
    fn an_event_ending_at_midnight_does_not_spill_into_the_next_day() {
        let event = event("late", "2026-11-02 23:00:00", "2026-11-03 00:00:00");
        let next = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        assert_eq!(row_span(&event, next), None);
    }

    #[test]
    fn all_day_events_are_not_part_of_the_time_grid() {
        let mut event = event("holiday", "2026-11-02 00:00:00", "2026-11-03 00:00:00");
        event.all_day = true;
        assert_eq!(row_span(&event, day()), None);
    }

    #[test]
    fn a_dense_day_never_draws_two_blocks_in_one_cell() {
        let events: Vec<Event> = (0..40)
            .map(|i| {
                let start = 8 * 60 + (i * 37) % 600;
                let end = start + 15 + (i * 53) % 150;
                let at = |m: i64| format!("2026-11-02 {:02}:{:02}:00", m / 60, m % 60);
                event(&format!("e{i}"), &at(start), &at(end.min(23 * 60 + 59)))
            })
            .collect();
        assert_disjoint(&events);
    }
}
