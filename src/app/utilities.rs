use super::*;
use std::collections::BTreeMap;

pub(super) fn availability_from_specs(value: &str) -> Result<crate::planner::Availability, String> {
    let mut days = BTreeMap::new();
    for spec in value
        .split(',')
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
    {
        let (day, window) = spec
            .split_once('=')
            .ok_or_else(|| "Availability uses day=HH:MM-HH:MM, separated by commas.".to_string())?;
        let (start, end) = window
            .split_once('-')
            .ok_or_else(|| "Availability uses day=HH:MM-HH:MM, separated by commas.".to_string())?;
        days.entry(day.trim().to_ascii_lowercase())
            .or_insert_with(Vec::new)
            .push(crate::planner::TimeWindow {
                start: start.trim().into(),
                end: end.trim().into(),
            });
    }
    let availability = crate::planner::Availability(days);
    crate::planner::validate_availability(&availability)
        .map_err(|error| format!("Weekly availability: {error}"))?;
    Ok(availability)
}

pub(super) fn default_weekly_availability() -> String {
    "mon=09:00-17:00, tue=09:00-17:00, wed=09:00-17:00, thu=09:00-17:00, fri=09:00-17:00".into()
}

pub(super) fn availability_to_specs(availability: &crate::planner::Availability) -> String {
    availability
        .0
        .iter()
        .flat_map(|(day, windows)| {
            windows
                .iter()
                .map(move |window| format!("{day}={}-{}", window.start, window.end))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month_year = if month == 12 { year + 1 } else { year };
    let next_month = if month == 12 { 1 } else { month + 1 };
    NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}

pub(super) fn google_sync_finished_status(
    calendars_succeeded: usize,
    calendars_failed: usize,
    events_added: usize,
    events_updated: usize,
    conflicts_detected: usize,
) -> (String, bool) {
    if calendars_failed > 0 {
        let mut parts = Vec::new();
        if calendars_succeeded > 0 {
            parts.push(format!("{} succeeded", calendars_succeeded));
        }
        parts.push(format!("{} failed", calendars_failed));
        if conflicts_detected > 0 {
            parts.push(format!("{} conflicts", conflicts_detected));
        }
        return (
            format!(
                "Google sync finished with failures: {}. Open Google management for details.",
                parts.join(", ")
            ),
            true,
        );
    }

    let status = if conflicts_detected > 0 {
        format!(
            "Google sync: +{} events, {} updated, {} conflicts.",
            events_added, events_updated, conflicts_detected
        )
    } else {
        format!(
            "Google sync: +{} events, {} updated.",
            events_added, events_updated
        )
    };
    (status, false)
}
