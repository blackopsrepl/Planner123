# PRD: Planner123 v0.4.x

Status: Draft
Date: 2026-04-12
Repo: `Planner123`
Current release baseline: `v0.3.0` on `main`

## 1. Executive Summary

Planner123 has reached a solid `v0.3.0` baseline for local calendar management, a JSON-first automation CLI, and inbound Google Calendar synchronization. The next release must turn that foundation into a product that is honest, elegant, compliant, and durable.

This PRD defines the next shipping milestone:

- Make all public product claims true.
- Add compliant Google account connection and calendar discovery/import.
- Deliver true two-way sync for writable Google calendars.
- Fix timezone correctness in the TUI and shared event flows.
- Implement real `.ics` import.
- Improve TUI and CLI UX without compromising the repo's small, maintainable architecture.

This document is written so a clean agent can execute from scratch without rediscovering the repo.

## 2. Why This Exists

The repo is currently in an in-between state:

- The codebase is healthy and CI passes.
- The CLI and Google sync internals grew materially in the last release cycle.
- Several product claims and help surfaces are ahead of reality.
- Google sync is currently inbound only.
- The TUI still creates events in `UTC` by default.
- `.ics` export exists, but `.ics` import does not.

The next milestone must close the gap between product promise and shipped behavior without turning the codebase into a sync-heavy mess.

## 3. Current Repo Snapshot

The current architecture is small and understandable:

- `src/app.rs`: TUI state machine and user actions.
- `src/worker.rs`: background task dispatch.
- `src/cli.rs`: JSON CLI parsing and command handling.
- `src/db.rs`: schema and CRUD.
- `src/calendar_service.rs`: shared calendar business rules.
- `src/google/auth.rs`: OAuth credential handling and refresh.
- `src/google/discovery.rs`: Google calendar discovery.
- `src/google/sync.rs`: inbound calendar event sync.

Known gaps in current behavior:

- README claims "incremental two-way sync", but `src/google/sync.rs` only fetches remote changes and applies them locally.
- Help and README claim `.ics` import support, but `Action::ImportIcal` only shows a status message.
- TUI event creation hardcodes `UTC`.
- Google OAuth flow uses a fixed port and does not yet implement the stronger installed-app security posture expected by the official docs.
- Current Google event handling is based on partial JSON parsing and optimistic assumptions rather than a conflict-aware sync engine.

## 4. Product Goals

### Primary goals

- Deliver true two-way Google Calendar sync for writable imported Google calendars.
- Keep the TUI responsive and non-blocking during all sync work.
- Keep the CLI strictly non-interactive and safe for automation.
- Preserve existing local-calendar behavior and deterministic testability.
- Make time and timezone behavior correct and predictable.
- Keep the code elegant: small modules, explicit ownership, shared domain rules, low duplication.

### User experience goals

- A user can connect Google once, see which calendars are available, import the right ones, and understand which are writable vs read-only.
- A user can create, edit, and delete events in an imported writable Google calendar from either the TUI or CLI and see those changes reach Google safely.
- A user can tell whether sync is healthy, pending, conflicted, or blocked by re-authentication.
- A user never loses data silently.

### Developer experience goals

- A clean agent can identify the right files, services, migrations, tests, and docs from this PRD alone.
- Shared business rules live below the TUI and CLI, not duplicated inside them.
- CI remains deterministic and does not require live Google access.
- Sync code is testable through fake backends and fixture payloads, not through brittle end-to-end network tests.

## 5. Non-Goals

These are not part of this milestone unless explicitly pulled in later:

- Outlook, CalDAV, or multi-provider sync.
- Multi-account Google support.
- Real-time push notifications from Google webhooks.
- Mobile clients.
- Collaborative conflict editing UI beyond the scoped resolution workflow below.
- Full attendee-management parity with Google Calendar.
- Full conference-data creation and attachment-management parity.

## 6. Non-Negotiable Product And Engineering Principles

### Product truth

