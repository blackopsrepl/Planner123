/* iCal (.ics) import/export.  */

use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use icalendar::{Calendar as ICalCalendar, Component, Event as ICalEvent, EventLike};
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    db, event_service,
    models::{Calendar, Event},
};

#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub calendar_id: String,
    pub calendar_name: String,
    pub path: String,
    pub imported: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedProperty {
    name: String,
    params: HashMap<String, String>,
    value: String,
}

#[derive(Debug, Default, Clone)]
struct ParsedEventBlock {
    properties: Vec<ParsedProperty>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ImportCandidate {
    event: Option<Event>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
enum ParsedEventTime {
    AllDay {
        date: NaiveDate,
    },
    Timed {
        utc: DateTime<Utc>,
        timezone: String,
    },
}

// ── Export ────────────────────────────────────────────────────────────

/* Export a list of events to an iCal string. */
pub fn export_events(events: &[Event], calendar_name: &str) -> String {
    let mut cal = ICalCalendar::new();
    cal.name(calendar_name);
    cal.description("Exported from SolverForge Calendar");

    for ev in events {
        let mut ical_event = ICalEvent::new();
        ical_event.uid(&ev.id);
        ical_event.summary(&ev.title);

        if let Some(desc) = &ev.description {
            ical_event.description(desc);
        }
        if let Some(loc) = &ev.location {
            ical_event.location(loc);
        }

        // Timestamps
        if ev.all_day {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&ev.start_at[..10], "%Y-%m-%d") {
                ical_event.add_property("DTSTART;VALUE=DATE", date.format("%Y%m%d").to_string());
            }
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&ev.end_at[..10], "%Y-%m-%d") {
                ical_event.add_property("DTEND;VALUE=DATE", date.format("%Y%m%d").to_string());
            }
        } else {
            if let Some(dt) = ev.start_dt() {
                ical_event.starts(dt);
            }
            if let Some(dt) = ev.end_dt() {
                ical_event.ends(dt);
            }
        }

        // RRULE
        if let Some(rrule) = &ev.rrule {
            ical_event.add_property("RRULE", rrule.strip_prefix("RRULE:").unwrap_or(rrule));
        }

        cal.push(ical_event.done());
    }

    cal.to_string()
}

/* Write events to an .ics file. */
pub fn export_to_file(events: &[Event], calendar_name: &str, path: &std::path::Path) -> Result<()> {
    let content = export_events(events, calendar_name);
    std::fs::write(path, content)
        .with_context(|| format!("cannot write iCal to {}", path.display()))
}

// ── Import ────────────────────────────────────────────────────────────

pub fn import_from_file(
    conn: &Connection,
    calendar_id: &str,
    path: &Path,
    default_timezone: &str,
) -> Result<ImportReport> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read iCal from {}", path.display()))?;
    import_from_str_with_label(
        conn,
        calendar_id,
        &content,
        default_timezone,
        path.display().to_string(),
    )
}

pub fn import_from_str(
    conn: &Connection,
    calendar_id: &str,
    content: &str,
    default_timezone: &str,
) -> Result<ImportReport> {
    import_from_str_with_label(
        conn,
        calendar_id,
        content,
        default_timezone,
        "<memory>".to_string(),
    )
}

fn import_from_str_with_label(
    conn: &Connection,
    calendar_id: &str,
    content: &str,
    default_timezone: &str,
    label: String,
) -> Result<ImportReport> {
    let calendar = db::get_calendar(conn, calendar_id)?
        .ok_or_else(|| anyhow!("calendar '{}' not found", calendar_id))?;
    let default_timezone = crate::time::normalize_timezone(default_timezone)?;
    let parsed_events = parse_events(content);
    if parsed_events.is_empty() {
        bail!("no VEVENT entries found in iCal input");
    }

    let mut report = ImportReport {
        calendar_id: calendar.id.clone(),
        calendar_name: calendar.name.clone(),
        path: label,
        imported: 0,
        skipped: 0,
        warnings: Vec::new(),
    };

    for parsed_event in parsed_events {
        let candidate = build_import_candidate(&calendar, parsed_event, &default_timezone);
        report.warnings.extend(candidate.warnings);
        if let Some(event) = candidate.event {
            event_service::save_event(conn, event, true).map_err(|error| anyhow!(error))?;
            report.imported += 1;
        } else {
            report.skipped += 1;
        }
    }

    Ok(report)
}

