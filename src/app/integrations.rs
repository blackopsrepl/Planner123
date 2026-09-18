use super::*;
impl App {
    pub(super) fn toggle_calendar_visibility(&mut self) {
        if let Some(cal) = self.calendars.get_mut(self.calendar_list_index) {
            cal.visible = !cal.visible;
        }
    }

    // ── iCal import ───────────────────────────────────────────────

    pub(super) fn open_ical_import(&mut self) {
        if self.calendars.is_empty() {
            self.set_status("No calendar available for import.", true);
            return;
        }
        self.ical_import_path.clear();
        self.ical_import_calendar_index = self.calendar_list_index.min(self.calendars.len() - 1);
        self.ical_import_field_index = 0;
        self.view = View::IcalImport;
    }

    pub(super) fn ical_import_next_field(&mut self) {
        if self.ical_import_field_index < 1 {
            self.ical_import_field_index += 1;
        }
    }

    pub(super) fn ical_import_prev_field(&mut self) {
        if self.ical_import_field_index > 0 {
            self.ical_import_field_index -= 1;
        }
    }

    pub(super) fn ical_import_input_char(&mut self, c: char) {
        match self.ical_import_field_index {
            0 => self.ical_import_path.push(c),
            1 => {
                if (c == 'h' || c == '-') && self.ical_import_calendar_index > 0 {
                    self.ical_import_calendar_index -= 1;
                } else if (c == 'l' || c == '+')
                    && self.ical_import_calendar_index + 1 < self.calendars.len()
                {
                    self.ical_import_calendar_index += 1;
                }
            }
            _ => {}
        }
    }

    pub(super) fn ical_import_input_backspace(&mut self) {
        if self.ical_import_field_index == 0 {
            self.ical_import_path.pop();
        }
    }

    pub(super) fn ical_import_submit(&mut self) {
        let path = self.ical_import_path.trim().to_string();
        if path.is_empty() {
            self.set_status("Import path cannot be empty.", true);
            return;
        }
        let Some(calendar_id) = self
            .calendars
            .get(self.ical_import_calendar_index)
            .map(|calendar| calendar.id.clone())
        else {
            self.set_status("No calendar selected for import.", true);
            return;
        };

        self.loading = true;
        self.set_status(format!("Importing {}…", path), false);
        self.worker
            .import_ical(calendar_id, path, crate::time::local_timezone_name());
    }

    // ── Google ────────────────────────────────────────────────────

    pub(super) fn google_manage(&mut self) {
        if self.google_client.is_some() {
            self.view = View::GoogleManage;
            if self.google_discovered_calendars.is_empty() {
                self.discover_google_calendars();
            }
        } else {
            self.view = View::GoogleAuth;
        }
    }

    pub(super) fn discover_google_calendars(&mut self) {
        let client_opt = self.google_client.clone();
        if let Some(client) = client_opt {
            self.loading = true;
            self.set_status("Loading Google calendars…", false);
            self.worker.discover_google_calendars(client);
        } else {
            self.view = View::GoogleAuth;
        }
    }

    pub(super) fn import_selected_google_calendar(&mut self) {
        let Some(selected) = self
            .google_discovered_calendars
            .get(self.google_discovery_index)
            .cloned()
        else {
            self.set_status("No Google calendar selected.", true);
            return;
        };

        if self
            .calendars
            .iter()
            .any(|calendar| calendar.google_id.as_deref() == Some(selected.google_id.as_str()))
        {
            self.set_status("Google calendar already imported.", true);
            return;
        }

        match crate::db::open().and_then(|conn| {
            crate::calendar_service::import_google_calendar(&conn, &selected, None)
                .map_err(|e| anyhow::anyhow!(e.to_string()))
        }) {
            Ok(_) => {
                self.worker.load_calendars();
                self.worker.load_calendar_sync_states();
                self.set_status(
                    format!("Imported Google calendar '{}'.", selected.name),
                    false,
                );
            }
            Err(err) => self.set_status(format!("Google import failed: {}", err), true),
        }
    }

    pub(super) fn google_logout(&mut self) {
        match crate::google::auth::GoogleClient::logout() {
            Ok(_) => {
                self.google_client = None;
                self.google_discovered_calendars.clear();
                self.view = View::Month;
                self.set_status("Google disconnected.", false);
            }
            Err(err) => self.set_status(format!("Google logout failed: {}", err), true),
        }
    }

    pub(super) fn google_sync(&mut self) {
        // Clone what we need before taking any mutable borrows
        let client_opt = self.google_client.clone();
        if let Some(client) = client_opt {
            self.loading = true;
            self.set_status("Syncing with Google Calendar…", false);
            let google_cals = self
                .calendars
                .iter()
                .filter(|c| c.source == crate::models::CalendarSource::Google)
                .cloned()
                .collect::<Vec<_>>();
            if google_cals.is_empty() {
                self.set_status("No Google calendars configured. Press G to set up.", false);
                self.loading = false;
            } else {
                self.worker.google_sync(google_cals, client);
            }
        } else {
            self.view = View::GoogleAuth;
        }
    }

    pub(super) fn complete_google_auth(&mut self) {
        let client_id = self.google_auth_client_id.clone();
        let client_secret = self.google_auth_client_secret.clone();

        if client_id.is_empty() {
            self.set_status("Client ID is required.", true);
            return;
        }

        self.set_status("Opening browser for Google authorization…", false);
        self.view = View::GoogleManage;
        self.loading = true;
        self.worker.complete_google_auth(client_id, client_secret);
        self.set_status("Waiting for browser authorization…", false);
    }

    // ── iCal export ───────────────────────────────────────────────

    pub(super) fn export_ical(&mut self) {
        let path = dirs::home_dir().unwrap_or_default().join("planner123.ics");

        match crate::ical::export_to_file(&self.events, "Planner123", &path) {
            Ok(()) => self.set_status(format!("Exported to {}", path.display()), false),
            Err(e) => self.set_status(format!("Export failed: {}", e), true),
        }
    }

    // ── Selected event helpers ─────────────────────────────────────

    pub(super) fn delete_selected_event(&mut self) {
        if let Some(event) = self.selected_event().cloned() {
            self.worker.delete_event(event.id);
        }
    }

    pub(super) fn select_event(&mut self) {
        // In month view, Select means switch to day view for the focused date
        if self.view == View::Month {
            self.view = View::Day;
        }
    }
}
