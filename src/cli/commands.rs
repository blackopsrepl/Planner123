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
    Tasks {
        #[command(subcommand)]
        action: TaskCommand,
    },
    Planner {
        #[command(subcommand)]
        action: PlannerCommand,
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
pub enum TaskCommand {
    List,
    Get {
        id: String,
    },
    Create(TaskCreateArgs),
    Update(TaskUpdateArgs),
    Delete {
        id: String,
    },
    ReturnToInbox {
        id: String,
    },
    Dependencies {
        #[command(subcommand)]
        action: TaskDependencyCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum TaskDependencyCommand {
    List,
    Add(TaskDependencyArgs),
    Remove(TaskDependencyArgs),
}

#[derive(Debug, Subcommand)]
pub enum PlannerCommand {
    Settings {
        #[command(subcommand)]
        action: PlannerSettingsCommand,
    },
    Optimize(PlannerOptimizeArgs),
    Proposals {
        #[command(subcommand)]
        action: PlannerProposalCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum PlannerSettingsCommand {
    Show,
    Update(Box<PlannerSettingsArgs>),
}

#[derive(Debug, Subcommand)]
pub enum PlannerProposalCommand {
    List,
    Get { id: String },
    Apply { id: String },
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TaskPriorityArg {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CognitiveLoadArg {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DeadlineKindArg {
    None,
    Hard,
    Soft,
}
