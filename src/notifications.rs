use anyhow::Result;
use chrono::{Duration, Utc};
use notify_rust::Notification;

use crate::models::Event;

/* Send an immediate desktop notification for an event reminder. */
pub fn notify_event(event: &Event, minutes_before: i64) -> Result<()> {
    let when = if minutes_before == 0 {
        "Now".to_string()
    } else if minutes_before < 60 {
        format!("In {} min", minutes_before)
    } else {
        format!("In {} h", minutes_before / 60)
    };

    let body = if let Some(loc) = &event.location {
        format!("{} — {}", when, loc)
    } else {
        when
    };

    Notification::new()
        .summary(&event.title)
        .body(&body)
        .icon("calendar")
        .appname("Planner123")
        .timeout(notify_rust::Timeout::Milliseconds(8000))
        .show()
        .map_err(|e| anyhow::anyhow!("notification failed: {}", e))?;

    Ok(())
}

/* Check a list of events and fire notifications for any that are due. */
/* `already_notified`: set of event IDs that already had a notification this session. */
pub fn check_and_notify(
    events: &[Event],
    already_notified: &mut std::collections::HashSet<String>,
) {
    let now = Utc::now();

    for event in events {
        let reminder_minutes = match event.reminder_minutes {
            Some(m) if m > 0 => m,
            _ => continue,
        };

        let notify_key = format!("{}:{}", event.id, reminder_minutes);
        if already_notified.contains(&notify_key) {
            continue;
        }

        if let Some(start) = event.start_dt() {
            let notify_at = start - Duration::minutes(reminder_minutes);
            // Fire if we're within a 30-second window of the notification time
            let delta = (now - notify_at).num_seconds().abs();
            if delta <= 30 && now <= start {
                if let Err(e) = notify_event(event, reminder_minutes) {
                    eprintln!("notification error: {}", e);
                } else {
                    already_notified.insert(notify_key);
                }
            }
        }
    }
}

/* Spawn a tokio task that checks for reminders every 30 seconds. */
/* `events_arc`: shared list of loaded events. */
pub fn spawn_reminder_task(
    events_arc: std::sync::Arc<tokio::sync::RwLock<Vec<Event>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut already_notified = std::collections::HashSet::new();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let events = events_arc.read().await;
            check_and_notify(&events, &mut already_notified);
        }
    })
}
