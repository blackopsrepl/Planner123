//! Wall-clock window arithmetic shared by the availability and cognitive rules.
//!
//! Both rules count the minutes of a task whose local clock time falls inside
//! a daily window, optionally only on one weekday. A minute belongs to the
//! date and time it shows on the local clock, so a window that wraps past
//! midnight (`22:00-02:00`) counts its early-morning part on the next date.

use chrono::{DateTime, Datelike, Duration, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;

/// Whether `value` lies in the daily window `[start, end)`; a window with
/// `start == end` covers the whole day, `start > end` wraps past midnight.
pub(super) fn clock_contains(value: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start == end {
        return true;
    }
    if start < end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}

/// Minutes of `[from, to)` whose local clock time in `timezone` lies in the
/// daily window, restricted to local `weekday` (days from Monday) when given.
///
/// When the UTC offset is the same at the first and the last minute, local
/// time is a fixed shift of UTC across the interval and the count is the
/// overlap of the local interval with each day's window segments. Across a
/// DST transition the shift changes mid-interval, so each minute is read off
/// the clock individually; both paths count exactly the same minutes.
pub(super) fn minutes_in_window(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    timezone: Tz,
    weekday: Option<u32>,
    start: NaiveTime,
    end: NaiveTime,
) -> i64 {
    if to <= from {
        return 0;
    }
    let offset_at = |at: DateTime<Utc>| timezone.offset_from_utc_datetime(&at.naive_utc()).fix();
    if offset_at(from) != offset_at(to - Duration::minutes(1)) {
        return minutes_in_window_per_minute(from, to, timezone, weekday, start, end);
    }
    let local_from = from.with_timezone(&timezone).naive_local();
    let local_to = local_from + (to - from);
    let mut total = 0;
    let mut date = local_from.date();
    while date.and_time(NaiveTime::MIN) < local_to {
        if weekday.is_none_or(|day| date.weekday().num_days_from_monday() == day) {
            for (segment_start, segment_end) in day_segments(date, start, end) {
                let overlap_start = segment_start.max(local_from);
                let overlap_end = segment_end.min(local_to);
                if overlap_end > overlap_start {
                    total += (overlap_end - overlap_start).num_minutes();
                }
            }
        }
        let Some(next) = date.succ_opt() else { break };
        date = next;
    }
    total
}

/// The parts of `date` that the daily window covers, as local intervals.
fn day_segments(
    date: chrono::NaiveDate,
    start: NaiveTime,
    end: NaiveTime,
) -> Vec<(NaiveDateTime, NaiveDateTime)> {
    let day_start = date.and_time(NaiveTime::MIN);
    let day_end = day_start + Duration::days(1);
    let at = |time: NaiveTime| date.and_time(time);
    if start == end {
        vec![(day_start, day_end)]
    } else if start < end {
        vec![(at(start), at(end))]
    } else {
        vec![(day_start, at(end)), (at(start), day_end)]
    }
}

/// Reference definition: read every minute's local clock individually.
fn minutes_in_window_per_minute(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    timezone: Tz,
    weekday: Option<u32>,
    start: NaiveTime,
    end: NaiveTime,
) -> i64 {
    let minutes = (to - from).num_minutes();
    (0..minutes)
        .filter(|offset| {
            let local = (from + Duration::minutes(*offset)).with_timezone(&timezone);
            weekday.is_none_or(|day| local.weekday().num_days_from_monday() == day)
                && clock_contains(local.time(), start, end)
        })
        .count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    /// The fast path must agree with the per-minute definition everywhere,
    /// including across both Europe/Rome DST transitions of 2026.
    #[test]
    fn segment_overlap_matches_the_per_minute_definition() {
        let windows = [
            (clock(9, 0), clock(17, 0)),
            (clock(22, 0), clock(2, 0)),
            (clock(8, 0), clock(8, 0)),
            (clock(1, 30), clock(3, 15)),
        ];
        let starts = [
            Utc.with_ymd_and_hms(2026, 3, 28, 20, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 10, 24, 21, 45, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 11, 2, 6, 15, 0).unwrap(),
        ];
        for timezone in [
            chrono_tz::Europe::Rome,
            chrono_tz::UTC,
            chrono_tz::Asia::Kathmandu,
        ] {
            for base in starts {
                for step in 0..40 {
                    let from = base + Duration::minutes(step * 45);
                    let to = from + Duration::minutes(15 + (step * 37) % 300);
                    for (start, end) in windows {
                        for weekday in [None, Some(5), Some(6), Some(0)] {
                            assert_eq!(
                                minutes_in_window(from, to, timezone, weekday, start, end),
                                minutes_in_window_per_minute(
                                    from, to, timezone, weekday, start, end
                                ),
                                "{timezone} {from}..{to} {weekday:?} {start}-{end}"
                            );
                        }
                    }
                }
            }
        }
    }
}
