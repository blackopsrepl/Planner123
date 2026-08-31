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