- Do not claim support for behavior that is not actually shipped.
- Do not label sync as "two-way" until local writes to Google are complete for the scoped feature set in this PRD.
- Do not advertise `.ics` import until it is implemented end-to-end.

### Code quality

- No new god-files.
- No UI layer should contain business rules that also matter to the CLI.
- No sync path may ignore API or DB errors.
- No new code may silently swallow conflicts.
- Prefer typed DTOs and domain structs over repeated `serde_json::Value` indexing once a payload is used beyond a thin parsing boundary.
- Add comments only where the code would otherwise be hard to reason about.

### Wiring quality

- TUI and CLI must call shared services for event and calendar behavior.
- DB helpers remain persistence-focused, not policy-focused.
- Google HTTP adapters must be thin and explicit.
- Sync orchestration must be isolated from UI and parsing code.
- Timezone conversion must have one canonical implementation path.

### User safety

- Never silently overwrite a newer remote event version.
- Never clear unmodeled Google event fields by accident.
- Never log tokens, refresh tokens, authorization codes, or raw keyring secrets.

## 7. Users And Jobs

### User type A: keyboard-first local user

Job: manage calendars and events in the TUI, with optional Google sync, without the app freezing or feeling unreliable.

### User type B: automation agent

Job: inspect calendars and events, mutate them safely through JSON, and trigger sync without prompts or UI state.

### User type C: maintainer

Job: evolve the sync engine safely, reproduce issues locally, and trust CI to catch regressions without live Google credentials.

## 8. Release Theme

Release theme: "Truthful sync, timezone correctness, and elegant wiring."

This release is successful if the product feels like one coherent system instead of a local app with a partially attached Google bridge.

## 9. Scope Overview

### Must ship in this PRD scope

- Google OAuth hardening and official-flow compliance for desktop use.
- Google calendar discovery and import UX.
- Writable vs read-only Google calendar handling.
- True two-way sync for supported Google calendars.
- Safe conflict detection and resolution.
- TUI and CLI use shared event services.
- Timezone-correct create/edit flows.
- Real `.ics` import in both shared logic and at least one user-facing entrypoint.
- Docs, help, wireframes, and README aligned with actual behavior.

### May ship after core sync if schedule is tight

- Auto-sync on startup or timed interval.
- More elaborate sync dashboards.
- Rich conflict-resolution UI beyond a first solid modal/CLI flow.
- Recurring instance exception write support if it risks quality.

## 10. Functional Requirements

### 10.1 Product truth and cleanup

- Remove or correct any user-facing claims that are currently ahead of implementation before merging partial work.
- Align README keybindings with the real keymap.
- Align help text, README, and actual `.ics` behavior.
- Align "two-way sync" wording with the real shipped scope.

### 10.2 Google account connection

The product must support a compliant installed-app OAuth flow for desktop usage.

Requirements:

- Use a Desktop OAuth client type.
- Use the system browser, not an embedded browser.
- Use a loopback redirect on a random available local port, not a hardcoded fixed port.
- Use PKCE with `S256`.
- Use a random `state` value and validate it on callback.
- Keep refresh-token handling in the OS keyring.
- Handle `invalid_grant` by marking the account as disconnected and prompting for reconnect.
- Keep CLI non-interactive even if it can initiate a browser-based auth flow.

Product behavior:

- TUI must expose a clear "Connect Google" flow.
- CLI must expose a machine-safe auth status command and may expose a browser-launching login command that returns JSON status.
- TUI and CLI must both surface whether Google is connected, disconnected, or needs re-auth.

### 10.3 Google calendar discovery and import

Users must be able to discover calendars from Google and choose which to import locally.

Requirements:

- Use `calendarList.list`.
- Surface `primary`, color, name, and access role.
- Distinguish writable calendars from read-only calendars.
- Prevent duplicate imports of the same Google calendar.
- Preserve local calendar ordering semantics.
- Keep imported calendars visible by default unless the user chooses otherwise.

