#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEventListResponse {
    #[serde(default)]
    pub items: Vec<GoogleEvent>,
    #[serde(rename = "nextSyncToken")]
    pub next_sync_token: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEvent {
    pub id: Option<String>,
    pub etag: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<GoogleEventTime>,
    pub end: Option<GoogleEventTime>,
    pub recurrence: Option<Vec<String>>,
    pub updated: Option<String>,
    #[serde(rename = "recurringEventId")]
    pub recurring_event_id: Option<String>,
    #[serde(rename = "originalStartTime")]
    pub original_start_time: Option<GoogleEventTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}
