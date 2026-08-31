fn prepare_event(
    conn: &Connection,
    event: &mut Event,
    is_new: bool,
) -> Result<crate::models::Calendar, EventServiceError> {
    if is_new {
        event.id = if event.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            event.id.trim().to_string()
        };
    } else {
        ensure_event_exists(conn, &event.id)?;
    }

    let calendar = db::get_calendar(conn, &event.calendar_id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::NotFound {
            resource: "calendar",
            id: event.calendar_id.clone(),
        })?;

    if let Some(project_id) = event.project_id.as_deref() {
        let exists = db::get_project(conn, project_id).map_err(map_internal)?;
        if exists.is_none() {
            return Err(EventServiceError::NotFound {
                resource: "project",
                id: project_id.to_string(),
            });
        }
    }

    if event.title.trim().is_empty() {
        return Err(EventServiceError::Validation(
            "title cannot be empty".to_string(),
        ));
    }

    event.title = event.title.trim().to_string();
    event.description = normalize_optional(event.description.take());
    event.location = normalize_optional(event.location.take());
    event.rrule = normalize_rrule(normalize_optional(event.rrule.take()))?;
    event.timezone = time::normalize_timezone(event.timezone.trim()).map_err(map_validation)?;

    if event.all_day {
        let (start_at, end_at) = time::normalize_all_day_bounds(&event.start_at, &event.end_at)
            .map_err(map_validation)?;
        event.start_at = start_at;
        event.end_at = end_at;
    } else {
        event.start_at = time::normalize_timestamp(&event.start_at).map_err(map_validation)?;
        event.end_at = time::normalize_timestamp(&event.end_at).map_err(map_validation)?;
    }

    time::validate_range(&event.start_at, &event.end_at, &event.timezone)
        .map_err(map_validation)?;

    if calendar.source == CalendarSource::Google
        && !state::google_calendar_writable(conn, &calendar).map_err(map_sync_error)?
    {
        return Err(EventServiceError::Conflict(format!(
            "calendar '{}' is read-only and cannot accept local edits",
            calendar.name
        )));
    }

    let now = timestamp_now();
    if is_new {
        event.created_at = now.clone();
    }
    event.updated_at = now;

    Ok(calendar)
}

fn normalize_rrule(value: Option<String>) -> Result<Option<String>, EventServiceError> {
    let Some(value) = value else {
        return Ok(None);
    };

    if value.starts_with("RRULE:") {
        return Err(EventServiceError::Validation(
            "rrule must contain rule content beginning with FREQ=, not an RRULE: property prefix"
                .to_string(),
        ));
    }
    if !value.starts_with("FREQ=") {
        return Err(EventServiceError::Validation(
            "rrule must contain rule content beginning with FREQ=".to_string(),
        ));
    }

    Ok(Some(value))
}

fn ensure_event_exists(conn: &Connection, event_id: &str) -> Result<(), EventServiceError> {
    if db::get_event(conn, event_id)
        .map_err(map_internal)?
        .is_none()
    {
        return Err(EventServiceError::NotFound {
            resource: "event",
            id: event_id.to_string(),
        });
    }
    Ok(())
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

fn map_internal(err: impl std::fmt::Display) -> EventServiceError {
    EventServiceError::Internal(err.to_string())
}

fn map_validation(err: AnyError) -> EventServiceError {
    EventServiceError::Validation(err.to_string())
}

fn map_sync_error(err: impl std::fmt::Display) -> EventServiceError {
    let message = err.to_string();
    if message.contains("read-only") || message.contains("not supported") {
        EventServiceError::Conflict(message)
    } else {
        EventServiceError::Internal(message)
    }
}

fn timestamp_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