UX behavior:

- TUI must expose a Google calendar picker/import flow.
- CLI must support discovery and import.
- Read-only Google calendars may be imported as read-only mirrors, but the product must not pretend they support outbound writes.

### 10.4 True two-way sync

Two-way sync means:

- Remote changes arrive locally.
- Local creates on writable Google calendars create remote Google events.
- Local updates on writable Google calendars patch remote Google events.
- Local deletes on writable Google calendars delete remote Google events.
- Sync is idempotent and resumable.
- Conflicts are detected and surfaced rather than hidden.

Core rules:

- Local changes apply to the local DB immediately for responsiveness.
- Local writes enqueue outbound sync work instead of blocking the UI.
- Manual sync runs both inbound and outbound phases.
- TUI background work remains non-blocking.
- CLI `google sync` remains explicit and deterministic.
- Incremental pull sync must obey official `syncToken` parameter constraints and keep deleted events in scope.

### 10.5 Conflict handling

Conflict behavior must be explicit and safe.

Requirements:

- Use remote etags and `If-Match` for update and delete requests.
- On `412 Precondition Failed`, fetch the latest remote event, persist a conflict record, and mark the event as conflicted.
- Do not silently discard local changes.
- Do not blindly overwrite remote changes.
- Expose conflict status in both TUI and CLI.
- Provide an initial resolution path with at least `keep-local` and `keep-remote`.

Initial UX:

- TUI may show a conflict badge and resolution modal.
- CLI must support listing conflicts and resolving them.

### 10.6 Timezone correctness

Timezone behavior must become correct across local and Google-synced events.

Requirements:

- The TUI must not hardcode `UTC` for new events.
- Event create and update flows must normalize authored wall-clock time through a shared timezone conversion layer.
- Stored `start_at` and `end_at` semantics must be documented and enforced consistently.
- Google ingress must preserve event timezone metadata when available.
- Rendering and edit forms must round-trip time values correctly.

Product rule:

- If a user creates an event in local time, the saved event must display the same intended wall-clock time when reopened.

### 10.7 `.ics` import

`.ics` import must be real, not implied.

Requirements:

- Add shared `.ics` parsing/import logic.
- Preserve title, description, location, all-day flag, start/end, and RRULE when supported.
- Report unsupported fields clearly instead of failing silently.
- Support importing into a chosen calendar.
- Keep export working.

User entrypoint:

- TUI `i` must trigger a real import path.
- CLI must expose an import command.

### 10.8 TUI UX

The TUI must remain fast, legible, and quiet under error.

Requirements:

- `G` opens Google management or connection.
- `S` triggers sync now.
- Status bar shows connection state, sync progress, and errors without overwhelming the main view.
- Imported read-only calendars show a lock or equivalent clear state.
- Sync conflicts and per-calendar failures are visible.
- Event forms on Google calendars must communicate whether edits will sync.

### 10.9 CLI UX

The CLI remains the automation contract.

Requirements:

- JSON success on stdout.
- JSON error on stderr.
- No prompts.
- No interactive confirmations.
- Destructive actions remain explicit.
- Google subcommands must be discoverable and composable.

Required command groups to add or refine:

- `google auth status`
- `google auth login`
- `google auth logout`
- `google calendars discover`
- `google calendars import`
- `google sync`
- `google sync status`
- `google conflicts list`
- `google conflicts resolve`
- `ical import`

The exact final command shape may vary, but the capabilities above must exist.

## 11. Google API Compliance Requirements

This repo must stay compliant with current official Google guidance for installed apps and the Calendar API.

Non-negotiable rules:

