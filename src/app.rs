use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{Datelike, Duration, Local, NaiveDate};
use tokio::sync::RwLock;

use crate::dag::EventDag;
use crate::google::discovery::DiscoveredGoogleCalendar;
use crate::keys::{Action, View};
use crate::models::{Calendar, Event, EventDependency, Project};
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
            Action::FormNextField => self.form_next_field(),
            Action::FormPrevField => self.form_prev_field(),
            Action::FormSubmit => self.form_submit(),
            Action::FormCancel => self.handle_escape(),
            Action::InputChar(c) => self.form_input_char(c),
            Action::InputBackspace => self.form_input_backspace(),
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
            Action::ImportIcal => {
                self.set_status("iCal import: use `import <path>` in quick-add.", false)
            }
            Action::ExportIcal => self.export_ical(),

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
            WorkerResult::GoogleSyncComplete {
                events_added,
                events_updated,
            } => {
                self.set_status(
                    format!(
                        "Google sync: +{} events, {} updated.",
                        events_added, events_updated
                    ),
                    false,
                );
                self.worker.load_events(self.view_year, self.view_month);
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
            | View::QuickAdd
            | View::GoogleAuth
            | View::GoogleManage => {
                self.view = View::Month;
            }
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
            Some(FormField::Reminder) => {
                if c.is_ascii_digit() {
                    self.form_reminder.push(c);
                }
            }
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
            Some(FormField::AllDay) => {
                if c == ' ' {
                    self.form_all_day = !self.form_all_day;
                }
            }
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
            View::Week | View::Day => {
                if self.week_scroll > 0 {
                    self.week_scroll -= 1;
                }
            }
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        match self.view {
            View::Help => self.help_scroll = self.help_scroll.saturating_add(1),
            View::Agenda => self.agenda_scroll = self.agenda_scroll.saturating_add(1),
            View::Week | View::Day => {
                if self.week_scroll < 20 {
                    self.week_scroll += 1;
                }
            }
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

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month_year = if month == 12 { year + 1 } else { year };
    let next_month = if month == 12 { 1 } else { month + 1 };
    NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}
