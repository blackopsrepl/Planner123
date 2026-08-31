use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::Event;

include!("types/data.rs");
include!("types/mapping.rs");

#[cfg(test)]
include!("types/tests.rs");
