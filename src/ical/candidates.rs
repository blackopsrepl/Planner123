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
