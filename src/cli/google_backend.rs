fn handle_ical(conn: &Connection, action: IcalCommand) -> Result<Value, CliError> {
    match action {
        IcalCommand::Import(args) => {
            ensure_calendar_exists(conn, &args.calendar_id)?;
            let timezone = args
                .timezone
                .map(|value| non_empty(value, "timezone"))
                .transpose()?
                .unwrap_or_else(crate::time::local_timezone_name);
            let report = crate::ical::import_from_file(
                conn,
                &args.calendar_id,
                std::path::Path::new(&args.path),
                &timezone,
            )
            .map_err(internal_error)?;
            Ok(json!(report))
        }
    }
}

fn discover_google_calendars() -> Result<Vec<google::discovery::DiscoveredGoogleCalendar>, CliError>
{
    if let Some(discovered) = google_discovery_override()? {
        return Ok(discovered);
    }

    let client = google::auth::GoogleClient::from_keyring()
        .ok_or_else(|| CliError::external("google credentials are not configured in keyring"))?;
    let runtime = google_runtime()?;
    runtime
        .block_on(google::discovery::discover_calendars(&client))
        .map_err(|e| CliError::external(e.to_string()))
}

fn google_runtime() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Runtime::new()
        .context("failed to start tokio runtime")
        .map_err(|e| CliError::internal(e.to_string()))
}

trait GoogleSyncBackend {
    fn sync_calendar(
        &self,
        runtime: &tokio::runtime::Runtime,
        conn: &Connection,
        calendar: &models::Calendar,
    ) -> Result<google::sync::SyncCalendarReport, CliError>;
}

struct RealGoogleSyncBackend;

impl GoogleSyncBackend for RealGoogleSyncBackend {
    fn sync_calendar(
        &self,
        runtime: &tokio::runtime::Runtime,
        conn: &Connection,
        calendar: &models::Calendar,
    ) -> Result<google::sync::SyncCalendarReport, CliError> {
        if let Some(result) = google_sync_override_result(calendar)? {
            let (added, updated) = result?;
            return Ok(google::sync::SyncCalendarReport {
                events_added: added,
                events_updated: updated,
                ..Default::default()
            });
        }
        let client = google::auth::GoogleClient::from_keyring().ok_or_else(|| {
            CliError::external("google credentials are not configured in keyring")
        })?;
        runtime
            .block_on(google::sync::sync_calendar(&client, calendar))
            .with_context(|| format!("sync failed for calendar '{}'", calendar.name))
            .map_err(|e| {
                let _ =
                    crate::sync::engine::mark_calendar_sync_error(conn, calendar, &e.to_string());
                CliError::external(e.to_string())
            })
    }
}
