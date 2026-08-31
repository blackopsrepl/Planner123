use super::*;

pub fn proposal_applicability(
    conn: &Connection,
    id: &str,
) -> Result<ProposalApplicability, PlannerError> {
    let status: String = conn
        .query_row(
            "SELECT status FROM planner_proposals WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| PlannerError::NotFound {
            resource: "proposal",
            id: id.into(),
        })?;
    let mut reasons = Vec::new();
    if status != "ready" {
        reasons.push("proposal_is_not_ready".into());
    }
    let raw: String = conn
        .query_row(
            "SELECT snapshot_json FROM planner_proposals WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(internal)?;
    let snapshot: ProposalSnapshot = serde_json::from_str(&raw).map_err(internal)?;
    if snapshot.settings_json.is_none() || snapshot.dependencies.is_none() {
        reasons.push("legacy_snapshot_requires_reoptimization".into());
    }
    for (task_id, version) in snapshot.task_versions {
        match require_task(conn, &task_id) {
            Ok(task) if task.updated_at == version => {}
            _ => reasons.push("inbox_task_changed".into()),
        }
    }
    let events: HashMap<_, _> = db::load_events(conn)
        .map_err(internal)?
        .into_iter()
        .map(|event| (event.id, event.updated_at))
        .collect();
    if events != snapshot.event_versions {
        reasons.push("calendar_availability_changed".into());
    }
    if let Some(settings_json) = snapshot.settings_json {
        if canonical_settings_snapshot(&settings(conn)?)? != settings_json {
            reasons.push("planner_settings_changed".into());
        }
    }
    if let Some(dependencies) = snapshot.dependencies {
        if list_dependencies(conn)? != dependencies {
            reasons.push("planner_dependencies_changed".into());
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(ProposalApplicability {
        can_apply: reasons.is_empty(),
        reasons,
    })
}

pub(super) fn validate_snapshot(conn: &Connection, id: &str) -> Result<(), PlannerError> {
    let applicability = proposal_applicability(conn, id)?;
    if !applicability.can_apply {
        return Err(PlannerError::Conflict(format!(
            "proposal is stale: {}",
            applicability.reasons.join(", ")
        )));
    }
    Ok(())
}
