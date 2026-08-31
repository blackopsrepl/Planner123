use super::*;

// ── Form field definitions ────────────────────────────────────────────

pub enum FormField {
    Title,
    Date,
    StartTime,
    EndTime,
    Calendar,
    Timezone,
    Location,
    Description,
    Recurrence,
    Reminder,
    Project,
    AllDay,
}

impl FormField {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Title,
            Self::Date,
            Self::StartTime,
            Self::EndTime,
            Self::AllDay,
            Self::Calendar,
            Self::Timezone,
            Self::Location,
            Self::Description,
            Self::Recurrence,
            Self::Reminder,
            Self::Project,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Title => "Title",
            Self::Date => "Date",
            Self::StartTime => "Start",
            Self::EndTime => "End",
            Self::Calendar => "Calendar",
            Self::Timezone => "Timezone",
            Self::Location => "Location",
            Self::Description => "Description",
            Self::Recurrence => "Repeats",
            Self::Reminder => "Reminder",
            Self::Project => "Project",
            Self::AllDay => "All day",
        }
    }
}

// ── Main App struct ───────────────────────────────────────────────────

pub struct App {
    pub running: bool,
    pub view: View,

    // ── Calendar navigation ─────────────────────────────────────
    pub focused_date: NaiveDate, // currently selected date
    pub view_month: u32,         // month currently shown in month view
    pub view_year: i32,
    pub week_scroll: i16,            // hour offset in week/day view (0 = 00:00)
    pub selected_event_index: usize, // index into visible_events

    // ── Data ─────────────────────────────────────────────────────
    pub calendars: Vec<Calendar>,
    pub calendar_sync_state: HashMap<String, CalendarSyncState>,
    pub projects: Vec<Project>,
    pub events: Vec<Event>, // events in current view window
    pub dependencies: Vec<EventDependency>,
    pub planner_tasks: Vec<PlanningTask>,
    pub planner_proposal: Option<crate::planner::ProposalDetail>,
    pub planner_settings: Option<crate::models::PlannerSettings>,
    pub planner_selected_index: usize,

    // ── Planner task form state ──────────────────────────────────
    pub planner_task_field: usize,
    pub planner_task_title: String,
    pub planner_task_duration: String,
    pub planner_task_calendar_index: usize,
    pub planner_task_priority_index: usize,
    pub planner_task_cognitive_index: usize,
    pub planner_settings_field: usize,
    pub planner_settings_timezone: String,
    pub planner_settings_availability: String,
    pub planner_settings_horizon_days: String,
    pub planner_settings_slot_minutes: String,
    pub planner_settings_solve_seconds: String,
    pub dag: EventDag,
    pub completed_event_ids: HashSet<String>,

    // ── Sidebar state ────────────────────────────────────────────
    pub sidebar_focused: bool,
    pub calendar_list_index: usize,

    // ── Event form state ─────────────────────────────────────────
    pub form_editing_event: Option<Event>, // None = creating new
    pub form_is_new: bool,
    pub form_field_index: usize,
    pub form_fields: Vec<FormField>,
    // Per-field input buffers
    pub form_title: String,
    pub form_date: String,
    pub form_start_time: String,
    pub form_end_time: String,
    pub form_location: String,
    pub form_description: String,
    pub form_rrule: String,
    pub form_reminder: String,
    pub form_calendar_index: usize,
    pub form_timezone: String,
    pub form_project_index: usize, // 0 = none
    pub form_all_day: bool,
    pub form_recurrence_index: usize,

    // ── iCal import state ─────────────────────────────────────────
    pub ical_import_path: String,
    pub ical_import_calendar_index: usize,
    pub ical_import_field_index: usize,

    // ── Quick-add bar ─────────────────────────────────────────────
    pub quick_add_input: String,

    // ── Agenda scroll ─────────────────────────────────────────────
    pub agenda_scroll: u16,

    // ── Help popup ────────────────────────────────────────────────
    pub help_scroll: u16,