fn parse_events(input: &str) -> Vec<ParsedEventBlock> {
    let mut events = Vec::new();
    let mut current: Option<ParsedEventBlock> = None;
    let mut nested_components: Vec<String> = Vec::new();

    for line in unfold_lines(input) {
        let trimmed = line.trim();
        let upper = trimmed.to_ascii_uppercase();

        if let Some(current_event) = current.as_mut() {
            if let Some(component) = upper.strip_prefix("BEGIN:") {
                let component_name = trimmed[6..].trim().to_string();
                nested_components.push(component_name.clone());
                current_event.warnings.push(format!(
                    "Ignored unsupported {} component inside VEVENT.",
                    component_name
                ));
                let _ = component;
                continue;
            }

            if let Some(component) = upper.strip_prefix("END:") {
                if nested_components
                    .last()
                    .map(|active| active.eq_ignore_ascii_case(component))
                    .unwrap_or(false)
                {
                    nested_components.pop();
                    continue;
                }
            }

            if !nested_components.is_empty() {
                continue;
            }
        }

        if upper == "BEGIN:VEVENT" {
            current = Some(ParsedEventBlock::default());
            nested_components.clear();
            continue;
        }

        if upper == "END:VEVENT" {
            if let Some(event) = current.take() {
                events.push(event);
            }
            nested_components.clear();
            continue;
        }

        if let Some(current_event) = current.as_mut() {
            if let Some(property) = parse_property(trimmed) {
                current_event.properties.push(property);
            }
        }
    }

    events
}

fn unfold_lines(input: &str) -> Vec<String> {
    let mut unfolded: Vec<String> = Vec::new();
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    for line in normalized.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(previous) = unfolded.last_mut() {
                previous.push_str(line[1..].trim_end());
            }
        } else {
            unfolded.push(line.trim_end().to_string());
        }
    }
    unfolded
}

fn parse_property(line: &str) -> Option<ParsedProperty> {
    let (head, value) = line.split_once(':')?;
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let mut params = HashMap::new();
    for param in parts {
        let (key, raw_value) = param.split_once('=').unwrap_or((param, ""));
        params.insert(
            key.trim().to_ascii_uppercase(),
            raw_value.trim().trim_matches('"').to_string(),
        );
    }
    Some(ParsedProperty {
        name,
        params,
        value: value.to_string(),
    })
}

fn build_import_candidate(
    calendar: &Calendar,
    parsed_event: ParsedEventBlock,
    default_timezone: &str,
) -> ImportCandidate {
    let title = parsed_event
        .properties
        .iter()
        .find(|property| property.name == "SUMMARY")
        .map(|property| unescape_text(&property.value))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Untitled event".to_string());
    let mut warnings: BTreeSet<String> = parsed_event.warnings.into_iter().collect();

    match build_import_event(calendar, &parsed_event.properties, default_timezone, &title) {
        Ok(candidate) => {
            warnings.extend(candidate.warnings);
            ImportCandidate {
                event: candidate.event,
                warnings: warnings.into_iter().collect(),
            }
        }
        Err(error) => {
            warnings.insert(format!("Skipped event '{}': {}", title, error));
            ImportCandidate {
                event: None,
                warnings: warnings.into_iter().collect(),
            }
        }
    }
}

