use anyhow::Context;
use chrono::NaiveDateTime;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    calendar_service::{self, CreateCalendarInput, UpdateCalendarInput},
    dag::EventDag,
    db, event_service, google, models,
};

#[derive(Debug, Parser)]
#[command(name = "solverforge-calendar-cli", about = "JSON-first automation CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Calendars {
        #[command(subcommand)]
        action: CalendarCommand,
    },
    Projects {
        #[command(subcommand)]
        action: ProjectCommand,
    },
    Events {
        #[command(subcommand)]
        action: EventCommand,
    },
    Dependencies {
        #[command(subcommand)]
        action: DependencyCommand,
    },
    Google {
        #[command(subcommand)]
        action: GoogleCommand,
    },
    Ical {
        #[command(subcommand)]
        action: IcalCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    List,
    Get { id: String },
    Create(CalendarCreateArgs),
    Update(CalendarUpdateArgs),
    Delete(CalendarDeleteArgs),
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    List,
    Get { id: String },
    Create(ProjectCreateArgs),
    Update(ProjectUpdateArgs),
    Delete(ProjectDeleteArgs),
}

#[derive(Debug, Subcommand)]
pub enum EventCommand {
    List(EventListArgs),
    Get { id: String },
    Create(EventCreateArgs),
    Update(EventUpdateArgs),
    Delete { id: String },
}

#[derive(Debug, Subcommand)]
pub enum DependencyCommand {
    List,
    Get { id: String },
    Create(DependencyCreateArgs),
    Update(DependencyUpdateArgs),
    Delete { id: String },
}

