#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

// ── Event ────────────────────────────────────────────────────────────

/* A calendar event. Maps to the `events` table. */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub calendar_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: String, // ISO 8601 datetime
    pub end_at: String,
    pub all_day: bool,
    pub rrule: Option<String>, // RFC 5545 RRULE property value, beginning with FREQ=
    pub google_id: Option<String>,
    pub google_etag: Option<String>,
    pub reminder_minutes: Option<i64>,
    pub timezone: String, // IANA timezone name e.g. "Europe/Lisbon"
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Event {
    pub fn new(
        calendar_id: impl Into<String>,
        title: impl Into<String>,
        start_at: impl Into<String>,
        end_at: impl Into<String>,
        timezone: impl Into<String>,
    ) -> Self {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            calendar_id: calendar_id.into(),
            project_id: None,
            title: title.into(),
            description: None,
            location: None,
            start_at: start_at.into(),
            end_at: end_at.into(),
            all_day: false,
            rrule: None,
            google_id: None,
            google_etag: None,
            reminder_minutes: None,
            timezone: timezone.into(),
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Parse `start_at` into a `DateTime<Utc>` for calendar math.
    pub fn start_dt(&self) -> Option<DateTime<Utc>> {
        self.start_local_dt().map(|dt| dt.with_timezone(&Utc))
    }

    /// Parse `end_at` into a `DateTime<Utc>`.
    pub fn end_dt(&self) -> Option<DateTime<Utc>> {
        self.end_local_dt().map(|dt| dt.with_timezone(&Utc))
    }

    pub fn timezone_tz(&self) -> Option<Tz> {
        crate::time::parse_timezone(&self.timezone).ok()
    }

    pub fn start_local_dt(&self) -> Option<DateTime<Tz>> {
        crate::time::resolve_local_datetime(&self.start_at, &self.timezone).ok()
    }

    pub fn end_local_dt(&self) -> Option<DateTime<Tz>> {
        crate::time::resolve_local_datetime(&self.end_at, &self.timezone).ok()
    }

    /// Duration in minutes.
    pub fn duration_minutes(&self) -> Option<i64> {
        let start = self.start_dt()?;
        let end = self.end_dt()?;
        Some((end - start).num_minutes())
    }

    /// True if this event occurs on the given wall-clock date.
    pub fn occurs_on(&self, date: NaiveDate) -> bool {
        if let (Some(start), Some(end)) = (self.start_local_dt(), self.end_local_dt()) {
            let s = start.date_naive();
            let e = end.date_naive();
            date >= s && date <= e
        } else {
            false
        }
    }

    pub fn is_recurring(&self) -> bool {
        self.rrule.is_some()
    }
}

// ── EventDependency ──────────────────────────────────────────────────

/* A directed dependency edge between two events. Maps to `event_dependencies` table. */