- Use documented Google OAuth installed-app flows only.
- Use a system browser and a loopback redirect for desktop.
- Use PKCE and `state`.
- Use only documented Calendar API endpoints.
- Use documented scopes and prefer the narrowest scope set that supports the shipped behavior.
- Handle `410 Gone` by clearing sync state and performing a full resync.
- Handle `403` and `429` with exponential backoff.
- Handle `412` with re-fetch and conflict resolution, not blind overwrite.
- Use `events.patch` by default for partial outbound updates unless a full merged-resource update path is implemented safely.
- Honor `syncToken` restrictions on `events.list` and `calendarList.list`, and keep deleted items included where the official docs require it.
- Do not use deprecated OOB redirect behavior.
- Do not default to `sendUpdates=none` for synced user actions because the official docs warn about adverse sync effects.
- Do not embed browser content inside the TUI.

Recommended scope strategy:

- Prefer a narrow scope set if it cleanly supports discovery plus writable event sync.
- If the narrow scope set becomes operationally messy or misleading, use `https://www.googleapis.com/auth/calendar` and document why.
- The final scope decision must be written down in README and auth help text.

Official references to follow:

- Google OAuth for installed desktop apps.
- Calendar `calendarList.list`.
- Calendar `events.list`.
- Calendar `events.insert`.
- Calendar `events.patch`.
- Calendar `events.delete`.
- Calendar error handling guidance.
- Calendar versioned resources and etag guidance.

## 12. Supported Sync Scope

The product may only market "true two-way sync" for the feature set it actually preserves safely.

### In scope for the first true two-way milestone

- Single events.
- All-day events.
- Basic recurring master events represented by RRULE.
- Title.
- Description.
- Location.
- Start/end.
- All-day flag.
- Recurrence rule.
- Calendar assignment within the imported Google calendar.

### Must be preserved even if not fully editable locally

- Unmodeled remote fields must not be cleared accidentally.
- Existing remote attendees must not be wiped by a local title-only or time-only change.
- Existing conference data must not be wiped by unrelated edits.

### May be deferred if quality would suffer

- Detached recurring-instance edits.
- Rich attendee editing.
- Conference-data creation.
- Attachments.

If any of the deferred cases remain unsupported at release time:

- document them clearly.
- preserve remote data on sync.
- do not claim parity.

## 13. Data Model And Migration Plan

The current schema is not enough for safe two-way sync. Add forward-only migrations.

### Required calendar metadata

Add calendar fields or an equivalent auxiliary table for:

- Google access role.
- writable vs read-only state.
- last successful sync timestamp.
- last sync error code.
- last sync error message.

### Required event sync metadata

Keep existing `google_id` and `google_etag`, and add either event-level fields or a dedicated sync-state table for:

- sync state: `clean`, `pending_create`, `pending_update`, `pending_delete`, `conflicted`, `error`
- last synced at
- last push attempt at
- last remote modified at
- last sync error code
- last sync error message
- conflict payload or reference

### Required outbound queue

Add a dedicated outbox table rather than abusing ad-hoc booleans.

Suggested shape:

- `id`
- `provider`
- `calendar_id`
- `event_id`
- `operation`
- `enqueued_at`
- `attempt_count`
- `last_attempt_at`
- `last_error_code`
- `last_error_message`

Queue rules:

- One effective pending operation per event/provider pair.
- coalesce repeated updates.
- delete supersedes pending create or update where appropriate.
- queue writes inside the same transaction as the local DB mutation.

### Required conflict storage

Add either a dedicated conflict table or a structured JSON conflict payload attached to sync state.

Minimum stored data:

- event id
- calendar id
- local snapshot
- remote snapshot
- remote etag
- conflict detected at
- resolution status

## 14. Architecture And Wiring Plan

### Required module boundaries

The implementation must be split into explicit layers.

Recommended structure:

- `src/event_service.rs`
- `src/calendar_service.rs`
- `src/google/auth.rs`
- `src/google/calendar_list.rs`
- `src/google/events_api.rs`
- `src/google/types.rs`
- `src/sync/engine.rs`
- `src/sync/pull.rs`
- `src/sync/push.rs`
- `src/sync/conflicts.rs`
- `src/sync/state.rs`

Exact file names can vary, but the boundaries must exist.