#[derive(Debug, Subcommand)]
pub enum GoogleCommand {
    Auth {
        #[command(subcommand)]
        action: GoogleAuthCommand,
    },
    Calendars {
        #[command(subcommand)]
        action: GoogleCalendarCommand,
    },
    Sync(GoogleSyncArgs),
    SyncStatus(GoogleSyncStatusArgs),
    Conflicts {
        #[command(subcommand)]
        action: GoogleConflictCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum IcalCommand {
    Import(IcalImportArgs),
}

#[derive(Debug, Subcommand)]
pub enum GoogleAuthCommand {
    Status,
    Login(GoogleAuthLoginArgs),
    Logout,
}

#[derive(Debug, Subcommand)]
pub enum GoogleCalendarCommand {
    Discover,
    Import(GoogleCalendarImportArgs),
}

#[derive(Debug, Subcommand)]
pub enum GoogleConflictCommand {
    List,
    Resolve(GoogleConflictResolveArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CalendarSourceArg {
    Local,
    Google,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DependencyTypeArg {
    Blocks,
    Related,
}

#[derive(Debug, Args)]
pub struct CalendarCreateArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    color: String,
    #[arg(long, value_enum, default_value_t = CalendarSourceArg::Local)]
    source: CalendarSourceArg,
    #[arg(long)]
    google_id: Option<String>,
    #[arg(long, default_value_t = true, value_parser = clap::builder::BoolishValueParser::new())]
    visible: bool,
    #[arg(long, default_value_t = 0)]
    position: i64,
}

#[derive(Debug, Args)]
pub struct CalendarUpdateArgs {
    id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    color: Option<String>,
    #[arg(long, value_enum)]
    source: Option<CalendarSourceArg>,
    #[arg(long)]
    google_id: Option<String>,
    #[arg(long, value_parser = clap::builder::BoolishValueParser::new())]
    visible: Option<bool>,
    #[arg(long)]
    position: Option<i64>,
}

#[derive(Debug, Args)]
pub struct CalendarDeleteArgs {
    id: String,
    #[arg(long)]
    cascade_events: bool,
}

#[derive(Debug, Args)]
pub struct ProjectCreateArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    color: String,
    #[arg(long)]
    description: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProjectUpdateArgs {
    id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    color: Option<String>,
    #[arg(long)]
    description: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProjectDeleteArgs {
    id: String,
    #[arg(long)]
    detach_events: bool,
}

#[derive(Debug, Args)]
pub struct EventListArgs {
    #[arg(long, requires = "to", value_parser = parse_timestamp_arg)]
    from: Option<String>,
    #[arg(long, requires = "from", value_parser = parse_timestamp_arg)]
    to: Option<String>,
}

#[derive(Debug, Args)]
pub struct EventCreateArgs {
    #[arg(long)]
    calendar_id: String,
    #[arg(long)]
    title: String,
    #[arg(long)]
    project_id: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    location: Option<String>,
    #[arg(long, value_parser = parse_timestamp_arg)]
    start_at: String,
    #[arg(long, value_parser = parse_timestamp_arg)]
    end_at: String,
    #[arg(long, default_value_t = false, value_parser = clap::builder::BoolishValueParser::new())]
    all_day: bool,
    #[arg(long)]
    rrule: Option<String>,
    #[arg(long)]
    reminder_minutes: Option<i64>,
    #[arg(long)]
    timezone: Option<String>,
}

#[derive(Debug, Args)]
pub struct EventUpdateArgs {
    id: String,
    #[arg(long)]
    calendar_id: Option<String>,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    project_id: Option<String>,
    #[arg(long)]
    clear_project_id: bool,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    clear_description: bool,
    #[arg(long)]
    location: Option<String>,
    #[arg(long)]
    clear_location: bool,
    #[arg(long, value_parser = parse_timestamp_arg)]
    start_at: Option<String>,
    #[arg(long, value_parser = parse_timestamp_arg)]
    end_at: Option<String>,
    #[arg(long, value_parser = clap::builder::BoolishValueParser::new())]
    all_day: Option<bool>,
    #[arg(long)]
    rrule: Option<String>,
    #[arg(long)]
    clear_rrule: bool,
    #[arg(long)]
    reminder_minutes: Option<i64>,
    #[arg(long)]
    clear_reminder_minutes: bool,
    #[arg(long)]
    timezone: Option<String>,
}

#[derive(Debug, Args)]
pub struct DependencyCreateArgs {
    #[arg(long)]
    from_event_id: String,
    #[arg(long)]
    to_event_id: String,
    #[arg(long, value_enum, default_value_t = DependencyTypeArg::Blocks)]
    dependency_type: DependencyTypeArg,
}

#[derive(Debug, Args)]
pub struct DependencyUpdateArgs {
    id: String,
    #[arg(long)]
    from_event_id: Option<String>,
    #[arg(long)]
    to_event_id: Option<String>,
    #[arg(long, value_enum)]
    dependency_type: Option<DependencyTypeArg>,
}

#[derive(Debug, Args)]
pub struct GoogleSyncArgs {
    #[arg(long)]
    calendar_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct GoogleAuthLoginArgs {
    #[arg(long)]
    client_id: Option<String>,
    #[arg(long)]
    client_secret: Option<String>,
}

#[derive(Debug, Args)]
pub struct GoogleCalendarImportArgs {
    #[arg(long)]
    google_id: String,
    #[arg(long)]
    position: Option<i64>,
}

#[derive(Debug, Args)]
pub struct GoogleSyncStatusArgs {
    #[arg(long)]
    calendar_id: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConflictStrategyArg {
    KeepLocal,
    KeepRemote,
}

#[derive(Debug, Args)]
pub struct GoogleConflictResolveArgs {
    id: String,
    #[arg(long, value_enum)]
    strategy: ConflictStrategyArg,
}

#[derive(Debug, Args)]
pub struct IcalImportArgs {
    #[arg(long)]
    calendar_id: String,
    #[arg(long)]
    path: String,
    #[arg(long)]
    timezone: Option<String>,
}

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

fn execute_with_backend(
    conn: &Connection,
    cli: Cli,
    google_sync_backend: &dyn GoogleSyncBackend,
) -> Result<Value, CliError> {
    let data = match cli.command {
        Command::Calendars { action } => handle_calendars(conn, action)?,
        Command::Projects { action } => handle_projects(conn, action)?,
        Command::Events { action } => handle_events(conn, action)?,
        Command::Dependencies { action } => handle_dependencies(conn, action)?,
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

fn handle_google_with_backend(
    conn: &Connection,
    action: GoogleCommand,
    google_sync_backend: &dyn GoogleSyncBackend,
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
    let Ok(raw) = std::env::var("SOLVERFORGE_CALENDAR_TEST_GOOGLE_SYNC") else {
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
    let Ok(raw) = std::env::var("SOLVERFORGE_CALENDAR_TEST_GOOGLE_DISCOVERY") else {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use tempfile::TempDir;

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");
        let conn = db::open_at(&db_path).unwrap();
        (temp, conn)
    }

    fn seed_calendar(conn: &Connection, name: &str) -> models::Calendar {
        let now = timestamp_now();
        let calendar = models::Calendar {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            color: "#82FB9C".to_string(),
            source: models::CalendarSource::Local,
            google_id: None,
            visible: true,
            position: 0,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        };
        db::insert_calendar(conn, &calendar).unwrap();
        calendar
    }

    fn seed_project(conn: &Connection, name: &str) -> models::Project {
        let now = timestamp_now();
        let project = models::Project {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            color: "#ffaa00".to_string(),
            description: None,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        };
        db::insert_project(conn, &project).unwrap();
        project
    }

    fn seed_event(
        conn: &Connection,
        calendar_id: &str,
        project_id: Option<String>,
        title: &str,
    ) -> models::Event {
        let now = timestamp_now();
        let event = models::Event {
            id: Uuid::new_v4().to_string(),
            calendar_id: calendar_id.to_string(),
            project_id,
            title: title.to_string(),
            description: None,
            location: None,
            start_at: "2026-03-30 09:00:00".to_string(),
            end_at: "2026-03-30 10:00:00".to_string(),
            all_day: false,
            rrule: None,
            google_id: None,
            google_etag: None,
            reminder_minutes: None,
            timezone: "UTC".to_string(),
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        };
        db::insert_event(conn, &event).unwrap();
        event
    }

    struct FakeGoogleSyncBackend {
        default_result: Result<(usize, usize), CliError>,
        results_by_calendar_id: HashMap<String, Result<(usize, usize), CliError>>,
    }

    impl FakeGoogleSyncBackend {
        fn with_default(result: Result<(usize, usize), CliError>) -> Self {
            Self {
                default_result: result,
                results_by_calendar_id: HashMap::new(),
            }
        }

        fn with_calendar_result(
            mut self,
            calendar_id: &str,
            result: Result<(usize, usize), CliError>,
        ) -> Self {
            self.results_by_calendar_id
                .insert(calendar_id.to_string(), result);
            self
        }
    }

    impl GoogleSyncBackend for FakeGoogleSyncBackend {
        fn sync_calendar(
            &self,
            _runtime: &tokio::runtime::Runtime,
            _conn: &Connection,
            calendar: &models::Calendar,
        ) -> Result<google::sync::SyncCalendarReport, CliError> {
            let (added, updated) = self
                .results_by_calendar_id
                .get(&calendar.id)
                .cloned()
                .unwrap_or_else(|| self.default_result.clone())?;
            Ok(google::sync::SyncCalendarReport {
                events_added: added,
                events_updated: updated,
                ..Default::default()
            })
        }
    }

    #[test]
    fn blocks_cycle_is_rejected() {
        let (_temp, conn) = open_test_db();
        let calendar = seed_calendar(&conn, "Work");
        let event_a = seed_event(&conn, &calendar.id, None, "A");
        let event_b = seed_event(&conn, &calendar.id, None, "B");

        let first = Cli {
            command: Command::Dependencies {
                action: DependencyCommand::Create(DependencyCreateArgs {
                    from_event_id: event_a.id.clone(),
                    to_event_id: event_b.id.clone(),
                    dependency_type: DependencyTypeArg::Blocks,
                }),
            },
        };
        execute_with_connection(&conn, first).unwrap();

        let second = Cli {
            command: Command::Dependencies {
                action: DependencyCommand::Create(DependencyCreateArgs {
                    from_event_id: event_b.id.clone(),
                    to_event_id: event_a.id.clone(),
                    dependency_type: DependencyTypeArg::Blocks,
                }),
            },
        };
        let err = execute_with_connection(&conn, second).unwrap_err();
        assert_eq!(err.code, "conflict");
    }

    #[test]
    fn project_delete_requires_detach_flag() {
        let (_temp, conn) = open_test_db();
        let calendar = seed_calendar(&conn, "Work");
        let project = seed_project(&conn, "Launch");
        seed_event(&conn, &calendar.id, Some(project.id.clone()), "Milestone");

        let cli = Cli {
            command: Command::Projects {
                action: ProjectCommand::Delete(ProjectDeleteArgs {
                    id: project.id.clone(),
                    detach_events: false,
                }),
            },
        };
        let err = execute_with_connection(&conn, cli).unwrap_err();
        assert_eq!(err.code, "conflict");
    }

    #[test]
    fn calendar_delete_rejects_last_active_calendar() {
        let (_temp, conn) = open_test_db();
        let only_calendar = db::load_calendars(&conn)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let cli = Cli {
            command: Command::Calendars {
                action: CalendarCommand::Delete(CalendarDeleteArgs {
                    id: only_calendar.id,
                    cascade_events: false,
                }),
            },
        };
        let err = execute_with_connection(&conn, cli).unwrap_err();
        assert_eq!(err.code, "conflict");
        assert_eq!(err.message, "cannot delete the last active calendar");
    }

    #[test]
    fn project_delete_with_detach_clears_project_id() {
        let (_temp, conn) = open_test_db();
        let calendar = seed_calendar(&conn, "Work");
        let project = seed_project(&conn, "Launch");
        let event = seed_event(&conn, &calendar.id, Some(project.id.clone()), "Milestone");

        let cli = Cli {
            command: Command::Projects {
                action: ProjectCommand::Delete(ProjectDeleteArgs {
                    id: project.id.clone(),
                    detach_events: true,
                }),
            },
        };
        execute_with_connection(&conn, cli).unwrap();

        let updated = db::get_event(&conn, &event.id).unwrap().unwrap();
        assert_eq!(updated.project_id, None);
        assert!(db::get_project(&conn, &project.id).unwrap().is_none());
    }

    #[test]
    fn event_update_conflicting_flags_are_rejected() {
        let (_temp, conn) = open_test_db();
        let calendar = seed_calendar(&conn, "Work");
        let event = seed_event(&conn, &calendar.id, None, "Milestone");

        let cli = Cli {
            command: Command::Events {
                action: EventCommand::Update(EventUpdateArgs {
                    id: event.id,
                    calendar_id: None,
                    title: None,
                    project_id: None,
                    clear_project_id: false,
                    description: Some("new".to_string()),
                    clear_description: true,
                    location: None,
                    clear_location: false,
                    start_at: None,
                    end_at: None,
                    all_day: None,
                    rrule: None,
                    clear_rrule: false,
                    reminder_minutes: None,
                    clear_reminder_minutes: false,
                    timezone: None,
                }),
            },
        };

        let err = execute_with_connection(&conn, cli).unwrap_err();
        assert_eq!(err.code, "validation_error");
    }

    #[test]
    fn google_sync_without_google_calendars_returns_conflict() {
        let (_temp, conn) = open_test_db();
        let cli = Cli {
            command: Command::Google {
                action: GoogleCommand::Sync(GoogleSyncArgs { calendar_id: None }),
            },
        };

        let backend = FakeGoogleSyncBackend::with_default(Ok((0, 0)));
        let err = execute_with_backend(&conn, cli, &backend).unwrap_err();
        assert_eq!(err.code, "conflict");
        assert_eq!(err.message, "no google calendars found to sync");
    }

    #[test]
    fn google_sync_reports_missing_credentials() {
        let (_temp, conn) = open_test_db();
        let now = timestamp_now();
        let google_calendar = models::Calendar {
            id: Uuid::new_v4().to_string(),
            name: "Google Work".to_string(),
            color: "#00aaff".to_string(),
            source: models::CalendarSource::Google,
            google_id: Some("google-work".to_string()),
            visible: true,
            position: 1,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        };
        db::insert_calendar(&conn, &google_calendar).unwrap();

        let cli = Cli {
            command: Command::Google {
                action: GoogleCommand::Sync(GoogleSyncArgs { calendar_id: None }),
            },
        };
        let backend = FakeGoogleSyncBackend::with_default(Err(CliError::external(
            "google credentials are not configured in keyring",
        )));
        let err = execute_with_backend(&conn, cli, &backend).unwrap_err();
        assert_eq!(err.code, "external_error");
    }

    #[test]
    fn google_sync_filters_to_selected_calendar() {
        let (_temp, conn) = open_test_db();
        let now = timestamp_now();
        let google_calendar = models::Calendar {
            id: Uuid::new_v4().to_string(),
            name: "Google Work".to_string(),
            color: "#00aaff".to_string(),
            source: models::CalendarSource::Google,
            google_id: Some("google-work".to_string()),
            visible: true,
            position: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        };
        let other_google_calendar = models::Calendar {
            id: Uuid::new_v4().to_string(),
            name: "Google Personal".to_string(),
            color: "#ffaa00".to_string(),
            source: models::CalendarSource::Google,
            google_id: Some("google-personal".to_string()),
            visible: true,
            position: 2,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        };
        db::insert_calendar(&conn, &google_calendar).unwrap();
        db::insert_calendar(&conn, &other_google_calendar).unwrap();

        let cli = Cli {
            command: Command::Google {
                action: GoogleCommand::Sync(GoogleSyncArgs {
                    calendar_id: Some(google_calendar.id.clone()),
                }),
            },
        };
        let backend = FakeGoogleSyncBackend::with_default(Ok((7, 8)))
            .with_calendar_result(&google_calendar.id, Ok((2, 3)));
        let value = execute_with_backend(&conn, cli, &backend).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["data"]["calendars_synced"], 1);
        assert_eq!(value["data"]["events_added"], 2);
        assert_eq!(value["data"]["events_updated"], 3);
    }
}
