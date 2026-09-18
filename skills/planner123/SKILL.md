---
name: planner123
description: Query, drive, and maintain Planner123 (Rust ratatui calendar with SQLite, planner inbox, Google sync, and .ics import) through its non-interactive JSON CLI. Use when managing calendars, events, projects, dependencies, planner tasks/proposals, Google Calendar sync, or .ics data for planner123, or when building, testing, linting, or releasing this repository. Use ONLY for planner123; not for other calendar tools.
---

# Planner123 — Agent Control Skill

Operate the whole product without the TUI. Everything routes through one
non-interactive CLI that speaks JSON, so agents never need a terminal UI.

## Entry points

| Need | Command |
| --- | --- |
| Stable automation entry (works from any cwd inside the repo) | `./scripts/planner123-cli <args>` |
| Prebuilt binary | `./target/release/planner123-cli <args>` |
| TUI (humans only; never required for agents) | `cargo run` |

`scripts/planner123-cli` is a POSIX sh wrapper that resolves the repo
root and runs `cargo run --bin planner123-cli -- "$@"`. The examples
below abbreviate it to `cli`.

## Response contract

- Success: `{"status":"ok","data":...}` on **stdout**, exit code 0.
- Failure: `{"status":"error","code":"<code>","message":"<explanation>"}` on
  **stderr**, non-zero exit.
- Parsing is strict: unknown flags or malformed values fail fast. There are no
  prompts, confirmations, or choices anywhere in the CLI.
- List endpoints return arrays in `data`. `get`/`create`/`update` return the
  record; IDs are UUID strings. Capture `data.id` from create responses.

## Data isolation (always do this for experiments)

The SQLite database lives at `$XDG_DATA_HOME/planner123/calendar.db`
(default `~/.local/share/planner123/calendar.db`). On first open the binary
copies a pre-rebrand `$XDG_DATA_HOME/solverforge/calendar.db` into the new
location once (`VACUUM INTO` snapshot); after that the legacy file is ignored.
The binary has no `--db`
flag; isolate by overriding the env var, exactly like the repo's own tests:

```sh
scratch=$(mktemp -d)
XDG_DATA_HOME="$scratch" cli calendars list   # fresh DB, auto-seeded default calendar
```

Opening any database runs pending migrations and creates a default calendar if
none is active. Never run mutating commands against a real user database
unless explicitly asked.

## Timestamps and timezones

- Timestamps are naive wall clock, format `YYYY-MM-DD HH:MM:SS`.
- Event timestamps are interpreted with `--timezone <IANA name>` (e.g.
  `Europe/Rome`); without it they use the system local zone. Planner settings
  require an explicit IANA timezone.

## Command reference

Run `cli <group> <action> --help` to re-verify flags at runtime (clap help is
authoritative if this skill and the binary ever drift).

### calendars — `list | get <id> | create | update <id> | delete <id>`

```sh
cli calendars list
cli calendars get <id>
cli calendars create --name Work --color '#50f872' \
  [--source local|google] [--google-id <id>] [--visible true] [--position 0]
cli calendars update <id> [--name ..] [--color ..] [--source ..] [--google-id ..] [--visible ..] [--position ..]
cli calendars delete <id> [--cascade-events]   # without the flag, delete fails if events exist
```

### projects — same shape as calendars

```sh
cli projects create --name Api --color '#ff0000' [--description ..]
cli projects update <id> [--name ..] [--color ..] [--description ..]
cli projects delete <id> [--detach-events]     # without the flag, delete fails if events reference it
```

### events

```sh
cli events list [--from 'YYYY-MM-DD HH:MM:SS' --to '...']   # from/to are a required pair
cli events get <id>
cli events create --calendar-id <id> --title '..' --start-at '..' --end-at '..' \
  [--project-id ..] [--description ..] [--location ..] [--all-day false] \
  [--rrule 'FREQ=WEEKLY;...'] [--reminder-minutes 30] [--timezone Europe/Rome]
cli events update <id> [same fields as create] \
  [--clear-project-id] [--clear-description] [--clear-location] [--clear-rrule] [--clear-reminder-minutes]
cli events delete <id>
```

### dependencies — DAG links between events

```sh
cli dependencies list | get <id>
cli dependencies create --from-event-id <id> --to-event-id <id> [--dependency-type blocks|related]
cli dependencies update <id> [--from-event-id ..] [--to-event-id ..] [--dependency-type ..]
cli dependencies delete <id>
```

`blocks` is directional (A blocks B); the planner refuses cycles.

### tasks — planner inbox items (NOT events)

```sh
cli tasks list | get <id> | delete <id>
cli tasks create --title '..' --duration-minutes 60 --target-calendar-id <id> \
  [--project-id ..] [--priority low|normal|high] [--cognitive-load low|medium|high] \
  [--earliest-at 'YYYY-MM-DD HH:MM:SS'] [--deadline-kind none|hard|soft] [--deadline-at '..']
cli tasks update <id> [any create field] [--clear-project-id] [--clear-earliest-at] [--clear-deadline-at]
cli tasks return-to-inbox <id>   # deletes only this task's applied event (goes through sync outbox if Google)
cli tasks dependencies list
cli tasks dependencies add --from-task-id <id> --to-task-id <id>
cli tasks dependencies remove --from-task-id <id> --to-task-id <id>
```

### planner — explicit propose-then-apply scheduling