### Service ownership

- `db.rs` owns SQL and row mapping only.
- event service owns validation, timezone normalization, soft delete rules, and outbox enqueue behavior.
- calendar service owns import and calendar-state rules.
- Google adapters own HTTP request and response translation.
- sync engine owns sequencing, backoff, retry policy, token handling, and conflict transitions.
- TUI and CLI own presentation only.

### No direct wiring violations

- `src/app.rs` must not write Google sync business rules inline.
- `src/cli.rs` must not duplicate event mutation logic that the TUI also needs.
- worker tasks must call services, not raw business logic spread across the app.
- library code must not `eprintln!` operational sync failures directly; route through structured errors or logging.

## 15. Detailed UX Requirements

### 15.1 Google management in TUI

Add a coherent Google management surface.

Must support:

- show connection state
- connect or reconnect
- list discoverable calendars
- import calendars
- mark read-only calendars
- show last sync result
- show pending outbound changes
- show conflicts

### 15.2 Status bar

Status bar behavior must be high signal.

Show:

- disconnected
- connected
- syncing
- sync complete
- re-auth required
- conflicts present
- last sync failed

Do not:

- spam repetitive status messages
- hide actionable errors behind transient messages only

### 15.3 Event forms

Event forms must surface sync implications.

Requirements:

- show the selected calendar clearly
- indicate if the selected calendar is local, writable Google, or read-only Google
- block save into a read-only Google calendar with a clear message
- use a real timezone value

### 15.4 CLI output

CLI outputs must remain contract-stable.

Requirements:

- continue current `status` plus `data` shape for success
- continue current `status`, `code`, `message` shape for errors
- add structured sync result fields, not vague strings

## 16. Recurrence Requirements

Recurrence is already part of the product surface, so sync cannot treat it as an afterthought.

Requirements:

- preserve RRULE values on import, local edit, export, and Google sync where supported
- do not flatten recurring masters into lossy local data when that would break outbound sync semantics
- define explicit behavior for recurring instances and exceptions before marketing full parity

Implementation guidance:

- Revisit the current `singleEvents=true` pull approach before shipping true two-way sync.
- Prefer a model that can distinguish recurring masters from expanded instances when that distinction matters for writes.
- If detached-instance editing is not shipped in this release, preserve remote data and document the limitation clearly.

## 17. Implementation Plan

Implement in the following order. Do not jump ahead if a lower layer is still structurally wrong.

### Phase 0: Truth alignment and foundation cleanup

- Fix README, help text, and wireframes that currently over-claim behavior.
- correct quick-add keybinding docs.
- add this PRD to repo documentation references if appropriate.
- introduce shared event service and move duplicated mutation rules out of TUI and CLI.

### Phase 1: OAuth and calendar discovery

- Replace the current fixed-port auth flow with compliant desktop OAuth.
- add auth status and reconnect behavior.
- extend calendar discovery to include access role and writable state.
- add TUI and CLI calendar import flows.

### Phase 2: Sync state and outbox

- add DB migrations for calendar metadata, event sync state, outbox, and conflicts.
- route local mutations on Google calendars through the outbox.
- add sync status reporting primitives.

### Phase 3: Inbound sync hardening

- keep incremental sync via `syncToken`.
- honor `syncToken` request-parameter restrictions and deleted-item rules.
- handle `410 Gone` correctly with full resync.
- stop ignoring sync errors.
- preserve remote fields safely.

### Phase 4: Outbound sync

- implement create, patch, and delete for writable Google calendars.
- use etags and `If-Match`.
- add retry and backoff behavior.
- treat already-deleted remote events as terminal success for local delete reconciliation.

### Phase 5: Conflict handling

- persist conflicts.
- expose conflicts in CLI first.
- add TUI conflict visibility and initial resolution flow.

### Phase 6: Timezone and `.ics` completion

- fix TUI timezone defaults and round-trips.
- implement real `.ics` import.
- align export and import behavior with shared event services.