fn build_import_event(
    calendar: &Calendar,
    properties: &[ParsedProperty],
    default_timezone: &str,
    title: &str,
) -> Result<ImportCandidate> {
    let mut warnings: BTreeSet<String> = BTreeSet::new();
    for property in properties {
        match property.name.as_str() {
            "EXDATE" | "RDATE" | "RECURRENCE-ID" => {
                warnings.insert(format!(
                    "Skipped event '{}' because {} is not supported yet.",
                    title, property.name
                ));
                return Ok(ImportCandidate {
                    event: None,
                    warnings: warnings.into_iter().collect(),
                });
            }
            "DURATION" => {
                warnings.insert(format!(
                    "Skipped event '{}' because DURATION-based events are not supported yet.",
                    title
                ));
                return Ok(ImportCandidate {
                    event: None,
                    warnings: warnings.into_iter().collect(),
                });
            }
            "STATUS" if property.value.eq_ignore_ascii_case("CANCELLED") => {
                warnings.insert(format!("Skipped cancelled event '{}'.", title));
                return Ok(ImportCandidate {
                    event: None,
                    warnings: warnings.into_iter().collect(),
                });
            }
            "ATTENDEE" | "ATTACH" | "ORGANIZER" | "GEO" | "URL" => {
                warnings.insert(format!(
                    "Event '{}' ignored unsupported {} data.",
                    title,
                    property.name.to_lowercase()
                ));
            }
            _ => {}
        }
    }

    let start_property =
        find_property(properties, "DTSTART").ok_or_else(|| anyhow!("missing DTSTART in VEVENT"))?;
    let end_property = find_property(properties, "DTEND");
    let start_time = parse_event_time(start_property, default_timezone)?;
    let end_time = match end_property {
        Some(property) => Some(parse_event_time(property, default_timezone)?),
        None => None,
    };
    let start_is_all_day = matches!(&start_time, ParsedEventTime::AllDay { .. });

    let mut event = match (start_time, end_time) {
        (ParsedEventTime::AllDay { date: start_date }, Some(ParsedEventTime::AllDay { date })) => {
            let end_date = if date > start_date {
                date.pred_opt().unwrap_or(start_date)
            } else {
                start_date
            };
            Event::new(
                calendar.id.clone(),
                title,
                format!("{} 00:00:00", start_date.format("%Y-%m-%d")),
                format!("{} 23:59:59", end_date.format("%Y-%m-%d")),
                "UTC",
            )
        }
        (ParsedEventTime::AllDay { date }, None) => Event::new(
            calendar.id.clone(),
            title,
            format!("{} 00:00:00", date.format("%Y-%m-%d")),
            format!("{} 23:59:59", date.format("%Y-%m-%d")),
            "UTC",
        ),
        (
            ParsedEventTime::Timed {
                utc: start_utc,
                timezone: start_timezone,
            },
            Some(ParsedEventTime::Timed {
                utc: end_utc,
                timezone: end_timezone,
            }),
        ) => {
            if start_timezone != end_timezone {
                warnings.insert(format!(
                    "Event '{}' used different start/end timezones; end time was normalized into {}.",
                    title, start_timezone
                ));
            }
            let timezone = crate::time::parse_timezone(&start_timezone)?;
            Event::new(
                calendar.id.clone(),
                title,
                start_utc
                    .with_timezone(&timezone)
                    .format(crate::time::STORAGE_FORMAT)
                    .to_string(),
                end_utc
                    .with_timezone(&timezone)
                    .format(crate::time::STORAGE_FORMAT)
                    .to_string(),
                start_timezone,
            )
        }
        (ParsedEventTime::Timed { .. }, None) => {
            bail!("missing DTEND for timed event");
        }
        _ => {
            bail!("DTSTART and DTEND use incompatible value types");
        }
    };

    event.all_day = start_is_all_day;
    event.description = find_property(properties, "DESCRIPTION")
        .map(|property| unescape_text(&property.value))
        .filter(|value| !value.trim().is_empty());
    event.location = find_property(properties, "LOCATION")
        .map(|property| unescape_text(&property.value))
        .filter(|value| !value.trim().is_empty());
    event.rrule = find_property(properties, "RRULE")
        .map(|property| property.value.trim().to_string())
        .filter(|value| !value.is_empty());

    if event.all_day {
        event.timezone = "UTC".to_string();
    }

    Ok(ImportCandidate {
        event: Some(event),
        warnings: warnings.into_iter().collect(),
    })
}

fn find_property<'a>(properties: &'a [ParsedProperty], name: &str) -> Option<&'a ParsedProperty> {
    properties.iter().find(|property| property.name == name)
}

