use std::collections::BTreeMap;

use anyhow::Context;
use chrono::NaiveDateTime;
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    calendar_service::{self, CreateCalendarInput, UpdateCalendarInput},
    dag::EventDag,
    db, event_service, google, models, planner,
};

// Keep the public CLI entrypoint stable while each command family remains in a
// focused implementation file.
include!("cli/commands.rs");
include!("cli/arguments_calendar_events.rs");
include!("cli/arguments_planner_google.rs");
include!("cli/runtime.rs");
include!("cli/handlers_calendar_project.rs");
include!("cli/handlers_event_dependency.rs");
include!("cli/handlers_task_planner.rs");
include!("cli/handlers_google.rs");
include!("cli/google_backend.rs");
include!("cli/google_overrides.rs");
include!("cli/validation.rs");

#[cfg(test)]
mod tests;
