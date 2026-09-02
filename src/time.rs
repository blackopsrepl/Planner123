use std::str::FromStr;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

pub const STORAGE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn local_timezone_name() -> String {
    iana_time_zone::get_timezone()
        .ok()
        .filter(|timezone| Tz::from_str(timezone).is_ok())
        .unwrap_or_else(|| "UTC".to_string())
}

pub fn parse_timezone(value: &str) -> Result<Tz> {
    Tz::from_str(value).map_err(|_| anyhow!("invalid timezone '{}'", value))
}

pub fn normalize_timezone(value: &str) -> Result<String> {
    Ok(parse_timezone(value)?.name().to_string())
}

pub fn parse_timestamp(value: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, STORAGE_FORMAT).map_err(|_| {
        anyhow!(
            "invalid timestamp '{}'; expected YYYY-MM-DD HH:MM:SS",
            value
        )
    })
}

pub fn normalize_timestamp(value: &str) -> Result<String> {
    Ok(parse_timestamp(value)?.format(STORAGE_FORMAT).to_string())
}

pub fn resolve_local_datetime(value: &str, timezone: &str) -> Result<DateTime<Tz>> {
    let timezone = parse_timezone(timezone)?;
    let timestamp = parse_timestamp(value)?;
    resolve_local(timestamp, timezone)
}

pub fn resolve_utc_datetime(value: &str, timezone: &str) -> Result<DateTime<Utc>> {
    Ok(resolve_local_datetime(value, timezone)?.with_timezone(&Utc))
}

pub fn validate_range(start_at: &str, end_at: &str, timezone: &str) -> Result<()> {
    let start = resolve_utc_datetime(start_at, timezone)?;
    let end = resolve_utc_datetime(end_at, timezone)?;
    if end < start {
        bail!("end_at must be greater than or equal to start_at");
    }
    Ok(())
}

pub fn normalize_all_day_bounds(start_at: &str, end_at: &str) -> Result<(String, String)> {
    let start = parse_timestamp(start_at)?;
    let end = parse_timestamp(end_at)?;
    let start_date = start.date();
    let end_date = end.date();
    if end_date < start_date {
        bail!("end_at must be greater than or equal to start_at");
    }
    Ok((
        format!("{} 00:00:00", start_date.format("%Y-%m-%d")),
        format!("{} 23:59:59", end_date.format("%Y-%m-%d")),
    ))
}

fn resolve_local(timestamp: NaiveDateTime, timezone: Tz) -> Result<DateTime<Tz>> {
    match timezone.from_local_datetime(&timestamp) {
        LocalResult::Single(dt) => Ok(dt),
        LocalResult::Ambiguous(_, _) => bail!(
            "timestamp '{}' is ambiguous in timezone '{}'",
            timestamp.format(STORAGE_FORMAT),
            timezone.name()
        ),
        LocalResult::None => bail!(
            "timestamp '{}' does not exist in timezone '{}'",
            timestamp.format(STORAGE_FORMAT),
            timezone.name()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        local_timezone_name, normalize_all_day_bounds, normalize_timestamp, resolve_utc_datetime,
        validate_range,
    };

    #[test]
    fn local_timezone_detection_always_returns_a_timezone() {
        assert!(!local_timezone_name().is_empty());
    }

    #[test]
    fn validate_range_uses_the_supplied_timezone() {
        validate_range("2026-04-12 09:00:00", "2026-04-12 10:00:00", "Europe/Rome").unwrap();
    }

    #[test]
    fn resolve_utc_datetime_round_trips_wall_clock_time() {
        let dt = resolve_utc_datetime("2026-04-12 09:30:00", "Europe/Rome").unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-04-12 07:30:00"
        );
    }

    #[test]
    fn normalize_all_day_bounds_snaps_to_whole_day() {
        let (start, end) =
            normalize_all_day_bounds("2026-04-12 08:00:00", "2026-04-14 13:00:00").unwrap();
        assert_eq!(start, "2026-04-12 00:00:00");
        assert_eq!(end, "2026-04-14 23:59:59");
    }

    #[test]
    fn normalize_timestamp_rejects_invalid_input() {
        let err = normalize_timestamp("bad").unwrap_err();
        assert!(err.to_string().contains("expected YYYY-MM-DD HH:MM:SS"));
    }
}