### Phase 7: Polish and release

- update README, AGENT, wireframes, and changelog.
- validate TUI states manually.
- cut release only after docs and product claims are true.

## 18. Acceptance Criteria

The release is done only when all items below are true.

### Product acceptance

- README and help text match the actual feature set.
- a user can connect Google via a compliant installed-app flow.
- a user can discover and import Google calendars.
- writable Google calendars support create, update, delete, and inbound sync.
- read-only Google calendars are clearly marked and protected from local writes.
- the product can recover from an expired sync token via full resync.
- conflict cases surface explicitly and do not silently lose data.
- TUI create/edit flows use the correct timezone behavior.
- `.ics` import works end to end.

### Engineering acceptance

- TUI and CLI event mutations go through shared services.
- sync logic is covered by deterministic tests with fake backends and fixture payloads.
- no library layer prints ad-hoc sync failures directly to stderr.
- no new undocumented env var is required for production behavior.
- migrations are forward-only and safe on an existing `v0.3.0` DB.

### UX acceptance

- the user can tell whether Google is connected, syncing, failed, or conflicted.
- the TUI never blocks on Google network activity.
- error messages tell the user what to do next.
- JSON CLI responses remain machine-friendly.

## 19. Testing Strategy

### Unit tests

Add unit tests for:

- OAuth URL builder and callback validation.
- timezone conversion and wall-clock round-trips.
- Google calendar discovery parsing including access role and filtering.
- Google event mapping.
- outbox coalescing rules.
- conflict transitions.
- sync-token invalidation handling.

### Integration tests

Add integration tests for:

- CLI auth status behavior without credentials.
- calendar discovery and import against a fake backend.
- local event create on Google calendar enqueues outbound work.
- local update patch path.
- local delete path.
- sync full-resync on `410 Gone`.
- `412` conflict detection and resolution.
- read-only calendar mutation rejection.
- `.ics` import command behavior.

### Manual validation

Perform explicit manual checks for:

- desktop OAuth connect and reconnect
- writable calendar import
- read-only calendar import
- create/update/delete in TUI on a Google calendar
- conflict reproduction by editing the same event in Google UI and Planner123
- timezone behavior across local and Google events
- `.ics` import from a representative sample file

### CI rules

- CI must not hit live Google endpoints.
- All Google API behavior in CI must use fake backends or fixture playback.
- `make ci-local` must continue to pass.

## 20. Error Handling Requirements

Minimum handling rules:

- `410 Gone` on `syncToken`: clear calendar sync state, then full resync.
- `412 Precondition Failed`: persist conflict, do not overwrite silently.
- `403 forbiddenForNonOrganizer`: surface a clear write-permission or patch-constraint error.
- `403 quotaExceeded` and `429 rateLimitExceeded`: exponential backoff with bounded retries.
- `404 notFound` on delete: treat as already reconciled if the intent was deletion.
- `invalid_grant`: mark account disconnected and require reconnect.

## 21. Developer Experience Requirements

### Repo ergonomics

- Update `AGENT.md` with the new sync architecture and command surface.
- Update wireframes when TUI or CLI surfaces change.
- Keep `cargo run` as the TUI default.
- Keep the CLI as the stable automation surface.

### Code review bar

Reject implementations that:

- scatter sync logic across UI files
- hardcode special cases in multiple places
- use broad raw JSON mutation where typed mapping would be clearer
- add undocumented silent retries
- hide operational failures

### Logging and diagnostics

- Introduce structured logging if needed, but keep default user output clean.
- Never log secrets.
- make sync diagnostics opt-in and developer-focused.

## 22. Open Product Decisions

These decisions must be made during implementation and documented in the final README.

- Final Google scope set: narrow combo vs full `calendar`.
- Default `sendUpdates` behavior for synced event writes.
- Whether the first release of true two-way sync includes recurring-instance exception writes or explicitly defers them.
- Whether auto-sync on startup ships in the same release.

