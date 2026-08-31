fn handle_google_with_backend<B: GoogleSyncBackend>(
    conn: &Connection,
    action: GoogleCommand,
    google_sync_backend: &B,
) -> Result<Value, CliError> {
    match action {
        GoogleCommand::Auth { action } => handle_google_auth(action),
        GoogleCommand::Calendars { action } => handle_google_calendars(conn, action),
        GoogleCommand::SyncStatus(args) => handle_google_sync_status(conn, args),
        GoogleCommand::Conflicts { action } => handle_google_conflicts(conn, action),
        GoogleCommand::Sync(args) => {
            let mut calendars: Vec<_> = db::load_calendars(conn)
                .map_err(internal_error)?
                .into_iter()
                .filter(|calendar| calendar.source == models::CalendarSource::Google)
                .collect();

            if let Some(calendar_id) = args.calendar_id.as_deref() {
                calendars.retain(|calendar| calendar.id == calendar_id);
                if calendars.is_empty() {
                    return Err(CliError::not_found("google calendar", calendar_id));
                }
            }

            if calendars.is_empty() {
                return Err(CliError::conflict("no google calendars found to sync"));
            }

            let rt = tokio::runtime::Runtime::new()
                .context("failed to start tokio runtime")
                .map_err(|e| CliError::internal(e.to_string()))?;
            let mut results = Vec::with_capacity(calendars.len());
            let mut total_added = 0usize;
            let mut total_updated = 0usize;
            let mut total_conflicts = 0usize;

            for calendar in &calendars {
                let report = google_sync_backend.sync_calendar(&rt, conn, calendar)?;
                total_added += report.events_added;
                total_updated += report.events_updated + report.pushed_updates;
                total_conflicts += report.conflicts_detected;
                results.push(SyncCalendarData {
                    calendar_id: calendar.id.clone(),
                    calendar_name: calendar.name.clone(),
                    google_id: calendar.google_id.clone(),
                    events_added: report.events_added,
                    events_updated: report.events_updated + report.pushed_updates,
                    pushed_creates: report.pushed_creates,
                    pushed_updates: report.pushed_updates,
                    pushed_deletes: report.pushed_deletes,
                    conflicts_detected: report.conflicts_detected,
                });
            }

            Ok(json!({
                "calendars_synced": results.len(),
                "events_added": total_added,
                "events_updated": total_updated,
                "conflicts_detected": total_conflicts,
                "results": results,
            }))
        }
    }
}

fn handle_google_auth(action: GoogleAuthCommand) -> Result<Value, CliError> {
    match action {
        GoogleAuthCommand::Status => Ok(json!(google::auth::auth_status())),
        GoogleAuthCommand::Login(args) => {
            let saved = google::auth::GoogleClient::saved_credentials();
            let client_id = args
                .client_id
                .or_else(|| saved.as_ref().map(|saved| saved.client_id.clone()))
                .ok_or_else(|| {
                    CliError::validation(
                        "google auth login requires --client-id or saved credentials",
                    )
                })?;
            let client_secret = args
                .client_secret
                .or_else(|| saved.as_ref().and_then(|saved| saved.client_secret.clone()));
            let runtime = google_runtime()?;
            runtime
                .block_on(google::auth::authorize_and_persist(
                    &client_id,
                    client_secret.as_deref(),
                ))
                .map_err(|e| CliError::external(e.to_string()))?;
            Ok(json!(google::auth::auth_status()))
        }
        GoogleAuthCommand::Logout => {
            let status = google::auth::GoogleClient::logout().map_err(internal_error)?;
            Ok(json!(status))
        }
    }
}

fn handle_google_calendars(
    conn: &Connection,
    action: GoogleCalendarCommand,
) -> Result<Value, CliError> {
    match action {
        GoogleCalendarCommand::Discover => {
            let discovered = discover_google_calendars()?;
            let imported_google_ids = db::load_calendars(conn)
                .map_err(internal_error)?
                .into_iter()
                .filter(|calendar| calendar.source == models::CalendarSource::Google)
                .filter_map(|calendar| calendar.google_id)
                .collect::<std::collections::HashSet<_>>();
            let results = discovered
                .into_iter()
                .map(|calendar| GoogleCalendarDiscoveryData {
                    imported: imported_google_ids.contains(&calendar.google_id),
                    google_id: calendar.google_id,
                    name: calendar.name,
                    color: calendar.color,
                    primary: calendar.primary,
                    access_role: calendar.access_role,
                    writable: calendar.writable,
                })
                .collect::<Vec<_>>();
            Ok(json!(results))
        }
        GoogleCalendarCommand::Import(args) => {
            let calendar = discover_google_calendars()?
                .into_iter()
                .find(|calendar| calendar.google_id == args.google_id)
                .ok_or_else(|| CliError::not_found("google calendar", &args.google_id))?;
            let imported = calendar_service::import_google_calendar(conn, &calendar, args.position)
                .map_err(calendar_service_error)?;
            let sync_state = crate::sync::state::load_calendar_sync_state(conn, &imported.id)
                .map_err(internal_error)?;
            Ok(json!({
                "calendar": imported,
                "sync_state": sync_state,
            }))
        }
    }
}

fn handle_google_sync_status(
    conn: &Connection,
    args: GoogleSyncStatusArgs,
) -> Result<Value, CliError> {
    let mut calendars: Vec<_> = db::load_calendars(conn)
        .map_err(internal_error)?
        .into_iter()
        .filter(|calendar| calendar.source == models::CalendarSource::Google)
        .collect();
    if let Some(calendar_id) = args.calendar_id.as_deref() {
        calendars.retain(|calendar| calendar.id == calendar_id);
        if calendars.is_empty() {
            return Err(CliError::not_found("google calendar", calendar_id));
        }
    }

    let status = calendars
        .iter()
        .map(|calendar| crate::sync::engine::sync_status(conn, calendar).map_err(internal_error))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!(status))
}

fn handle_google_conflicts(
    conn: &Connection,
    action: GoogleConflictCommand,
) -> Result<Value, CliError> {
    match action {
        GoogleConflictCommand::List => {
            let conflicts =
                crate::sync::conflicts::list_pending_conflicts(conn).map_err(internal_error)?;
            Ok(json!(conflicts))
        }
        GoogleConflictCommand::Resolve(args) => {
            let strategy = match args.strategy {
                ConflictStrategyArg::KeepLocal => {
                    crate::sync::state::ConflictResolutionStrategy::KeepLocal
                }
                ConflictStrategyArg::KeepRemote => {
                    crate::sync::state::ConflictResolutionStrategy::KeepRemote
                }
            };
            let resolved = crate::sync::conflicts::resolve_conflict(conn, &args.id, strategy)
                .map_err(|error| {
                    if error.to_string().contains("not found") {
                        CliError::not_found("conflict", &args.id)
                    } else {
                        CliError::internal(error.to_string())
                    }
                })?;
            Ok(json!(resolved))
        }
    }
}
