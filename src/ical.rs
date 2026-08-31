/* iCal (.ics) import/export.  */

use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use icalendar::{Calendar as ICalCalendar, Component, Event as ICalEvent, EventLike};
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    db, event_service,
    models::{Calendar, Event},
};

include!("ical/types.rs");
include!("ical/export.rs");
include!("ical/import.rs");
include!("ical/parsing.rs");
include!("ical/candidates.rs");
include!("ical/times.rs");

#[cfg(test)]
include!("ical/tests.rs");
