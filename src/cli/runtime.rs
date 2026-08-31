#[derive(Debug, Clone)]
pub struct CliError {
    pub code: &'static str,
    pub message: String,
}

impl CliError {
    fn validation(message: impl Into<String>) -> Self {
        Self {
            code: "validation_error",
            message: message.into(),
        }
    }

    fn not_found(resource: &'static str, id: &str) -> Self {
        Self {
            code: "not_found",
            message: format!("{} '{}' not found", resource, id),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "conflict",
            message: message.into(),
        }
    }

    fn external(message: impl Into<String>) -> Self {
        Self {
            code: "external_error",
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_error",
            message: message.into(),
        }
    }

    pub fn invalid_arguments(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_arguments",
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorPayload<'a> {
    status: &'static str,
    code: &'a str,
    message: &'a str,
}

#[derive(Debug, Serialize)]
struct SuccessPayload<T> {
    status: &'static str,
    data: T,
}

#[derive(Debug, Serialize)]
struct DeleteData<'a> {
    resource: &'a str,
    id: String,
}

#[derive(Debug, Serialize)]
struct SyncCalendarData {
    calendar_id: String,
    calendar_name: String,
    google_id: Option<String>,
    events_added: usize,
    events_updated: usize,
    pushed_creates: usize,
    pushed_updates: usize,
    pushed_deletes: usize,
    conflicts_detected: usize,
}

#[derive(Debug, Serialize)]
struct GoogleCalendarDiscoveryData {
    google_id: String,
    name: String,
    color: String,
    primary: bool,
    access_role: Option<String>,
    writable: bool,
    imported: bool,
}

pub fn error_value(err: &CliError) -> Value {
    serde_json::to_value(ErrorPayload {
        status: "error",
        code: err.code,
        message: &err.message,
    })
    .expect("serializable error payload")
}

pub fn execute(cli: Cli) -> Result<Value, CliError> {
    let conn = db::open().map_err(|e| CliError::internal(e.to_string()))?;
    execute_with_connection(&conn, cli)
}

pub fn execute_with_connection(conn: &Connection, cli: Cli) -> Result<Value, CliError> {
    execute_with_backend(conn, cli, &RealGoogleSyncBackend)
}

fn execute_with_backend<B: GoogleSyncBackend>(
    conn: &Connection,
    cli: Cli,
    google_sync_backend: &B,
) -> Result<Value, CliError> {
    let data = match cli.command {
        Command::Calendars { action } => handle_calendars(conn, action)?,
        Command::Projects { action } => handle_projects(conn, action)?,
        Command::Events { action } => handle_events(conn, action)?,
        Command::Dependencies { action } => handle_dependencies(conn, action)?,
        Command::Tasks { action } => handle_tasks(conn, action)?,
        Command::Planner { action } => handle_planner(conn, action)?,
        Command::Google { action } => {
            handle_google_with_backend(conn, action, google_sync_backend)?
        }
        Command::Ical { action } => handle_ical(conn, action)?,
    };
    Ok(success_value(data))
}

fn success_value<T: Serialize>(data: T) -> Value {
    serde_json::to_value(SuccessPayload { status: "ok", data }).expect("serializable success")
}

fn parse_timestamp_arg(value: &str) -> Result<String, String> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .map_err(|_| {
            format!(
                "invalid timestamp '{}'; expected YYYY-MM-DD HH:MM:SS",
                value
            )
        })
}

fn parse_availability_args(values: Vec<String>) -> Result<Option<planner::Availability>, CliError> {
    if values.is_empty() {
        return Ok(None);
    }
    let mut days: BTreeMap<String, Vec<planner::TimeWindow>> = BTreeMap::new();
    for value in values {
        let (day, window) = value.split_once('=').ok_or_else(|| {
            CliError::invalid_arguments(format!(
                "invalid --availability '{}'; expected DAY=HH:MM-HH:MM",
                value
            ))
        })?;
        let (start, end) = window.split_once('-').ok_or_else(|| {
            CliError::invalid_arguments(format!(
                "invalid --availability '{}'; expected DAY=HH:MM-HH:MM",
                value
            ))
        })?;
        let weekday = day.to_ascii_lowercase();
        days.entry(weekday).or_default().push(planner::TimeWindow {
            start: start.into(),
            end: end.into(),
        });
    }
    Ok(Some(planner::Availability(days)))
}
