use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use chrono::{Datelike, Duration, Local, NaiveDate};
use tokio::sync::RwLock;

use crate::dag::EventDag;
use crate::google::discovery::DiscoveredGoogleCalendar;
use crate::keys::{Action, View};
use crate::models::{Calendar, Event, EventDependency, Project};
use crate::models::{CognitiveLoad, PlanningTask, TaskPriority};
use crate::sync::state::CalendarSyncState;
use crate::worker::{Worker, WorkerResult};

// ── Form field definitions ────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
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
            planner_settings_availability: String::new(),
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

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        let action = crate::keys::resolve(&self.view, key);
        self.dispatch(action);
    }

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::Help => self.view = View::Help,
            Action::Escape => self.handle_escape(),

            // View switching
            Action::ViewMonth => self.switch_view(View::Month),
            Action::ViewWeek => self.switch_view(View::Week),
            Action::ViewDay => self.switch_view(View::Day),
            Action::ViewAgenda => self.switch_view(View::Agenda),
            Action::PlannerInbox => {
                self.view = View::PlannerInbox;
                self.worker.load_planner_tasks();
                self.worker.load_planner_settings();
            }

            // Focus
            Action::FocusSidebar => {
                self.sidebar_focused = true;
                self.view = View::CalendarList;
            }
            Action::FocusMain => {
                self.sidebar_focused = false;
                self.view = View::Month; // return to last main view (simplified)
            }

            // Time navigation
            Action::PrevPeriod => self.prev_period(),
            Action::NextPeriod => self.next_period(),
            Action::PrevUnit => self.prev_unit(),
            Action::NextUnit => self.next_unit(),
            Action::PrevDay => self.move_day(-1),
            Action::NextDay => self.move_day(1),
            Action::JumpToday => self.jump_today(),

            // Event actions
            Action::CreateEvent => self.open_event_form(None),
            Action::EditEvent => self.open_edit_form(),
            Action::DeleteEvent => self.delete_selected_event(),
            Action::SelectEvent => self.select_event(),

            // Form
            Action::FormNextField => match self.view {
                View::GoogleAuth => {
                    if self.google_auth_field < 1 {
                        self.google_auth_field += 1;
                    }
                }
                View::IcalImport => self.ical_import_next_field(),
                View::PlannerTaskForm => self.planner_task_next_field(),
                View::PlannerSettingsForm => self.planner_settings_next_field(),
                _ => self.form_next_field(),
            },
            Action::FormPrevField => match self.view {
                View::GoogleAuth => {
                    if self.google_auth_field > 0 {
                        self.google_auth_field -= 1;
                    }
                }
                View::IcalImport => self.ical_import_prev_field(),
                View::PlannerTaskForm => self.planner_task_prev_field(),
                View::PlannerSettingsForm => self.planner_settings_prev_field(),
                _ => self.form_prev_field(),
            },
            Action::FormSubmit => match self.view {
                View::GoogleAuth => self.handle_input_submit(),
                View::IcalImport => self.ical_import_submit(),
                View::PlannerTaskForm => self.planner_task_submit(),
                View::PlannerSettingsForm => self.planner_settings_submit(),
                _ => self.form_submit(),
            },
            Action::FormCancel => self.handle_escape(),
            Action::InputChar(c) => {
                if self.view == View::PlannerTaskForm {
                    self.planner_task_input_char(c)
                } else if self.view == View::PlannerSettingsForm {
                    self.planner_settings_input_char(c)
                } else {
                    self.form_input_char(c)
                }
            }
            Action::InputBackspace => {
                if self.view == View::PlannerTaskForm {
                    self.planner_task_input_backspace()
                } else if self.view == View::PlannerSettingsForm {
                    self.planner_settings_input_backspace()
                } else {
                    self.form_input_backspace()
                }
            }
            Action::InputSubmit => self.handle_input_submit(),
            Action::InputCancel => self.handle_escape(),

            // Sidebar
            Action::CalendarUp => {
                if self.view == View::GoogleManage {
                    if self.google_discovery_index > 0 {
                        self.google_discovery_index -= 1;
                    }
                } else if self.calendar_list_index > 0 {
                    self.calendar_list_index -= 1;
                }
            }
            Action::CalendarDown => {
                if self.view == View::GoogleManage {
                    if self.google_discovery_index + 1 < self.google_discovered_calendars.len() {
                        self.google_discovery_index += 1;
                    }
                } else if self.calendar_list_index + 1 < self.calendars.len() {
                    self.calendar_list_index += 1;
                }
            }
            Action::ToggleCalendar => self.toggle_calendar_visibility(),

            // Scroll
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),
            Action::ScrollPageUp => self.scroll_page(-10),
            Action::ScrollPageDown => self.scroll_page(10),

            // Quick add
            Action::QuickAdd => {
                self.quick_add_input.clear();
                self.view = View::QuickAdd;
            }

            // Google
            Action::GoogleManage => self.google_manage(),
            Action::GoogleSync => self.google_sync(),
            Action::GoogleDiscoverCalendars => self.discover_google_calendars(),
            Action::GoogleImportCalendar => self.import_selected_google_calendar(),
            Action::GoogleLogin => self.view = View::GoogleAuth,
            Action::GoogleAuthLogout => self.google_logout(),

            // iCal
            Action::ImportIcal => self.open_ical_import(),
            Action::ExportIcal => self.export_ical(),

            Action::CreateTask => self.open_planner_task_form(),
            Action::PlannerOptimize => {
                if let Some(message) = self.planner_optimization_prerequisite() {
                    self.set_status(message, true);
                    self.open_planner_settings_form();
                    return;
                }
                self.loading = true;
                self.worker.optimize_planner();
            }
            Action::PlannerApply => {
                if let Some(proposal) = &self.planner_proposal {
                    self.loading = true;
                    self.worker
                        .apply_planner_proposal(proposal.proposal.id.clone());
                } else {
                    self.set_status("No proposal is ready to apply.", true);
                }
            }
            Action::PlannerSettings => self.open_planner_settings_form(),

            Action::None | Action::JumpToDate => {}
        }
    }

    // ── Handle tick (animations, worker poll) ─────────────────────

    pub fn handle_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Poll worker for results
        for result in self.worker.drain() {
            self.handle_worker_result(result);
        }
    }

    fn handle_worker_result(&mut self, result: WorkerResult) {
        match result {
            WorkerResult::CalendarsLoaded(cals) => {
                self.calendars = cals;
                // Now load events for the current view window
                self.worker.load_events(self.view_year, self.view_month);
                self.loading = false; // spinner shows until EventsLoaded arrives
            }
            WorkerResult::CalendarSyncStatesLoaded(states) => {
                self.calendar_sync_state = states
                    .into_iter()
                    .map(|state| (state.calendar_id.clone(), state))
                    .collect();
            }
            WorkerResult::ProjectsLoaded(projs) => {
                self.projects = projs;
            }
            WorkerResult::EventsLoaded { events } => {
                self.events = events.clone();
                // Update shared arc for notification task
                let arc = self.events_arc.clone();
                tokio::spawn(async move {
                    *arc.write().await = events;
                });
                self.loading = false;
            }
            WorkerResult::DependenciesLoaded(deps) => {
                self.dag = EventDag::from_dependencies(&deps);
                self.dependencies = deps;
            }
            WorkerResult::PlannerTasksLoaded(tasks) => {
                self.planner_tasks = tasks;
                self.planner_selected_index = self
                    .planner_selected_index
                    .min(self.planner_tasks.len().saturating_sub(1));
            }
            WorkerResult::PlannerSettingsLoaded(settings) => {
                self.planner_settings_timezone = settings
                    .timezone
                    .clone()
                    .unwrap_or_else(crate::time::local_timezone_name);
                self.planner_settings_availability = serde_json::from_str::<
                    crate::planner::Availability,
                >(&settings.availability_json)
                .map(|availability| availability_to_specs(&availability))
                .unwrap_or_default();
                self.planner_settings_horizon_days = settings.horizon_days.to_string();
                self.planner_settings_slot_minutes = settings.slot_minutes.to_string();
                self.planner_settings_solve_seconds = settings.solve_seconds.to_string();
                self.planner_settings = Some(settings);
                self.loading = false;
            }
            WorkerResult::PlannerProposalReady(proposal) => {
                self.planner_proposal = Some(proposal);
                self.loading = false;
                self.set_status(
                    "Planner proposal is ready for review; press a to apply.",
                    false,
                );
            }
            WorkerResult::PlannerProposalApplied(proposal) => {
                self.planner_proposal = Some(proposal);
                self.worker.load_planner_tasks();
                self.worker.load_events(self.view_year, self.view_month);
                self.loading = false;
                self.set_status("Scheduled planner tasks were applied.", false);
            }
            WorkerResult::EventSaved(ev) => {
                // Refresh events for the current window
                self.worker.load_events(self.view_year, self.view_month);
                self.set_status(format!("Saved: {}", ev.title), false);
                self.view = View::Month;
            }
            WorkerResult::EventDeleted(id) => {
                self.events.retain(|e| e.id != id);
                self.set_status("Event deleted.", false);
            }
            WorkerResult::GoogleAuthComplete(client) => {
                self.google_client = Some(client);
                self.set_status("Google authorization complete.", false);
                self.view = View::GoogleManage;
                self.worker.load_calendar_sync_states();
                self.discover_google_calendars();
                self.loading = false;
            }
            WorkerResult::GoogleCalendarsDiscovered(calendars) => {
                self.google_discovered_calendars = calendars;
                if self.google_discovery_index >= self.google_discovered_calendars.len() {
                    self.google_discovery_index =
                        self.google_discovered_calendars.len().saturating_sub(1);
                }
                self.view = View::GoogleManage;
                self.loading = false;
                self.set_status("Google calendar discovery updated.", false);
            }
            WorkerResult::GoogleSyncFinished {
                calendars_succeeded,
                calendars_failed,
                events_added,
                events_updated,
                conflicts_detected,
            } => {
                self.worker.load_events(self.view_year, self.view_month);
                self.worker.load_calendar_sync_states();
                let (status, is_error) = google_sync_finished_status(
                    calendars_succeeded,
                    calendars_failed,
                    events_added,
                    events_updated,
                    conflicts_detected,
                );
                self.set_status(status, is_error);
                self.loading = false;
            }
            WorkerResult::IcalImported(report) => {
                self.worker.load_events(self.view_year, self.view_month);
                self.worker.load_calendar_sync_states();
                self.view = View::Month;
                let warning_suffix = if report.warnings.is_empty() {
                    String::new()
                } else {
                    format!(" {} warnings.", report.warnings.len())
                };
                self.set_status(
                    format!(
                        "Imported {} events from {}.{}",
                        report.imported, report.path, warning_suffix
                    ),
                    false,
                );
                self.loading = false;
            }
            WorkerResult::StatusMessage(msg) => {
                self.set_status(msg, false);
                self.loading = false;
            }
            WorkerResult::Error(e) => {
                self.set_status(e, true);
                self.loading = false;
            }
        }
    }

    // ── Navigation helpers ────────────────────────────────────────

    fn switch_view(&mut self, view: View) {
        self.view = view;
        self.sidebar_focused = false;
    }

    fn prev_period(&mut self) {
        match self.view {
            View::Month => self.shift_month(-1),
            View::Week => self.focused_date -= Duration::weeks(1),
            View::Day => self.focused_date -= Duration::days(1),
            _ => {}
        }
        self.reload_events_if_needed();
    }

    fn next_period(&mut self) {
        match self.view {
            View::Month => self.shift_month(1),
            View::Week => self.focused_date += Duration::weeks(1),
            View::Day => self.focused_date += Duration::days(1),
            _ => {}
        }
        self.reload_events_if_needed();
    }

    fn prev_unit(&mut self) {
        match self.view {
            View::Month => self.focused_date -= Duration::weeks(1),
            View::Week | View::Day => {
                if self.selected_event_index > 0 {
                    self.selected_event_index -= 1;
                } else if self.week_scroll > 0 {
                    self.week_scroll -= 1;
                }
            }
            View::Agenda => {
                self.agenda_scroll = self.agenda_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn next_unit(&mut self) {
        match self.view {
            View::Month => self.focused_date += Duration::weeks(1),
            View::Week | View::Day => {
                let count = self.visible_events().len();
                if self.selected_event_index + 1 < count {
                    self.selected_event_index += 1;
                } else if self.week_scroll < 18 {
                    self.week_scroll += 1;
                }
            }
            View::Agenda => {
                self.agenda_scroll = self.agenda_scroll.saturating_add(1);
            }
            _ => {}
        }
    }

    fn move_day(&mut self, delta: i64) {
        self.focused_date += Duration::days(delta);
        self.reload_events_if_needed();
    }

    fn jump_today(&mut self) {
        let today = Local::now().date_naive();
        self.focused_date = today;
        self.view_month = today.month();
        self.view_year = today.year();
        self.reload_events_if_needed();
    }

    fn shift_month(&mut self, delta: i32) {
        let mut month = self.view_month as i32 + delta;
        let mut year = self.view_year;
        while month < 1 {
            month += 12;
            year -= 1;
        }
        while month > 12 {
            month -= 12;
            year += 1;
        }
        self.view_month = month as u32;
        self.view_year = year;

        // Keep focused_date in sync
        let day = self
            .focused_date
            .day()
            .min(days_in_month(year, month as u32));
        self.focused_date =
            NaiveDate::from_ymd_opt(year, month as u32, day).unwrap_or(self.focused_date);
    }

    fn reload_events_if_needed(&mut self) {
        // Check if focused_date is outside the currently loaded window
        if self.focused_date.month() != self.view_month
            || self.focused_date.year() != self.view_year
        {
            self.view_month = self.focused_date.month();
            self.view_year = self.focused_date.year();
            self.loading = true;
            self.worker.load_events(self.view_year, self.view_month);
        }
    }

    fn handle_escape(&mut self) {
        match self.view {
            View::Help
            | View::EventForm
            | View::IcalImport
            | View::QuickAdd
            | View::GoogleAuth
            | View::GoogleManage => {
                self.view = View::Month;
            }
            View::PlannerTaskForm | View::PlannerSettingsForm => self.view = View::PlannerInbox,
            View::PlannerInbox => self.view = View::Month,
            View::CalendarList => {
                self.sidebar_focused = false;
                self.view = View::Month;
            }
            _ => {}
        }
    }

    // ── Event form ────────────────────────────────────────────────

    pub fn open_event_form(&mut self, event: Option<&Event>) {
        match event {
            None => {
                // New event defaults
                self.form_is_new = true;
                self.form_editing_event = None;
                self.form_title.clear();
                self.form_date = self.focused_date.format("%Y-%m-%d").to_string();
                self.form_start_time = "09:00".to_string();
                self.form_end_time = "10:00".to_string();
                self.form_location.clear();
                self.form_description.clear();
                self.form_rrule.clear();
                self.form_reminder = "15".to_string();
                self.form_all_day = false;
                self.form_calendar_index = 0;
                self.form_timezone = crate::time::local_timezone_name();
                self.form_project_index = 0;
                self.form_recurrence_index = 0;
            }
            Some(ev) => {
                self.form_is_new = false;
                self.form_editing_event = Some(ev.clone());
                self.form_title = ev.title.clone();
                self.form_date = ev.start_at[..10].to_string();
                self.form_start_time = if ev.start_at.len() >= 16 {
                    ev.start_at[11..16].to_string()
                } else {
                    "09:00".to_string()
                };
                self.form_end_time = if ev.end_at.len() >= 16 {
                    ev.end_at[11..16].to_string()
                } else {
                    "10:00".to_string()
                };
                self.form_location = ev.location.clone().unwrap_or_default();
                self.form_description = ev.description.clone().unwrap_or_default();
                self.form_rrule = ev.rrule.clone().unwrap_or_default();
                self.form_reminder = ev
                    .reminder_minutes
                    .map(|m| m.to_string())
                    .unwrap_or_default();
                self.form_all_day = ev.all_day;
                self.form_calendar_index = self
                    .calendars
                    .iter()
                    .position(|c| c.id == ev.calendar_id)
                    .unwrap_or(0);
                self.form_timezone = ev.timezone.clone();
                self.form_project_index = ev
                    .project_id
                    .as_ref()
                    .and_then(|pid| self.projects.iter().position(|p| p.id == *pid))
                    .map(|i| i + 1)
                    .unwrap_or(0);
            }
        }
        self.form_field_index = 0;
        self.view = View::EventForm;
    }

    fn open_edit_form(&mut self) {
        if let Some(event) = self.selected_event().cloned() {
            self.open_event_form(Some(&event));
        } else {
            self.set_status("No event selected.", false);
        }
    }

    fn form_next_field(&mut self) {
        if self.form_field_index + 1 < self.form_fields.len() {
            self.form_field_index += 1;
        }
    }

    fn form_prev_field(&mut self) {
        if self.form_field_index > 0 {
            self.form_field_index -= 1;
        }
    }

    fn form_input_char(&mut self, c: char) {
        match self.view {
            View::QuickAdd => self.quick_add_input.push(c),
            View::GoogleAuth => {
                if self.google_auth_field == 0 {
                    self.google_auth_client_id.push(c);
                } else {
                    self.google_auth_client_secret.push(c);
                }
            }
            View::IcalImport => self.ical_import_input_char(c),
            View::EventForm => self.form_active_field_push(c),
            _ => {}
        }
    }

    fn form_input_backspace(&mut self) {
        match self.view {
            View::QuickAdd => {
                self.quick_add_input.pop();
            }
            View::GoogleAuth => {
                if self.google_auth_field == 0 {
                    self.google_auth_client_id.pop();
                } else {
                    self.google_auth_client_secret.pop();
                }
            }
            View::IcalImport => self.ical_import_input_backspace(),
            View::EventForm => self.form_active_field_pop(),
            _ => {}
        }
    }

    fn form_active_field_push(&mut self, c: char) {
        match self.form_fields.get(self.form_field_index) {
            Some(FormField::Title) => self.form_title.push(c),
            Some(FormField::Date) => self.form_date.push(c),
            Some(FormField::StartTime) => self.form_start_time.push(c),
            Some(FormField::EndTime) => self.form_end_time.push(c),
            Some(FormField::Location) => self.form_location.push(c),
            Some(FormField::Description) => self.form_description.push(c),
            Some(FormField::Timezone) => self.form_timezone.push(c),
            Some(FormField::Recurrence) => self.form_rrule.push(c),
            Some(FormField::Reminder) if c.is_ascii_digit() => self.form_reminder.push(c),
            Some(FormField::Calendar) => {
                // Cycle through calendars with +/-
                if c == '+' || c == 'l' {
                    if self.form_calendar_index + 1 < self.calendars.len() {
                        self.form_calendar_index += 1;
                    }
                } else if (c == '-' || c == 'h') && self.form_calendar_index > 0 {
                    self.form_calendar_index -= 1;
                }
            }
            Some(FormField::Project) => {
                if c == '+' || c == 'l' {
                    if self.form_project_index < self.projects.len() {
                        self.form_project_index += 1;
                    }
                } else if (c == '-' || c == 'h') && self.form_project_index > 0 {
                    self.form_project_index -= 1;
                }
            }
            Some(FormField::AllDay) if c == ' ' => self.form_all_day = !self.form_all_day,
            _ => {}
        }
    }

    fn form_active_field_pop(&mut self) {
        match self.form_fields.get(self.form_field_index) {
            Some(FormField::Title) => {
                self.form_title.pop();
            }
            Some(FormField::Date) => {
                self.form_date.pop();
            }
            Some(FormField::StartTime) => {
                self.form_start_time.pop();
            }
            Some(FormField::EndTime) => {
                self.form_end_time.pop();
            }
            Some(FormField::Location) => {
                self.form_location.pop();
            }
            Some(FormField::Description) => {
                self.form_description.pop();
            }
            Some(FormField::Timezone) => {
                self.form_timezone.pop();
            }
            Some(FormField::Recurrence) => {
                self.form_rrule.pop();
            }
            Some(FormField::Reminder) => {
                self.form_reminder.pop();
            }
            _ => {}
        }
    }

    fn form_submit(&mut self) {
        if self.form_title.trim().is_empty() {
            self.set_status("Title cannot be empty.", true);
            return;
        }

        let cal_id = self
            .calendars
            .get(self.form_calendar_index)
            .map(|c| c.id.clone())
            .unwrap_or_default();
        if cal_id.is_empty() {
            self.set_status("No calendar selected.", true);
            return;
        }

        let start_at = format!("{} {}:00", self.form_date, self.form_start_time);
        let end_at = format!("{} {}:00", self.form_date, self.form_end_time);
        let timezone = self.form_timezone.trim().to_string();

        let mut event = if let Some(existing) = &self.form_editing_event {
            existing.clone()
        } else {
            Event::new(&cal_id, &self.form_title, &start_at, &end_at, &timezone)
        };

        event.calendar_id = cal_id;
        event.title = self.form_title.clone();
        event.start_at = start_at;
        event.end_at = end_at;
        event.all_day = self.form_all_day;
        event.timezone = timezone;
        event.location = if self.form_location.is_empty() {
            None
        } else {
            Some(self.form_location.clone())
        };
        event.description = if self.form_description.is_empty() {
            None
        } else {
            Some(self.form_description.clone())
        };
        event.rrule = if self.form_rrule.is_empty() {
            None
        } else {
            Some(self.form_rrule.clone())
        };
        event.reminder_minutes = self.form_reminder.parse().ok();
        event.project_id = if self.form_project_index == 0 {
            None
        } else {
            self.projects
                .get(self.form_project_index - 1)
                .map(|p| p.id.clone())
        };

        self.loading = true;
        self.worker.save_event(event, self.form_is_new);
    }

    fn handle_input_submit(&mut self) {
        match self.view {
            View::QuickAdd => {
                let input = self.quick_add_input.trim().to_string();
                if !input.is_empty() {
                    self.parse_and_create_event(&input);
                }
                self.view = View::Month;
            }
            View::GoogleAuth => {
                if self.google_auth_field == 0 {
                    self.google_auth_field = 1;
                } else {
                    self.complete_google_auth();
                }
            }
            View::IcalImport => self.ical_import_submit(),
            _ => {}
        }
    }

    fn parse_and_create_event(&mut self, input: &str) {
        // Simple natural language parser: "title at HH:MM" or "title on YYYY-MM-DD at HH:MM"
        let title = input.to_string();
        let date = self.focused_date.format("%Y-%m-%d").to_string();
        let start = format!("{} 09:00:00", date);
        let end = format!("{} 10:00:00", date);

        let cal_id = self
            .calendars
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_default();
        if cal_id.is_empty() {
            self.set_status("No calendar available.", true);
            return;
        }

        let event = Event::new(cal_id, title, start, end, "UTC");
        let mut event = event;
        event.timezone = crate::time::local_timezone_name();
        self.loading = true;
        self.worker.save_event(event, true);
    }

    // ── Calendar list actions ─────────────────────────────────────

    fn toggle_calendar_visibility(&mut self) {
        if let Some(cal) = self.calendars.get_mut(self.calendar_list_index) {
            cal.visible = !cal.visible;
        }
    }

    // ── iCal import ───────────────────────────────────────────────

    fn open_ical_import(&mut self) {
        if self.calendars.is_empty() {
            self.set_status("No calendar available for import.", true);
            return;
        }
        self.ical_import_path.clear();
        self.ical_import_calendar_index = self.calendar_list_index.min(self.calendars.len() - 1);
        self.ical_import_field_index = 0;
        self.view = View::IcalImport;
    }

    fn ical_import_next_field(&mut self) {
        if self.ical_import_field_index < 1 {
            self.ical_import_field_index += 1;
        }
    }

    fn ical_import_prev_field(&mut self) {
        if self.ical_import_field_index > 0 {
            self.ical_import_field_index -= 1;
        }
    }

    fn ical_import_input_char(&mut self, c: char) {
        match self.ical_import_field_index {
            0 => self.ical_import_path.push(c),
            1 => {
                if (c == 'h' || c == '-') && self.ical_import_calendar_index > 0 {
                    self.ical_import_calendar_index -= 1;
                } else if (c == 'l' || c == '+')
                    && self.ical_import_calendar_index + 1 < self.calendars.len()
                {
                    self.ical_import_calendar_index += 1;
                }
            }
            _ => {}
        }
    }

    fn ical_import_input_backspace(&mut self) {
        if self.ical_import_field_index == 0 {
            self.ical_import_path.pop();
        }
    }

    fn ical_import_submit(&mut self) {
        let path = self.ical_import_path.trim().to_string();
        if path.is_empty() {
            self.set_status("Import path cannot be empty.", true);
            return;
        }
        let Some(calendar_id) = self
            .calendars
            .get(self.ical_import_calendar_index)
            .map(|calendar| calendar.id.clone())
        else {
            self.set_status("No calendar selected for import.", true);
            return;
        };

        self.loading = true;
        self.set_status(format!("Importing {}…", path), false);
        self.worker
            .import_ical(calendar_id, path, crate::time::local_timezone_name());
    }

    // ── Google ────────────────────────────────────────────────────

    fn google_manage(&mut self) {
        if self.google_client.is_some() {
            self.view = View::GoogleManage;
            if self.google_discovered_calendars.is_empty() {
                self.discover_google_calendars();
            }
        } else {
            self.view = View::GoogleAuth;
        }
    }

    fn discover_google_calendars(&mut self) {
        let client_opt = self.google_client.clone();
        if let Some(client) = client_opt {
            self.loading = true;
            self.set_status("Loading Google calendars…", false);
            self.worker.discover_google_calendars(client);
        } else {
            self.view = View::GoogleAuth;
        }
    }

    fn import_selected_google_calendar(&mut self) {
        let Some(selected) = self
            .google_discovered_calendars
            .get(self.google_discovery_index)
            .cloned()
        else {
            self.set_status("No Google calendar selected.", true);
            return;
        };

        if self
            .calendars
            .iter()
            .any(|calendar| calendar.google_id.as_deref() == Some(selected.google_id.as_str()))
        {
            self.set_status("Google calendar already imported.", true);
            return;
        }

        match crate::db::open().and_then(|conn| {
            crate::calendar_service::import_google_calendar(&conn, &selected, None)
                .map_err(|e| anyhow::anyhow!(e.to_string()))
        }) {
            Ok(_) => {
                self.worker.load_calendars();
                self.worker.load_calendar_sync_states();
                self.set_status(
                    format!("Imported Google calendar '{}'.", selected.name),
                    false,
                );
            }
            Err(err) => self.set_status(format!("Google import failed: {}", err), true),
        }
    }

    fn google_logout(&mut self) {
        match crate::google::auth::GoogleClient::logout() {
            Ok(_) => {
                self.google_client = None;
                self.google_discovered_calendars.clear();
                self.view = View::Month;
                self.set_status("Google disconnected.", false);
            }
            Err(err) => self.set_status(format!("Google logout failed: {}", err), true),
        }
    }

    fn google_sync(&mut self) {
        // Clone what we need before taking any mutable borrows
        let client_opt = self.google_client.clone();
        if let Some(client) = client_opt {
            self.loading = true;
            self.set_status("Syncing with Google Calendar…", false);
            let google_cals = self
                .calendars
                .iter()
                .filter(|c| c.source == crate::models::CalendarSource::Google)
                .cloned()
                .collect::<Vec<_>>();
            if google_cals.is_empty() {
                self.set_status("No Google calendars configured. Press G to set up.", false);
                self.loading = false;
            } else {
                self.worker.google_sync(google_cals, client);
            }
        } else {
            self.view = View::GoogleAuth;
        }
    }

    fn complete_google_auth(&mut self) {
        let client_id = self.google_auth_client_id.clone();
        let client_secret = self.google_auth_client_secret.clone();

        if client_id.is_empty() {
            self.set_status("Client ID is required.", true);
            return;
        }

        self.set_status("Opening browser for Google authorization…", false);
        self.view = View::GoogleManage;
        self.loading = true;
        self.worker.complete_google_auth(client_id, client_secret);
        self.set_status("Waiting for browser authorization…", false);
    }

    // ── iCal export ───────────────────────────────────────────────

    fn export_ical(&mut self) {
        let path = dirs::home_dir()
            .unwrap_or_default()
            .join("solverforge-calendar.ics");

        match crate::ical::export_to_file(&self.events, "SolverForge Calendar", &path) {
            Ok(()) => self.set_status(format!("Exported to {}", path.display()), false),
            Err(e) => self.set_status(format!("Export failed: {}", e), true),
        }
    }

    // ── Selected event helpers ─────────────────────────────────────

    fn delete_selected_event(&mut self) {
        if let Some(event) = self.selected_event().cloned() {
            self.worker.delete_event(event.id);
        }
    }

    fn select_event(&mut self) {
        // In month view, Select means switch to day view for the focused date
        if self.view == View::Month {
            self.view = View::Day;
        }
    }

    fn open_planner_task_form(&mut self) {
        self.planner_task_field = 0;
        self.planner_task_title.clear();
        self.planner_task_duration = "60".to_string();
        self.planner_task_calendar_index = 0;
        self.planner_task_priority_index = 1;
        self.planner_task_cognitive_index = 1;
        self.view = View::PlannerTaskForm;
    }

    fn planner_task_next_field(&mut self) {
        self.planner_task_field = (self.planner_task_field + 1) % 5;
    }

    fn planner_task_prev_field(&mut self) {
        self.planner_task_field = self.planner_task_field.checked_sub(1).unwrap_or(4);
    }

    fn planner_task_input_char(&mut self, c: char) {
        match self.planner_task_field {
            0 => self.planner_task_title.push(c),
            1 if c.is_ascii_digit() => self.planner_task_duration.push(c),
            2 if !self.calendars.is_empty() => {
                self.planner_task_calendar_index =
                    (self.planner_task_calendar_index + 1) % self.calendars.len();
            }
            3 => self.planner_task_priority_index = (self.planner_task_priority_index + 1) % 3,
            4 => self.planner_task_cognitive_index = (self.planner_task_cognitive_index + 1) % 3,
            _ => {}
        }
    }

    fn planner_task_input_backspace(&mut self) {
        match self.planner_task_field {
            0 => {
                self.planner_task_title.pop();
            }
            1 => {
                self.planner_task_duration.pop();
            }
            _ => {}
        }
    }

    fn planner_task_submit(&mut self) {
        let Some(calendar) = self.calendars.get(self.planner_task_calendar_index) else {
            self.set_status("Create a calendar before adding a planner task.", true);
            return;
        };
        let duration_minutes = match self.planner_task_duration.parse() {
            Ok(value) if value > 0 => value,
            _ => {
                self.set_status("Task duration must be a positive number of minutes.", true);
                return;
            }
        };
        let priority = [TaskPriority::Low, TaskPriority::Normal, TaskPriority::High]
            [self.planner_task_priority_index]
            .clone();
        let cognitive_load = [
            CognitiveLoad::Low,
            CognitiveLoad::Medium,
            CognitiveLoad::High,
        ][self.planner_task_cognitive_index]
            .clone();
        self.loading = true;
        self.worker
            .create_planner_task(crate::planner::CreateTaskInput {
                title: self.planner_task_title.clone(),
                duration_minutes,
                target_calendar_id: calendar.id.clone(),
                project_id: None,
                priority,
                cognitive_load,
                earliest_at: None,
                deadline_kind: crate::models::DeadlineKind::None,
                deadline_at: None,
            });
        self.view = View::PlannerInbox;
    }

    fn open_planner_settings_form(&mut self) {
        self.planner_settings_field = 0;
        self.worker.load_planner_settings();
        self.view = View::PlannerSettingsForm;
    }

    fn planner_settings_next_field(&mut self) {
        self.planner_settings_field = (self.planner_settings_field + 1) % 5;
    }

    fn planner_settings_prev_field(&mut self) {
        self.planner_settings_field = self.planner_settings_field.checked_sub(1).unwrap_or(4);
    }

    fn planner_settings_input_char(&mut self, character: char) {
        match self.planner_settings_field {
            0 => self.planner_settings_timezone.push(character),
            1 => self.planner_settings_availability.push(character),
            2 => self.planner_settings_horizon_days.push(character),
            3 => self.planner_settings_slot_minutes.push(character),
            4 => self.planner_settings_solve_seconds.push(character),
            _ => {}
        }
    }

    fn planner_settings_input_backspace(&mut self) {
        match self.planner_settings_field {
            0 => self.planner_settings_timezone.pop(),
            1 => self.planner_settings_availability.pop(),
            2 => self.planner_settings_horizon_days.pop(),
            3 => self.planner_settings_slot_minutes.pop(),
            4 => self.planner_settings_solve_seconds.pop(),
            _ => None,
        };
    }

    fn planner_settings_submit(&mut self) {
        let timezone = match crate::time::normalize_timezone(self.planner_settings_timezone.trim())
        {
            Ok(timezone) => timezone,
            Err(_) => {
                self.set_status(
                    "Timezone must be an IANA name such as Europe/Rome or UTC.",
                    true,
                );
                return;
            }
        };
        let availability = match availability_from_specs(&self.planner_settings_availability) {
            Ok(value) => value,
            Err(message) => {
                self.set_status(message, true);
                return;
            }
        };
        let parse_number = |value: &str, label: &str| {
            value
                .parse::<i64>()
                .map_err(|_| format!("{label} must be a whole number."))
        };
        let horizon_days = match parse_number(&self.planner_settings_horizon_days, "Horizon days") {
            Ok(value) => value,
            Err(message) => {
                self.set_status(message, true);
                return;
            }
        };
        let slot_minutes = match parse_number(&self.planner_settings_slot_minutes, "Slot minutes") {
            Ok(value) => value,
            Err(message) => {
                self.set_status(message, true);
                return;
            }
        };
        let solve_seconds =
            match parse_number(&self.planner_settings_solve_seconds, "Solve seconds") {
                Ok(value) => value,
                Err(message) => {
                    self.set_status(message, true);
                    return;
                }
            };
        self.loading = true;
        self.worker
            .update_planner_settings(crate::planner::SettingsUpdate {
                timezone: Some(timezone),
                availability: Some(availability),
                horizon_days: Some(horizon_days),
                slot_minutes: Some(slot_minutes),
                solve_seconds: Some(solve_seconds),
                ..Default::default()
            });
        self.view = View::PlannerInbox;
    }

    fn planner_optimization_prerequisite(&self) -> Option<String> {
        let Some(settings) = self.planner_settings.as_ref() else {
            return Some("Planner settings are loading; wait a moment, then optimize.".into());
        };
        let Some(timezone) = settings.timezone.as_deref() else {
            return Some(
                "Set a planner timezone first: use an IANA name such as Europe/Rome or UTC.".into(),
            );
        };
        if crate::time::normalize_timezone(timezone).is_err() {
            return Some(
                "Correct the planner timezone: use an IANA name such as Europe/Rome or UTC.".into(),
            );
        }
        let availability =
            serde_json::from_str::<crate::planner::Availability>(&settings.availability_json);
        match availability {
            Ok(value) if !value.0.is_empty() => None,
            _ => Some("Set at least one weekly availability window before optimizing.".into()),
        }
    }

    pub fn selected_event(&self) -> Option<&Event> {
        self.visible_events()
            .get(self.selected_event_index)
            .copied()
    }

    /// Events visible in the current view window, filtered by calendar visibility.
    pub fn visible_events(&self) -> Vec<&Event> {
        let visible_cal_ids: std::collections::HashSet<&str> = self
            .calendars
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id.as_str())
            .collect();

        self.events
            .iter()
            .filter(|e| visible_cal_ids.contains(e.calendar_id.as_str()) && e.deleted_at.is_none())
            .collect()
    }

    /// Events for a specific date.
    pub fn events_on_date(&self, date: NaiveDate) -> Vec<&Event> {
        self.visible_events()
            .into_iter()
            .filter(|e| e.occurs_on(date))
            .collect()
    }

    // ── Calendar color lookup ──────────────────────────────────────

    /// Find the 0-based index of a calendar in the calendars list (for color lookup).
    pub fn calendar_index_for(&self, calendar_id: &str) -> usize {
        self.calendars
            .iter()
            .position(|c| c.id == calendar_id)
            .unwrap_or(0)
    }

    // ── Status ────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_message = msg.into();
        self.status_is_error = is_error;
    }

    /// True if the cursor should be visible (500ms on, 500ms off at 250ms tick rate).
    pub fn cursor_visible(&self) -> bool {
        self.tick_count % 4 < 2
    }

    // ── Scroll helpers ────────────────────────────────────────────

    fn scroll_up(&mut self) {
        match self.view {
            View::Help => self.help_scroll = self.help_scroll.saturating_sub(1),
            View::Agenda => self.agenda_scroll = self.agenda_scroll.saturating_sub(1),
            View::Week | View::Day if self.week_scroll > 0 => self.week_scroll -= 1,
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        match self.view {
            View::Help => self.help_scroll = self.help_scroll.saturating_add(1),
            View::Agenda => self.agenda_scroll = self.agenda_scroll.saturating_add(1),
            View::Week | View::Day if self.week_scroll < 20 => self.week_scroll += 1,
            _ => {}
        }
    }

    fn scroll_page(&mut self, delta: i16) {
        match self.view {
            View::Help => {
                if delta > 0 {
                    self.help_scroll = self.help_scroll.saturating_add(delta as u16);
                } else {
                    self.help_scroll = self.help_scroll.saturating_sub((-delta) as u16);
                }
            }
            View::Agenda => {
                if delta > 0 {
                    self.agenda_scroll = self.agenda_scroll.saturating_add(delta as u16);
                } else {
                    self.agenda_scroll = self.agenda_scroll.saturating_sub((-delta) as u16);
                }
            }
            _ => {}
        }
    }
}

fn availability_from_specs(value: &str) -> Result<crate::planner::Availability, String> {
    let mut days = BTreeMap::new();
    for spec in value
        .split(',')
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
    {
        let (day, window) = spec
            .split_once('=')
            .ok_or_else(|| "Availability uses day=HH:MM-HH:MM, separated by commas.".to_string())?;
        let (start, end) = window
            .split_once('-')
            .ok_or_else(|| "Availability uses day=HH:MM-HH:MM, separated by commas.".to_string())?;
        days.entry(day.to_ascii_lowercase())
            .or_insert_with(Vec::new)
            .push(crate::planner::TimeWindow {
                start: start.into(),
                end: end.into(),
            });
    }
    Ok(crate::planner::Availability(days))
}

fn availability_to_specs(availability: &crate::planner::Availability) -> String {
    availability
        .0
        .iter()
        .flat_map(|(day, windows)| {
            windows
                .iter()
                .map(move |window| format!("{day}={}-{}", window.start, window.end))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month_year = if month == 12 { year + 1 } else { year };
    let next_month = if month == 12 { 1 } else { month + 1 };
    NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}

fn google_sync_finished_status(
    calendars_succeeded: usize,
    calendars_failed: usize,
    events_added: usize,
    events_updated: usize,
    conflicts_detected: usize,
) -> (String, bool) {
    if calendars_failed > 0 {
        let mut parts = Vec::new();
        if calendars_succeeded > 0 {
            parts.push(format!("{} succeeded", calendars_succeeded));
        }
        parts.push(format!("{} failed", calendars_failed));
        if conflicts_detected > 0 {
            parts.push(format!("{} conflicts", conflicts_detected));
        }
        return (
            format!(
                "Google sync finished with failures: {}. Open Google management for details.",
                parts.join(", ")
            ),
            true,
        );
    }

    let status = if conflicts_detected > 0 {
        format!(
            "Google sync: +{} events, {} updated, {} conflicts.",
            events_added, events_updated, conflicts_detected
        )
    } else {
        format!(
            "Google sync: +{} events, {} updated.",
            events_added, events_updated
        )
    };
    (status, false)
}

#[cfg(test)]
mod tests {
    use super::google_sync_finished_status;

    #[test]
    fn google_sync_finished_status_prefers_failure_over_success_banner() {
        let (message, is_error) = google_sync_finished_status(1, 1, 2, 3, 0);
        assert!(is_error);
        assert!(message.contains("1 succeeded"));
        assert!(message.contains("1 failed"));
    }

    #[test]
    fn google_sync_finished_status_reports_success_totals() {
        let (message, is_error) = google_sync_finished_status(2, 0, 4, 5, 1);
        assert!(!is_error);
        assert_eq!(message, "Google sync: +4 events, 5 updated, 1 conflicts.");
    }
}
