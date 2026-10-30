use super::palette::Command;
use super::*;
use crate::effects::{engine_toggle, Effects, NATIVE};
use std::time::Instant;

impl App {
    /// Scheduler contract: only visible, enabled, live layers request frames.
    pub fn animation_active(&self) -> bool {
        if self.pending_confirm.is_some() {
            return false;
        }
        self.effects.active(&self.view)
    }

    pub(super) fn effect_commands(&self) -> Vec<Command> {
        let pref = &self.effects.preferences;
        let mut commands = vec![
            Command {
                label: format!("Effects: all [{}]", if pref.enabled { "ON" } else { "OFF" }).into(),
                action: Action::ToggleEffect("all".into()),
            },
            Command {
                label: "Credits and glyphfx gallery".into(),
                action: Action::Credits,
            },
        ];
        commands.extend(
            NATIVE
                .iter()
                .map(|name| (*name).to_owned())
                .chain(crate::glyphfx::EFFECTS.iter().map(|e| engine_toggle(e)))
                .map(|name| Command {
                    label: format!(
                        "Effect: {name} [{}]",
                        if pref.individual.get(&name).copied().unwrap_or(true) {
                            "ON"
                        } else {
                            "OFF"
                        }
                    )
                    .into(),
                    action: Action::ToggleEffect(name),
                }),
        );
        commands
    }
    pub(super) fn toggle_effect(&mut self, name: &str) {
        let previous = self.effects.preferences.clone();
        if name == "all" {
            self.effects.preferences.enabled = !previous.enabled;
        } else {
            let on = previous.individual.get(name).copied().unwrap_or(true);
            self.effects
                .preferences
                .individual
                .insert(name.to_owned(), !on);
        }
        match self.effects.preferences.save() {
            Ok(()) => {
                self.effects.cancel();
                self.set_status(format!("Effects preference saved: {name}"), false);
            }
            Err(e) => {
                self.effects.preferences = previous;
                self.set_status(format!("Cannot save effects preferences: {e}"), true);
            }
        }
    }
    pub(super) fn credits_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.effects.credits = false;
                self.effects.cancel();
            }
            KeyCode::Char('c' | 'q') if key.modifiers == KeyModifiers::CONTROL => {
                self.running = false
            }
            KeyCode::Right | KeyCode::Left => {
                let n = crate::glyphfx::EFFECTS
                    .iter()
                    .filter(|e| self.effects.engine_on(e))
                    .count();
                if n > 0 {
                    self.effects.gallery_index = (self.effects.gallery_index
                        + if key.code == KeyCode::Right { 1 } else { n - 1 })
                        % n;
                    self.effects.cancel();
                }
            }
            _ => {}
        }
    }
    pub(super) fn proposal_ready_effect(&mut self) {
        self.effects.task_labels = self
            .planner_tasks
            .iter()
            .map(|t| {
                (
                    t.id.clone(),
                    (t.title.clone(), t.target_calendar_id.clone()),
                )
            })
            .collect();
        self.effects.task_events.clear();
        self.effects.proposal = Some(Instant::now());
        self.effects.applied = None;
        self.effects.pending_ids.clear();
        self.effects.applied_ids.clear();
    }

    pub(super) fn invalidate_proposal(&mut self) {
        if self
            .planner_proposal
            .as_ref()
            .is_some_and(|p| p.proposal.status == "ready")
        {
            self.planner_proposal = None;
            self.effects.proposal = None;
        }
    }
    pub(super) fn proposal_applied_effect(&mut self, id: &str) {
        // Read committed identity links, never infer identity from titles or times.
        // A read-only connection cannot seed calendars, migrate or mutate the DB.
        let read_ids = || -> rusqlite::Result<std::collections::HashMap<String, String>> {
            let conn = rusqlite::Connection::open_with_flags(
                crate::db::db_path(),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            conn.busy_timeout(std::time::Duration::ZERO)?;
            let mut stmt = conn.prepare(
                "SELECT task_id,event_id FROM planning_task_events WHERE proposal_id=?1",
            )?;
            let rows = stmt.query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect()
        };
        self.effects.task_events = match read_ids() {
            Ok(ids) => ids,
            Err(e) => {
                self.effects.diagnostic = Some(format!(
                    "Applied successfully; visual identity lookup unavailable: {e}"
                ));
                Default::default()
            }
        };
        self.effects.pending_ids = self.effects.task_events.values().cloned().collect();
        self.effects.applied = None;
        self.effects.proposal = None;
    }
    pub(super) fn events_loaded_effect(&mut self) {
        if self.effects.pending_ids.is_empty() {
            return;
        }
        let ids: Vec<String> = self
            .events
            .iter()
            .filter(|e| self.effects.pending_ids.contains(&e.id))
            .map(|e| e.id.clone())
            .collect();
        if !ids.is_empty() {
            self.effects.applied_ids = ids;
            self.effects.pending_ids.clear();
            self.effects.applied = Some(Instant::now());
        }
    }
    pub fn applied_event_style(&self, id: &str) -> bool {
        self.effects.on("apply sweep")
            && Effects::age(self.effects.applied) < 1.1
            && self.effects.applied_ids.iter().any(|i| i == id)
    }

    pub(super) fn planner_row_count(&self) -> usize {
        if !self.planner_tasks.is_empty() {
            self.planner_tasks.len()
        } else {
            self.planner_proposal.as_ref().map_or(0, |p| p.items.len())
        }
    }
}
