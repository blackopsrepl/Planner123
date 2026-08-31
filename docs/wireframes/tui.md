# TUI Wireframe

```text
┌──────────────── SolverForge Calendar ────────────────┬──────────── Sidebar ────────────┐
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

## Overlay surfaces

- Google management: connection state, imported calendar health, pending outbound work, conflicts, and discoverable calendars for import
- Google auth: desktop-client credential entry plus browser-based connect flow
- Event form: selected calendar plus local / writable Google / read-only Google sync implications
- `.ics` import: file path plus target calendar, imported in the background

## Interaction notes

- `G` opens Google management or the auth surface if no account is configured.
- `S` triggers sync immediately; there is no automatic startup sync.
- Read-only Google calendars stay visible but reject local edits.
- Google conflicts are surfaced in the status bar and Google management view.
- `.ics` import is explicit through `i` and targets the selected calendar.
# Planner Inbox

`p` opens a dedicated planner surface; it does not alter event quick-add or event forms. `n` opens a task form with title, duration, target calendar, priority, and cognitive load. `s` opens Planner Settings for timezone, weekly availability, horizon, slot size, and solve time. `o` runs the batch optimizer and displays a proposal with timing and fatigue explanations. `a` applies the reviewed proposal explicitly.

The TUI and JSON CLI share one persisted planner setting source. Weekly availability uses visual TUI editing or repeatable typed CLI values such as `--availability mon=09:00-17:00`.