fn parse_event_time(property: &ParsedProperty, default_timezone: &str) -> Result<ParsedEventTime> {
    if property
        .params
        .get("VALUE")
        .map(|value| value.eq_ignore_ascii_case("DATE"))
        .unwrap_or(false)
    {
        return Ok(ParsedEventTime::AllDay {
            date: parse_basic_date(property.value.trim())?,
        });
    }

    let raw_value = property.value.trim();
    if !raw_value.contains('T') && raw_value.len() == 8 {
        return Ok(ParsedEventTime::AllDay {
            date: parse_basic_date(raw_value)?,
        });
    }

    if raw_value.ends_with('Z') {
        let naive = parse_basic_datetime(raw_value.trim_end_matches('Z'))?;
        return Ok(ParsedEventTime::Timed {
            utc: DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
            timezone: "UTC".to_string(),
        });
    }

    let timezone = property
        .params
        .get("TZID")
        .map(|value| crate::time::normalize_timezone(value))
        .transpose()?
        .unwrap_or_else(|| default_timezone.to_string());
    let naive = parse_basic_datetime(raw_value)?;
    let utc = crate::time::resolve_utc_datetime(
        &naive.format(crate::time::STORAGE_FORMAT).to_string(),
        &timezone,
    )?;
    Ok(ParsedEventTime::Timed { utc, timezone })
}

fn parse_basic_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y%m%d")
        .with_context(|| format!("invalid DATE value '{}'", value))
}

fn parse_basic_datetime(value: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M"))
        .with_context(|| format!("invalid DATE-TIME value '{}'", value))
}

fn unescape_text(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{export_events, import_from_str};
    use crate::{db, models::Event};

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
        (temp, conn)
    }

    fn first_calendar_id(conn: &Connection) -> String {
        db::load_calendars(conn).unwrap()[0].id.clone()
    }

    #[test]
    fn export_round_trips_rrule() {
        let event = Event {
            id: "event-1".to_string(),
            calendar_id: "calendar-1".to_string(),
            project_id: None,
            title: "Planning".to_string(),
            description: Some("Discuss".to_string()),
            location: Some("HQ".to_string()),
            start_at: "2026-04-12 09:00:00".to_string(),
            end_at: "2026-04-12 10:00:00".to_string(),
            all_day: false,
            rrule: Some("FREQ=WEEKLY;COUNT=4".to_string()),
            google_id: None,
            google_etag: None,
            reminder_minutes: None,
            timezone: "Europe/Rome".to_string(),
            created_at: "2026-04-12 08:00:00".to_string(),
            updated_at: "2026-04-12 08:00:00".to_string(),
            deleted_at: None,
        };

        let exported = export_events(&[event], "Demo");
        assert!(exported.contains("RRULE:FREQ=WEEKLY;COUNT=4"));
    }

    #[test]
    fn import_preserves_timed_and_all_day_fields() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let report = import_from_str(
            &conn,
            &calendar_id,
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Planning\r\nDESCRIPTION:Discuss\\nPlan\r\nLOCATION:HQ\r\nDTSTART;TZID=Europe/Rome:20260412T090000\r\nDTEND;TZID=Europe/Rome:20260412T103000\r\nRRULE:FREQ=WEEKLY;COUNT=2\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20260420\r\nDTEND;VALUE=DATE:20260422\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            "Europe/Rome",
        )
        .unwrap();

        assert_eq!(report.imported, 2);
        assert_eq!(report.skipped, 0);

        let events = db::load_events(&conn).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].timezone, "Europe/Rome");
        assert_eq!(events[0].rrule.as_deref(), Some("FREQ=WEEKLY;COUNT=2"));
        assert_eq!(
            crate::google::types::local_event_insert_body(&events[0]).unwrap()["recurrence"],
            serde_json::json!(["RRULE:FREQ=WEEKLY;COUNT=2"])
        );
        assert!(events[1].all_day);
        assert_eq!(events[1].start_at, "2026-04-20 00:00:00");
        assert_eq!(events[1].end_at, "2026-04-21 23:59:59");
    }

    #[test]
    fn import_reports_unsupported_event_shapes() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let report = import_from_str(
            &conn,
            &calendar_id,
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Exception\r\nRECURRENCE-ID:20260412T090000Z\r\nDTSTART:20260412T090000Z\r\nDTEND:20260412T100000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:With Alarm\r\nDTSTART:20260412T110000Z\r\nDTEND:20260412T120000Z\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            "UTC",
        )
        .unwrap();

        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped, 1);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("RECURRENCE-ID")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("VALARM")));
    }
}
