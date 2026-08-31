#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDependency {
    pub id: String,
    pub from_event_id: String, // the blocking event
    pub to_event_id: String,   // the event that is blocked
    pub dependency_type: DependencyType,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Blocks,  // from must finish before to can start
    Related, // soft informational link
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Blocks => write!(f, "blocks"),
            DependencyType::Related => write!(f, "related"),
        }
    }
}

// ── Planner inbox ───────────────────────────────────────────────────
