fn normalize_required(
    value: impl Into<String>,
    field: &'static str,
) -> Result<String, CalendarServiceError> {
    let trimmed = value.into().trim().to_string();
    if trimmed.is_empty() {
        return Err(CalendarServiceError::Validation(format!(
            "{} cannot be empty",
            field
        )));
    }
    Ok(trimmed)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn validate_calendar_source(
    source: &CalendarSource,
    google_id: Option<&str>,
) -> Result<(), CalendarServiceError> {
    match source {
        CalendarSource::Google if google_id.is_none() => Err(CalendarServiceError::Validation(
            "google calendars require a google_id".to_string(),
        )),
        CalendarSource::Local if google_id.is_some() => Err(CalendarServiceError::Validation(
            "local calendars cannot set a google_id".to_string(),
        )),
        _ => Ok(()),
    }
}

fn ensure_google_calendar_not_imported(
    conn: &Connection,
    google_id: &str,
    current_calendar_id: Option<&str>,
) -> Result<(), CalendarServiceError> {
    let existing = db::get_google_calendar_by_google_id(conn, google_id).map_err(map_internal)?;
    if let Some(existing) = existing {
        if current_calendar_id != Some(existing.id.as_str()) {
            return Err(CalendarServiceError::Conflict(format!(
                "google calendar '{}' is already imported",
                google_id
            )));
        }
    }
    Ok(())
}

fn map_write_error(err: AnyError, google_id: Option<&str>) -> CalendarServiceError {
    if is_google_id_unique_violation(&err) {
        if let Some(google_id) = google_id {
            return CalendarServiceError::Conflict(format!(
                "google calendar '{}' is already imported",
                google_id
            ));
        }
        return CalendarServiceError::Conflict("google calendar is already imported".to_string());
    }
    map_internal(err)
}

fn is_google_id_unique_violation(err: &AnyError) -> bool {
    let Some(sqlite_err) = err.downcast_ref::<SqliteError>() else {
        return false;
    };

    match sqlite_err {
        SqliteError::SqliteFailure(code, message)
            if code.code == ErrorCode::ConstraintViolation =>
        {
            message
                .as_deref()
                .map(|message| {
                    message.contains("idx_calendars_google_id_unique")
                        || message.contains("calendars.google_id")
                })
                .unwrap_or(true)
        }
        _ => false,
    }
}

fn map_internal(err: impl std::fmt::Display) -> CalendarServiceError {
    CalendarServiceError::Internal(err.to_string())
}

fn timestamp_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
