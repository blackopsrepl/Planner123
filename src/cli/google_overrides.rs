#[derive(Debug, Default, Deserialize)]
struct GoogleSyncOverrideConfig {
    #[serde(default)]
    default: Option<GoogleSyncOverrideResult>,
    #[serde(default)]
    by_calendar_id: std::collections::HashMap<String, GoogleSyncOverrideResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleSyncOverrideResult {
    added: Option<usize>,
    updated: Option<usize>,
    error: Option<String>,
}

type GoogleSyncCounts = (usize, usize);
type MaybeGoogleSyncOverride = Option<Result<GoogleSyncCounts, CliError>>;

fn google_sync_override_result(
    calendar: &models::Calendar,
) -> Result<MaybeGoogleSyncOverride, CliError> {
    let Ok(raw) = std::env::var("PLANNER123_TEST_GOOGLE_SYNC") else {
        return Ok(None);
    };

    let config: GoogleSyncOverrideConfig = serde_json::from_str(&raw)
        .map_err(|e| CliError::internal(format!("invalid google sync override: {}", e)))?;
    let result = config
        .by_calendar_id
        .get(&calendar.id)
        .cloned()
        .or(config.default);

    Ok(result.map(|result| {
        if let Some(error) = result.error {
            Err(CliError::external(error))
        } else {
            Ok((result.added.unwrap_or(0), result.updated.unwrap_or(0)))
        }
    }))
}

#[derive(Debug, Deserialize)]
struct GoogleDiscoveryOverrideItem {
    google_id: String,
    name: String,
    color: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    access_role: Option<String>,
    #[serde(default)]
    writable: bool,
}

fn google_discovery_override(
) -> Result<Option<Vec<google::discovery::DiscoveredGoogleCalendar>>, CliError> {
    let Ok(raw) = std::env::var("PLANNER123_TEST_GOOGLE_DISCOVERY") else {
        return Ok(None);
    };

    let parsed: Vec<GoogleDiscoveryOverrideItem> = serde_json::from_str(&raw)
        .map_err(|e| CliError::internal(format!("invalid google discovery override: {}", e)))?;
    Ok(Some(
        parsed
            .into_iter()
            .map(|calendar| google::discovery::DiscoveredGoogleCalendar {
                google_id: calendar.google_id,
                name: calendar.name,
                color: calendar.color,
                primary: calendar.primary,
                access_role: calendar.access_role,
                writable: calendar.writable,
            })
            .collect(),
    ))
}
