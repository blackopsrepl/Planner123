fn handle_calendars(conn: &Connection, action: CalendarCommand) -> Result<Value, CliError> {
    match action {
        CalendarCommand::List => Ok(json!(db::load_calendars(conn).map_err(internal_error)?)),
        CalendarCommand::Get { id } => Ok(json!(require_resource(
            db::get_calendar(conn, &id).map_err(internal_error)?,
            "calendar",
            &id
        )?)),
        CalendarCommand::Create(args) => {
            let calendar = calendar_service::create_calendar(
                conn,
                CreateCalendarInput {
                    name: args.name,
                    color: args.color,
                    source: calendar_source_from_arg(args.source),
                    google_id: args.google_id,
                    visible: args.visible,
                    position: Some(args.position),
                },
            )
            .map_err(calendar_service_error)?;
            Ok(json!(calendar))
        }
        CalendarCommand::Update(args) => {
            let google_id = if args.google_id.is_some() {
                Some(normalize_optional(args.google_id))
            } else {
                None
            };

            let calendar = calendar_service::update_calendar(
                conn,
                UpdateCalendarInput {
                    id: args.id,
                    name: args.name,
                    color: args.color,
                    source: args.source.map(calendar_source_from_arg),
                    google_id,
                    visible: args.visible,
                    position: args.position,
                },
            )
            .map_err(calendar_service_error)?;
            Ok(json!(calendar))
        }
        CalendarCommand::Delete(args) => {
            let tx = conn.unchecked_transaction().map_err(internal_error)?;
            require_resource(
                db::get_calendar(&tx, &args.id).map_err(internal_error)?,
                "calendar",
                &args.id,
            )?;
            if db::count_active_calendars(&tx).map_err(internal_error)? <= 1 {
                return Err(CliError::conflict("cannot delete the last active calendar"));
            }
            let active_events =
                db::count_active_events_for_calendar(&tx, &args.id).map_err(internal_error)?;
            if active_events > 0 && !args.cascade_events {
                return Err(CliError::conflict(
                    "calendar has active events; rerun with --cascade-events to delete them too",
                ));
            }
            if args.cascade_events {
                for event_id in
                    db::load_active_event_ids_for_calendar(&tx, &args.id).map_err(internal_error)?
                {
                    db::soft_delete_event(&tx, &event_id).map_err(internal_error)?;
                }
            }
            db::soft_delete_calendar(&tx, &args.id).map_err(internal_error)?;
            db::delete_sync_token(&tx, &args.id).map_err(internal_error)?;
            tx.commit().map_err(internal_error)?;
            Ok(json!(DeleteData {
                resource: "calendar",
                id: args.id,
            }))
        }
    }
}

fn handle_projects(conn: &Connection, action: ProjectCommand) -> Result<Value, CliError> {
    match action {
        ProjectCommand::List => Ok(json!(db::load_projects(conn).map_err(internal_error)?)),
        ProjectCommand::Get { id } => Ok(json!(require_resource(
            db::get_project(conn, &id).map_err(internal_error)?,
            "project",
            &id
        )?)),
        ProjectCommand::Create(args) => {
            let now = timestamp_now();
            let project = models::Project {
                id: Uuid::new_v4().to_string(),
                name: non_empty(args.name, "name")?,
                color: non_empty(args.color, "color")?,
                description: normalize_optional(args.description),
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            };
            db::insert_project(conn, &project).map_err(internal_error)?;
            Ok(json!(project))
        }
        ProjectCommand::Update(args) => {
            let mut project = require_resource(
                db::get_project(conn, &args.id).map_err(internal_error)?,
                "project",
                &args.id,
            )?;
            if let Some(name) = args.name {
                project.name = non_empty(name, "name")?;
            }
            if let Some(color) = args.color {
                project.color = non_empty(color, "color")?;
            }
            if args.description.is_some() {
                project.description = normalize_optional(args.description);
            }
            db::update_project(conn, &project).map_err(internal_error)?;
            Ok(json!(require_resource(
                db::get_project(conn, &args.id).map_err(internal_error)?,
                "project",
                &args.id,
            )?))
        }
        ProjectCommand::Delete(args) => {
            require_resource(
                db::get_project(conn, &args.id).map_err(internal_error)?,
                "project",
                &args.id,
            )?;
            let active_events =
                db::count_active_events_for_project(conn, &args.id).map_err(internal_error)?;
            if active_events > 0 && !args.detach_events {
                return Err(CliError::conflict(
                    "project has active events; rerun with --detach-events to clear project_id first",
                ));
            }

            let tx = conn.unchecked_transaction().map_err(internal_error)?;
            if args.detach_events {
                db::clear_project_id_for_project(&tx, &args.id).map_err(internal_error)?;
            }
            db::soft_delete_project(&tx, &args.id).map_err(internal_error)?;
            tx.commit().map_err(internal_error)?;
            Ok(json!(DeleteData {
                resource: "project",
                id: args.id,
            }))
        }
    }
}
