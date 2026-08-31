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
