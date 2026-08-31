fn handle_events(conn: &Connection, action: EventCommand) -> Result<Value, CliError> {
    match action {
        EventCommand::List(args) => {
            let events = match (args.from, args.to) {
                (Some(from), Some(to)) => {
                    db::load_events_in_range(conn, &from, &to).map_err(internal_error)?
                }
                (None, None) => db::load_events(conn).map_err(internal_error)?,
                _ => unreachable!("clap enforces paired args"),
            };
            Ok(json!(events))
        }
        EventCommand::Get { id } => Ok(json!(require_resource(
            db::get_event(conn, &id).map_err(internal_error)?,
            "event",
            &id
        )?)),
        EventCommand::Create(args) => {
            ensure_calendar_exists(conn, &args.calendar_id)?;
            if let Some(project_id) = args.project_id.as_deref() {
                ensure_project_exists(conn, project_id)?;
            }
            ensure_title(&args.title)?;
            validate_event_datetime(&args.start_at, &args.end_at)?;
            let now = timestamp_now();
            let event = models::Event {
                id: Uuid::new_v4().to_string(),
                calendar_id: args.calendar_id,
                project_id: normalize_optional(args.project_id),
                title: args.title.trim().to_string(),
                description: normalize_optional(args.description),
                location: normalize_optional(args.location),
                start_at: args.start_at,
                end_at: args.end_at,
                all_day: args.all_day,
                rrule: normalize_optional(args.rrule),
                google_id: None,
                google_etag: None,
                reminder_minutes: args.reminder_minutes,
                timezone: args
                    .timezone
                    .map(|timezone| non_empty(timezone, "timezone"))
                    .transpose()?
                    .unwrap_or_else(crate::time::local_timezone_name),
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            };
            let event =
                event_service::save_event(conn, event, true).map_err(event_service_error)?;
            Ok(json!(event))
        }
        EventCommand::Update(args) => {
            validate_event_update_args(&args)?;
            let mut event = require_resource(
                db::get_event(conn, &args.id).map_err(internal_error)?,
                "event",
                &args.id,
            )?;

            if let Some(calendar_id) = args.calendar_id {
                ensure_calendar_exists(conn, &calendar_id)?;
                event.calendar_id = calendar_id;
            }
            if let Some(title) = args.title {
                ensure_title(&title)?;
                event.title = title.trim().to_string();
            }
            if args.clear_project_id {
                event.project_id = None;
            } else if let Some(project_id) = args.project_id {
                ensure_project_exists(conn, &project_id)?;
                event.project_id = Some(project_id);
            }
            if args.clear_description {
                event.description = None;
            } else if args.description.is_some() {
                event.description = normalize_optional(args.description);
            }
            if args.clear_location {
                event.location = None;
            } else if args.location.is_some() {
                event.location = normalize_optional(args.location);
            }
            if let Some(start_at) = args.start_at {
                event.start_at = start_at;
            }
            if let Some(end_at) = args.end_at {
                event.end_at = end_at;
            }
            if let Some(all_day) = args.all_day {
                event.all_day = all_day;
            }
            if args.clear_rrule {
                event.rrule = None;
            } else if args.rrule.is_some() {
                event.rrule = normalize_optional(args.rrule);
            }
            if args.clear_reminder_minutes {
                event.reminder_minutes = None;
            } else if let Some(reminder_minutes) = args.reminder_minutes {
                event.reminder_minutes = Some(reminder_minutes);
            }
            if let Some(timezone) = args.timezone {
                event.timezone = non_empty(timezone, "timezone")?;
            }

            validate_event_datetime(&event.start_at, &event.end_at)?;
            let event =
                event_service::save_event(conn, event, false).map_err(event_service_error)?;
            Ok(json!(event))
        }
        EventCommand::Delete { id } => {
            event_service::delete_event(conn, &id).map_err(event_service_error)?;
            Ok(json!(DeleteData {
                resource: "event",
                id,
            }))
        }
    }
}

fn handle_dependencies(conn: &Connection, action: DependencyCommand) -> Result<Value, CliError> {
    match action {
        DependencyCommand::List => Ok(json!(db::load_dependencies(conn).map_err(internal_error)?)),
        DependencyCommand::Get { id } => Ok(json!(require_resource(
            db::get_dependency(conn, &id).map_err(internal_error)?,
            "dependency",
            &id,
        )?)),
        DependencyCommand::Create(args) => {
            validate_dependency_endpoints(conn, &args.from_event_id, &args.to_event_id)?;
            validate_dependency_edge(
                conn,
                &args.from_event_id,
                &args.to_event_id,
                dependency_type_from_arg(args.dependency_type),
                None,
            )?;
            let now = timestamp_now();
            let dependency = models::EventDependency {
                id: Uuid::new_v4().to_string(),
                from_event_id: args.from_event_id,
                to_event_id: args.to_event_id,
                dependency_type: dependency_type_from_arg(args.dependency_type),
                created_at: now.clone(),
                updated_at: now,
            };
            db::insert_dependency(conn, &dependency).map_err(internal_error)?;
            Ok(json!(dependency))
        }
        DependencyCommand::Update(args) => {
            let mut dependency = require_resource(
                db::get_dependency(conn, &args.id).map_err(internal_error)?,
                "dependency",
                &args.id,
            )?;
            if let Some(from_event_id) = args.from_event_id {
                dependency.from_event_id = from_event_id;
            }
            if let Some(to_event_id) = args.to_event_id {
                dependency.to_event_id = to_event_id;
            }
            if let Some(dependency_type) = args.dependency_type {
                dependency.dependency_type = dependency_type_from_arg(dependency_type);
            }
            validate_dependency_endpoints(
                conn,
                &dependency.from_event_id,
                &dependency.to_event_id,
            )?;
            validate_dependency_edge(
                conn,
                &dependency.from_event_id,
                &dependency.to_event_id,
                dependency.dependency_type.clone(),
                Some(dependency.id.as_str()),
            )?;
            db::update_dependency(conn, &dependency).map_err(internal_error)?;
            Ok(json!(require_resource(
                db::get_dependency(conn, &args.id).map_err(internal_error)?,
                "dependency",
                &args.id,
            )?))
        }
        DependencyCommand::Delete { id } => {
            require_resource(
                db::get_dependency(conn, &id).map_err(internal_error)?,
                "dependency",
                &id,
            )?;
            db::delete_dependency(conn, &id).map_err(internal_error)?;
            Ok(json!(DeleteData {
                resource: "dependency",
                id,
            }))
        }
    }
}
