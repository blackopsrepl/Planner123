use anyhow::Result;

pub use crate::sync::engine::SyncCalendarReport;

pub async fn sync_calendar(
    client: &crate::google::auth::GoogleClient,
    calendar: &crate::models::Calendar,
) -> Result<SyncCalendarReport> {
    crate::sync::engine::sync_calendar(client, calendar).await
}
