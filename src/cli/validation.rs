fn timestamp_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn internal_error(err: impl std::fmt::Display) -> CliError {
    CliError::internal(err.to_string())
}

fn calendar_service_error(err: calendar_service::CalendarServiceError) -> CliError {
    match err {
        calendar_service::CalendarServiceError::NotFound { resource, id } => {
            CliError::not_found(resource, &id)
        }
        calendar_service::CalendarServiceError::Validation(message) => {
            CliError::validation(message)
        }
        calendar_service::CalendarServiceError::Conflict(message) => CliError::conflict(message),
        calendar_service::CalendarServiceError::Internal(message) => CliError::internal(message),
    }
}

fn event_service_error(err: event_service::EventServiceError) -> CliError {
    match err {
        event_service::EventServiceError::NotFound { resource, id } => {
            CliError::not_found(resource, &id)
        }
        event_service::EventServiceError::Validation(message) => CliError::validation(message),
        event_service::EventServiceError::Conflict(message) => CliError::conflict(message),
        event_service::EventServiceError::Internal(message) => CliError::internal(message),
    }
}

fn require_resource<T>(resource: Option<T>, name: &'static str, id: &str) -> Result<T, CliError> {
    resource.ok_or_else(|| CliError::not_found(name, id))
}

fn non_empty(value: String, field: &'static str) -> Result<String, CliError> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(CliError::validation(format!("{} cannot be empty", field)));
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

fn validate_event_update_args(args: &EventUpdateArgs) -> Result<(), CliError> {
    if args.project_id.is_some() && args.clear_project_id {
        return Err(CliError::validation(
            "cannot combine --project-id with --clear-project-id",
        ));
    }
    if args.description.is_some() && args.clear_description {
        return Err(CliError::validation(
            "cannot combine --description with --clear-description",
        ));
    }
    if args.location.is_some() && args.clear_location {
        return Err(CliError::validation(
            "cannot combine --location with --clear-location",
        ));
    }
    if args.rrule.is_some() && args.clear_rrule {
        return Err(CliError::validation(
            "cannot combine --rrule with --clear-rrule",
        ));
    }
    if args.reminder_minutes.is_some() && args.clear_reminder_minutes {
        return Err(CliError::validation(
            "cannot combine --reminder-minutes with --clear-reminder-minutes",
        ));
    }
    Ok(())
}

fn ensure_title(title: &str) -> Result<(), CliError> {
    if title.trim().is_empty() {
        return Err(CliError::validation("title cannot be empty"));
    }
    Ok(())
}

fn validate_event_datetime(start_at: &str, end_at: &str) -> Result<(), CliError> {
    let start = NaiveDateTime::parse_from_str(start_at, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| CliError::validation("invalid start_at timestamp"))?;
    let end = NaiveDateTime::parse_from_str(end_at, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| CliError::validation("invalid end_at timestamp"))?;
    if end < start {
        return Err(CliError::validation(
            "end_at must be greater than or equal to start_at",
        ));
    }
    Ok(())
}

fn ensure_calendar_exists(conn: &Connection, calendar_id: &str) -> Result<(), CliError> {
    require_resource(
        db::get_calendar(conn, calendar_id).map_err(internal_error)?,
        "calendar",
        calendar_id,
    )?;
    Ok(())
}

fn ensure_project_exists(conn: &Connection, project_id: &str) -> Result<(), CliError> {
    require_resource(
        db::get_project(conn, project_id).map_err(internal_error)?,
        "project",
        project_id,
    )?;
    Ok(())
}

fn ensure_event_exists(conn: &Connection, event_id: &str) -> Result<(), CliError> {
    require_resource(
        db::get_event(conn, event_id).map_err(internal_error)?,
        "event",
        event_id,
    )?;
    Ok(())
}

fn validate_dependency_endpoints(
    conn: &Connection,
    from_event_id: &str,
    to_event_id: &str,
) -> Result<(), CliError> {
    if from_event_id == to_event_id {
        return Err(CliError::validation(
            "dependency endpoints must reference two distinct events",
        ));
    }
    ensure_event_exists(conn, from_event_id)?;
    ensure_event_exists(conn, to_event_id)?;
    Ok(())
}

fn validate_dependency_edge(
    conn: &Connection,
    from_event_id: &str,
    to_event_id: &str,
    dependency_type: models::DependencyType,
    exclude_dependency_id: Option<&str>,
) -> Result<(), CliError> {
    let dependencies = db::load_dependencies(conn).map_err(internal_error)?;
    if dependencies.iter().any(|dependency| {
        Some(dependency.id.as_str()) != exclude_dependency_id
            && dependency.from_event_id == from_event_id
            && dependency.to_event_id == to_event_id
    }) {
        return Err(CliError::conflict("dependency edge already exists"));
    }

    if dependency_type != models::DependencyType::Blocks {
        return Ok(());
    }

    let active_blocks: Vec<_> = dependencies
        .into_iter()
        .filter(|dependency| {
            dependency.dependency_type == models::DependencyType::Blocks
                && Some(dependency.id.as_str()) != exclude_dependency_id
        })
        .collect();
    let mut dag = EventDag::from_dependencies(&active_blocks);
    if dag.add_edge(from_event_id, to_event_id).is_err() {
        return Err(CliError::conflict(
            "dependency would create a cycle in blocks edges",
        ));
    }
    Ok(())
}

fn calendar_source_from_arg(source: CalendarSourceArg) -> models::CalendarSource {
    match source {
        CalendarSourceArg::Local => models::CalendarSource::Local,
        CalendarSourceArg::Google => models::CalendarSource::Google,
    }
}

fn dependency_type_from_arg(dependency_type: DependencyTypeArg) -> models::DependencyType {
    match dependency_type {
        DependencyTypeArg::Blocks => models::DependencyType::Blocks,
        DependencyTypeArg::Related => models::DependencyType::Related,
    }
}
