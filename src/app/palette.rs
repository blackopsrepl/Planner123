use super::*;

/* The command palette.

It exists for the actions that do not earn a letter: `.ics` import and export,
Google connection and discovery, the planner settings, jump to date. Typing
filters the list; Enter runs the selection against the same dispatch path a key
would use, so a palette command can never do something a key cannot. */

/* One palette entry. */
pub struct Command {
    pub label: &'static str,
    pub action: Action,
}

/* Everything the palette offers, in display order. */
pub fn commands() -> Vec<Command> {
    vec![
        Command {
            label: "Go to date…",
            action: Action::JumpToDate,
        },
        Command {
            label: "Go to today",
            action: Action::JumpToday,
        },
        Command {
            label: "Month view",
            action: Action::ViewMonth,
        },
        Command {
            label: "Week view",
            action: Action::ViewWeek,
        },
        Command {
            label: "Day view",
            action: Action::ViewDay,
        },
        Command {
            label: "Agenda view",
            action: Action::ViewAgenda,
        },
        Command {
            label: "New event",
            action: Action::CreateEvent,
        },
        Command {
            label: "Quick add event",
            action: Action::QuickAdd,
        },
        Command {
            label: "Planner inbox",
            action: Action::PlannerInbox,
        },
        Command {
            label: "Planner settings",
            action: Action::PlannerSettings,
        },
        Command {
            label: "Sync with Google Calendar",
            action: Action::GoogleSync,
        },
        Command {
            label: "Google account and calendars",
            action: Action::GoogleManage,
        },
        Command {
            label: "Sign in to Google",
            action: Action::GoogleLogin,
        },
        Command {
            label: "Sign out of Google",
            action: Action::GoogleAuthLogout,
        },
        Command {
            label: "Discover Google calendars",
            action: Action::GoogleDiscoverCalendars,
        },
        Command {
            label: "Import .ics file",
            action: Action::ImportIcal,
        },
        Command {
            label: "Export .ics file",
            action: Action::ExportIcal,
        },
        Command {
            label: "Toggle selected calendar",
            action: Action::ToggleCalendar,
        },
        Command {
            label: "Quit",
            action: Action::Quit,
        },
    ]
}

/* Case-insensitive subsequence match: "gcal" finds "Sync with Google
Calendar", "exp" finds "Export .ics file". An empty query matches everything. */
pub fn fuzzy_match(query: &str, label: &str) -> bool {
    let mut label_chars = label.chars().flat_map(char::to_lowercase);
    query
        .chars()
        .flat_map(char::to_lowercase)
        .all(|wanted| label_chars.any(|candidate| candidate == wanted))
}

impl App {
    pub(super) fn open_palette(&mut self) {
        self.palette_input.clear();
        self.palette_index = 0;
        self.view = View::Palette;
    }

    /* The commands matching what has been typed so far. */
    pub fn palette_matches(&self) -> Vec<Command> {
        commands()
            .into_iter()
            .filter(|command| fuzzy_match(&self.palette_input, command.label))
            .collect()
    }

    pub(super) fn palette_input_char(&mut self, c: char) {
        self.palette_input.push(c);
        self.clamp_palette_index();
    }

    pub(super) fn palette_input_backspace(&mut self) {
        self.palette_input.pop();
        self.clamp_palette_index();
    }

    pub(super) fn palette_up(&mut self) {
        self.palette_index = self.palette_index.saturating_sub(1);
    }

    pub(super) fn palette_down(&mut self) {
        let last = self.palette_matches().len().saturating_sub(1);
        self.palette_index = (self.palette_index + 1).min(last);
    }

    /* Run the selected command through the normal dispatch path. */
    pub(super) fn palette_run(&mut self) {
        let Some(command) = self.palette_matches().into_iter().nth(self.palette_index) else {
            self.set_status("No command matches.", true);
            return;
        };
        let action = command.action;
        let return_view = self.return_view.clone();
        self.switch_view(return_view);
        self.dispatch(action);
    }

    /* Keep the selection inside the filtered list. */
    fn clamp_palette_index(&mut self) {
        let last = self.palette_matches().len().saturating_sub(1);
        self.palette_index = self.palette_index.min(last);
    }

    // ── Go to date ────────────────────────────────────────────────

    pub(super) fn open_date_jump(&mut self) {
        self.date_jump_input.clear();
        self.view = View::DateJump;
    }

    pub(super) fn date_jump_char(&mut self, c: char) {
        self.date_jump_input.push(c);
    }

    pub(super) fn date_jump_backspace(&mut self) {
        self.date_jump_input.pop();
    }

    /* Move the cursor to the typed date, staying open on a bad one. */
    pub(super) fn date_jump_submit(&mut self) {
        let input = self.date_jump_input.trim().to_string();
        if input.is_empty() {
            let return_view = self.return_view.clone();
            self.switch_view(return_view);
            return;
        }

        match parse_date_input(&input, Local::now().date_naive()) {
            Ok(date) => {
                self.focused_date = date;
                self.view_month = date.month();
                self.view_year = date.year();
                let return_view = self.return_view.clone();
                self.switch_view(return_view);
                self.set_status(
                    format!("Jumped to {}", date.format("%A, %b %-d, %Y")),
                    false,
                );
            }
            Err(message) => self.set_status(message, true),
        }
    }
}

/* An absolute ISO date, or a day offset from today: `+21`, `-7`. */
pub fn parse_date_input(input: &str, today: NaiveDate) -> Result<NaiveDate, String> {
    if let Some(offset) = input.strip_prefix('+').or_else(|| input.strip_prefix('-')) {
        let sign = if input.starts_with('-') { -1 } else { 1 };
        let days: i64 = offset
            .trim()
            .parse()
            .map_err(|_| "Offset must be a number of days, for example +21 or -7.".to_string())?;
        return Ok(today + Duration::days(sign * days));
    }

    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .map_err(|_| "Enter a date as YYYY-MM-DD, or an offset like +21 / -7.".to_string())
}
