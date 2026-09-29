use super::*;

/* Confirmations for the actions that cannot be undone.

Deleting an event and dumping every event to a file both used to happen on a
single keystroke — `d` in the calendar, `x` anywhere — with no way back. The
planner already had the right shape (review the proposal, then apply), so these
follow it: the first keystroke arms, `y` commits, anything else cancels with a
message. Nothing else is bound while a confirmation is armed, so a stray key
can never be the one that deletes something. */

/* What a confirmed action will actually do. */
pub enum ConfirmKind {
    DeleteEvent(String),
    ExportIcal,
}

/* An armed confirmation: the question in the status bar and the work behind it. */
pub struct Confirm {
    pub prompt: String,
    pub kind: ConfirmKind,
}

impl App {
    /* `d`: ask before deleting the selected event. */
    pub(super) fn request_delete(&mut self) {
        match self.selected_event() {
            Some(event) => {
                self.pending_confirm = Some(Confirm {
                    prompt: format!("Delete \u{201c}{}\u{201d}?", event.title),
                    kind: ConfirmKind::DeleteEvent(event.id.clone()),
                });
            }
            None => self.set_status("No event selected.", false),
        }
    }

    /* Exporting writes every event to a fixed path, so it asks too. */
    pub(super) fn request_export(&mut self) {
        self.pending_confirm = Some(Confirm {
            prompt: format!("Export all {} events to planner123.ics?", self.events.len()),
            kind: ConfirmKind::ExportIcal,
        });
    }

    pub(super) fn cancel_confirm(&mut self) {
        self.pending_confirm = None;
        self.set_status("Cancelled.", false);
    }

    /* The user pressed `y`. */
    pub(super) fn run_confirm(&mut self, confirm: Confirm) {
        self.pending_confirm = None;
        match confirm.kind {
            ConfirmKind::DeleteEvent(id) => {
                self.loading = true;
                self.worker.delete_event(id);
            }
            ConfirmKind::ExportIcal => self.export_ical(),
        }
    }
}
