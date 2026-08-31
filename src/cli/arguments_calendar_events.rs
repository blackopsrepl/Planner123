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