```sh
cli planner settings show
cli planner settings update --timezone Europe/Rome \
  --availability mon=09:00-17:00 --availability tue=09:00-17:00 \
  [--horizon-days 14] [--slot-minutes 30] [--solve-seconds 5] \
  [--priority-low-weight .. --priority-normal-weight .. --priority-high-weight ..] \
  [--cognitive-enabled true] \
  [--low-window-start 13:00 --low-window-end 17:00 --low-outside-penalty ..] \
  [--medium-window-start .. --medium-window-end .. --medium-outside-penalty ..] \
  [--high-window-start 08:00 --high-window-end 12:00 --high-outside-penalty .. \
   --high-streak-limit .. --recovery-minutes .. --excess-high-penalty ..]
cli planner optimize [--horizon-days N]     # creates a proposal only; mutates nothing else
cli planner proposals list | get <id>
cli planner proposals apply <id>            # THE mutation boundary; verifies inbox unchanged since optimize
```

Invariants:

- `--timezone` and at least one `--availability` window are required before the
  first optimize. Repeating a day adds a second window (split shifts).
- All active calendars are busy-time constraints; the target calendar per task
  is where its event gets created.
- Cognitive windows are soft preferences; availability, precedence, and hard
  deadlines always win.
- If any active Google calendar exists, each needs one prior successful
  `google sync` checkpoint or `planner optimize` refuses. No planner command
  starts a sync.
- Applying never auto-syncs.

### google — explicit auth, discovery, sync, conflicts

```sh
cli google auth status
cli google auth login [--client-id ..] [--client-secret ..]   # opens system browser, PKCE S256
cli google auth logout
cli google calendars discover
cli google calendars import --google-id <calendar-id> [--position 0]
cli google sync [--calendar-id <id>]        # explicit; never automatic
cli google sync-status [--calendar-id <id>]
cli google conflicts list
cli google conflicts resolve <conflict-id> --strategy keep-local|keep-remote
```

Behavior: refresh token in the OS keyring (service `planner123`; a legacy
`solverforge-calendar` entry is migrated into it on first read and removed on
logout);
read-only imported calendars reject local edits; two-way sync covers single
events, all-day events, title, description, location, start/end, and master
RRULE; detached recurrence exceptions, attendees editing, attachments, and
conference data are out of scope.

### ical

```sh
cli ical import --calendar-id <id> --path ./sample.ics [--timezone Europe/Rome]
```

Returns structured counts plus warnings; unsupported ICS shapes are skipped
with warnings, never silently.

## Standard recipes

```sh
# End-to-end in a scratch database
scratch=$(mktemp -d) && export XDG_DATA_HOME="$scratch"
cal=$(cli calendars create --name Work --color '#50f872' | jq -r .data.id)
cli events create --calendar-id "$cal" --title 'Sync' \
  --start-at '2026-09-15 10:00:00' --end-at '2026-09-15 11:00:00'
cli events list --from '2026-09-15 00:00:00' --to '2026-09-15 23:59:59'

# Plan a task
cli planner settings update --timezone UTC --availability mon=09:00-17:00
task=$(cli tasks create --title 'Review' --duration-minutes 60 \
  --target-calendar-id "$cal" --priority high --cognitive-load high | jq -r .data.id)
cli planner optimize | jq -r '.data.proposal.id'   # proposal nested under .data.proposal
prop=$(cli planner proposals list | jq -r '.data[0].id')   # list returns bare proposal objects
cli planner proposals get "$prop"   # review before applying; nests under .data.proposal again
cli planner proposals apply "$prop"
```

## Maintaining the repository

```sh
make build        # both binaries
make lint         # cargo fmt --check + clippy -D warnings
make test         # full test suite
make ci-local     # fmt + clippy + build + test (matches GitHub Actions)
make pre-release  # release-gate validation before tagging
```

Rules that keep changes mergeable:

- Every tracked Rust source and test file stays below 300 lines; large
  subsystems keep a stable root facade module plus a directory of focused
  fragments.
- Touching the CLI contract requires updating `README.md`, `tests/cli.rs`, and
  `docs/wireframes/cli.md` in the same change.
- Route event/calendar mutations through `event_service`/`calendar_service`,
  never UI-local or CLI-local rules.
- Tests stay deterministic: no live Google API, no real keyring. CLI tests
  isolate with `XDG_DATA_HOME` temp dirs; Google fakes use
  `PLANNER123_TEST_GOOGLE_SYNC` / `..._GOOGLE_DISCOVERY` /
  `PLANNER123_TEST_KEYRING_SERVICE`.
- Releases are cut with commit-and-tag-version from conventional commits; never
  hand-edit `CHANGELOG.md` or version files. See `AGENT.md` and `PRD.md`.

## Troubleshooting

- `validation_error` with timestamp text → use `YYYY-MM-DD HH:MM:SS`.
- Delete refused → dependent events exist; add `--cascade-events`
  (calendars) or `--detach-events` (projects) only when confirmed.
- `planner optimize` refuses → run `google sync` until `sync-status` is
  healthy for every active Google calendar, or check `planner settings show`
  for missing timezone/availability.
- `proposals apply` refuses → inbox tasks changed since optimize; re-run
  `planner optimize` and review the fresh proposal.
- Command not found / flag drift → `cli <group> <action> --help` is the
  live truth generated from the same clap definitions the binary uses.
