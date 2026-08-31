use std::collections::HashSet;

use anyhow::Error as AnyError;
use rusqlite::{Connection, Error as SqliteError, ErrorCode};
use uuid::Uuid;

use crate::{
    db,
    google::discovery::DiscoveredGoogleCalendar,
    models::{Calendar, CalendarSource},
    sync::state::CalendarSyncState,
};

include!("calendar_service/types.rs");
include!("calendar_service/mutations.rs");
include!("calendar_service/validation.rs");

#[cfg(test)]
include!("calendar_service/tests.rs");