    // ── Google Auth wizard ────────────────────────────────────────
    pub google_auth_client_id: String,
    pub google_auth_client_secret: String,
    pub google_auth_field: usize, // 0 = client_id, 1 = client_secret
    pub google_client: Option<Arc<crate::google::auth::GoogleClient>>,
    pub google_discovered_calendars: Vec<DiscoveredGoogleCalendar>,
    pub google_discovery_index: usize,

    // ── Status / loading ──────────────────────────────────────────
    pub status_message: String,
    pub status_is_error: bool,
    pub loading: bool,
    pub tick_count: u64,

    // ── Shared events list for notification task ───────────────────
    pub events_arc: Arc<RwLock<Vec<Event>>>,

    // ── Worker ────────────────────────────────────────────────────
    pub worker: Worker,
}

impl App {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        let today = Local::now().date_naive();
        let events_arc = Arc::new(RwLock::new(Vec::new()));

        let mut app = Self {
            running: true,
            view: View::Month,
            focused_date: today,
            view_month: today.month(),
            view_year: today.year(),
            week_scroll: 8, // default to showing 08:00
            selected_event_index: 0,
            calendars: Vec::new(),
            calendar_sync_state: HashMap::new(),
            projects: Vec::new(),
            events: Vec::new(),
            dependencies: Vec::new(),
            planner_tasks: Vec::new(),
            planner_proposal: None,
            planner_settings: None,
            planner_selected_index: 0,
            planner_task_field: 0,
            planner_task_title: String::new(),
            planner_task_duration: "60".to_string(),
            planner_task_calendar_index: 0,
            planner_task_priority_index: 1,
            planner_task_cognitive_index: 1,
            planner_settings_field: 0,
            planner_settings_timezone: String::new(),
            planner_settings_availability: default_weekly_availability(),
            planner_settings_horizon_days: "14".into(),
            planner_settings_slot_minutes: "15".into(),
            planner_settings_solve_seconds: "5".into(),
            dag: EventDag::new(),
            completed_event_ids: HashSet::new(),
            sidebar_focused: false,
            calendar_list_index: 0,
            form_editing_event: None,
            form_is_new: true,
            form_field_index: 0,
            form_fields: FormField::all(),
            form_title: String::new(),
            form_date: today.format("%Y-%m-%d").to_string(),
            form_start_time: "09:00".to_string(),
            form_end_time: "10:00".to_string(),
            form_location: String::new(),
            form_description: String::new(),
            form_rrule: String::new(),
            form_reminder: String::new(),
            form_calendar_index: 0,
            form_timezone: crate::time::local_timezone_name(),
            form_project_index: 0,
            form_all_day: false,
            form_recurrence_index: 0,
            ical_import_path: String::new(),
            ical_import_calendar_index: 0,
            ical_import_field_index: 0,
            quick_add_input: String::new(),
            agenda_scroll: 0,
            help_scroll: 0,
            google_auth_client_id: String::new(),
            google_auth_client_secret: String::new(),
            google_auth_field: 0,
            google_client: None,
            google_discovered_calendars: Vec::new(),
            google_discovery_index: 0,
            status_message: String::new(),
            status_is_error: false,
            loading: true,
            tick_count: 0,
            events_arc,
            worker: Worker::new(rt),
        };

        // Load saved Google credentials if available
        if let Some(saved) = crate::google::auth::GoogleClient::saved_credentials() {
            app.google_auth_client_id = saved.client_id;
            app.google_auth_client_secret = saved.client_secret.unwrap_or_default();
        }

        if let Some(client) = crate::google::auth::GoogleClient::from_keyring() {
            app.google_client = Some(Arc::new(client));
        }

        // Kick off initial data load
        app.worker.load_calendars();
        app.worker.load_calendar_sync_states();
        app.worker.load_projects();
        app.worker.load_dependencies();
        app.worker.load_planner_tasks();

        app
    }

    // ── Main event handler ────────────────────────────────────────
}
