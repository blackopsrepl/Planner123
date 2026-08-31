use anyhow::Error as AnyError;
use rusqlite::Connection;
use uuid::Uuid;

use crate::{
    db,
    models::{CalendarSource, Event},
    sync::state,
    time,
};

include!("event_service/error.rs");
include!("event_service/mutations.rs");
include!("event_service/validation.rs");

#[cfg(test)]
include!("event_service/tests.rs");