## 23. Risks And Mitigations

### Risk: destructive remote writes

Mitigation:

- use patch semantics carefully
- use etags
- preserve unmodeled fields
- test against fixtures representing remote-only fields

### Risk: recurrence model mismatch

Mitigation:

- define the supported recurrence write scope explicitly
- do not market unsupported exception editing
- avoid lossy sync strategies

### Risk: auth fragility

Mitigation:

- use random loopback port
- use PKCE and state
- treat reconnect as a first-class UX path

### Risk: codebase sprawl

Mitigation:

- enforce service boundaries
- keep modules focused
- move policy out of `app.rs` and `cli.rs`

## 24. Clean Agent Execution Guide

Any clean agent executing this PRD should follow this order:

1. Read `README.md`, `AGENT.md`, `Makefile`, `src/app.rs`, `src/cli.rs`, `src/db.rs`, `src/calendar_service.rs`, `src/google/auth.rs`, `src/google/discovery.rs`, and `src/google/sync.rs`.
2. Confirm current product-truth mismatches before editing docs.
3. Land shared service boundaries before adding more UI or CLI surface area.
4. Add DB migrations before implementing outbound sync.
5. Implement fake Google backends and fixture coverage before relying on manual validation.
6. Ship auth and discovery before shipping writable sync.
7. Ship writable sync before restoring any "true two-way sync" claim.
8. Ship conflict handling before calling the sync engine safe.
9. Ship timezone fixes before claiming correct Google event round-trips.
10. Update docs, help, wireframes, and changelog last, after behavior is real.

## 25. Definition Of Done

This PRD is complete only when:

- the repo's docs tell the truth
- Google connection is compliant
- writable Google calendars sync both directions
- conflicts are safe and visible
- timezone handling is correct
- `.ics` import is real
- CI is deterministic
- the code remains elegant enough that a new maintainer can still understand it quickly

## 26. Appendix: Official References

The implementation must follow current official Google documentation, especially:

- OAuth 2.0 for iOS and Desktop Apps
  `https://developers.google.com/identity/protocols/oauth2/native-app`
- Calendar API `calendarList.list`
  `https://developers.google.com/workspace/calendar/api/v3/reference/calendarList/list`
- Calendar API `events.list`
  `https://developers.google.com/workspace/calendar/api/v3/reference/events/list`
- Calendar API `events.insert`
  `https://developers.google.com/workspace/calendar/api/v3/reference/events/insert`
- Calendar API `events.patch`
  `https://developers.google.com/workspace/calendar/api/v3/reference/events/patch`
- Calendar API `events.delete`
  `https://developers.google.com/workspace/calendar/api/v3/reference/events/delete`
- Calendar API error handling
  `https://developers.google.com/workspace/calendar/api/guides/errors`
- Calendar API versioned resources and conditional modification guidance
  `https://developers.google.com/workspace/calendar/api/guides/version-resources`
# Planner123 Planner Inbox

The calendar includes an explicit Planner Inbox for unscheduled, contiguous tasks. A batch solve creates a persistent review proposal and only explicit apply creates events. Existing event creation, quick-add, and explicit Google synchronization remain unchanged.

The planner treats all active calendars as busy time, requires successful explicit Google sync state for active Google calendars, and uses configured weekly availability as a hard boundary. Priority, soft deadlines, cognitive preferred windows, and high-cognitive recovery streaks are optimization preferences; hard deadlines and dependencies remain hard constraints.

## v0.4.0 planner interface contract

Planner optimization is user-facing in both supported entrypoints. The Ratatui Planner Inbox configures timezone and weekly availability directly, while the JSON CLI accepts repeatable typed `--availability DAY=HH:MM-HH:MM` values; neither interface exposes persistence JSON. Proposal review reports bounded calendar-event blocker evidence, including recurring occurrences. A proposal is inapplicable after its planner settings or dependency graph changes, and explicit apply revalidates that state transactionally.
