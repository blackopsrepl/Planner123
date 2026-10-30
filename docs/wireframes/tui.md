# TUI Wireframe

```text
┌──────────────── Planner123 ────────────────┬──────────── Sidebar ────────────┐
│ Month / Week / Day / Agenda                          │ Calendars                        │
│ Date range + Google health summary                   │  [x] Personal                    │
├──────────────────────────────────────────────────────┤  [x] Work G                      │
│                                                      │  [x] Shared G lock               │
│ Main calendar surface                                ├──────────────────────────────────┤
│                                                      │ Projects                         │
│ - Month: 5-week grid                                 │  Launch                          │
│ - Week: hourly time grid                             │  Planning                        │
│ - Day: single-day schedule                           ├──────────────────────────────────┤
│ - Agenda: sorted upcoming events                     │ Selected event summary           │
│                                                      │  Title                           │
│                                                      │  Time                            │
│                                                      │  Project / dependency hints      │
├──────────────────────────────────────────────────────┴──────────────────────────────────┤
│ Status bar: key hints, transient errors, Google auth/sync health                      │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

## Week and day grid

Two text rows per hour, so each row is 30 minutes. An event starts on the row
its start time falls in and covers every row its interval touches, at least
one. Events whose rows meet are drawn side by side in lanes, so every timed
event is visible and no two blocks share a cell; an overnight event is clipped
to each day it touches. All-day events sit in an `all` strip above the hours.

```text
       Mon 26        Tue 27
all                  ▌Holiday
09:00 ▌Long ────────────────────
      ▌09:00        ▌Dentist        ← 09:30-10:00, on the half-hour row
10:00 ▌     ▌Inner ─▌Prep demo      ← 10:00, right below it
      ▌             ▌10:00
```

`Inner` (10:00-10:30) overlaps `Long` (09:00-11:00), so it gets its own lane.

## Effects and proposal preview

The calendar/sidebar shell stays in place. Background tints sit evenly under
every plain cell, glyphs included, and never touch selections or event fills.
Agenda stars stay two cells clear of any text; edge fades dim them at the edges. Calendar
entry wipes settle on rapid navigation and cancel on resize. The status chip
flash follows the registry action, including paired `j/k` bindings.

```text
┌ Calendars ┐┌ Planner Inbox ─────────────────────────────────────┐
│ [x] Work  ││ > Write report     60m                             │
│ Projects  │├ Selected Task ─────────────────────────────────────┤
│           ││ Calendar, priority, constraints, task ID           │
│           │├ Proposal preview · dashed = not applied · A applies┤
│           ││ GHOST / ready · 2/2 scheduled · Europe/Rome         │
│           ││       Mon 02 Nov      Tue 03 Nov                    │
│           ││ 09:00 ┊Write report  ▌Existing meeting             │
│           ││ 09:30 ┊              ┊Review                        │
└───────────┘└────────────────────────────────────────────────────┘
```

Dates, labels and placements come from the actual proposal, not this example.
Existing recurring occurrences and proposed slots share the working calendar's
overlap-lane allocator. Offscreen targets do not launch flights. Selection
anchors the preview; after successful apply and matching loaded IDs the rails
are solid, with bounded sweeps/ripples. No fake queue replaces the sidebar.

Palette `Effects: all [ON/OFF]` and `Effect: <name> [ON/OFF]` persist master,
native-layer and all 37 glyphfx switches. `Credits and glyphfx gallery` opens a
dedicated surface with a terminal-cell sine scroller, animated braille logo
and automatic gallery playback; left/right browse and Escape closes. This is
not the help screen and never opens on startup. Master-off cancels motion but
retains the complete normal UI, including the proposal preview.

## Overlay surfaces

- Google management: connection state, imported calendar health, pending outbound work, conflicts, and discoverable calendars for import
- Google auth: desktop-client credential entry plus browser-based connect flow
- Event form: selected calendar plus local / writable Google / read-only Google sync implications
- `.ics` import: file path plus target calendar, imported in the background

## Interaction notes

- `:` opens the command palette: the actions that do not earn a key of their own
  (`.ics` import and export, Google connection and discovery, planner settings,
  go to date), filtered by typing.
- `G` opens Google management or the auth surface if no account is configured.
- `S` triggers sync immediately; there is no automatic startup sync.
- The status bar shows the verbs for the current surface and drops trailing
  chips on a narrow terminal, so `? help` is always visible.
- Read-only Google calendars stay visible but reject local edits.
- Google conflicts are surfaced in the status bar and Google management view.
- `.ics` import is explicit (palette: "Import .ics file") and targets the selected calendar.
# Planner Inbox

`p` opens a dedicated planner surface; it does not alter event quick-add or event forms. The task list contains only unscheduled inbox work: tasks leave it when their proposal is applied, while their durable lifecycle record remains available through the JSON CLI. Use `j`/`k` to inspect the selected task's calendar, priority, cognitive load, timing constraints, and identifier. `n` opens a task form with title, duration, target calendar, priority, and cognitive load. `s` opens Planner Settings for timezone, weekly availability, horizon, slot size, and solve time. The timezone field explicitly requires an IANA name such as `Europe/Rome` or `UTC`, shows the detected system timezone, and validates before save. Availability explicitly uses comma-separated `day=HH:MM-HH:MM` entries, for `mon` through `sun` or full weekday names, with a working-week example and split-shift support. `o` opens settings instead of running if the persisted timezone or weekly availability is absent or invalid; otherwise it runs the batch optimizer and displays a proposal with timing and fatigue explanations. `A` applies the reviewed proposal explicitly; it is shifted on purpose, because it is the one action that writes events in bulk.

The TUI and JSON CLI share one persisted planner setting source. Weekly availability uses visual TUI editing or repeatable typed CLI values such as `--availability mon=09:00-17:00`.
