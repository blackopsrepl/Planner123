# CLI Wireframe

## Command shape

```text
solverforge-calendar-cli <group> <action> [flags]
```

Supported groups:

- `calendars`
- `projects`
- `events`
- `dependencies`
- `google auth`
- `google calendars`
- `google sync`
- `google sync-status`
- `google conflicts`
- `ical import`
- `tasks`
- `planner settings`
- `planner optimize`
- `planner proposals`

## Success response

```json
{
  "status": "ok",
  "data": { "...": "..." }
}
```

## Error response

```json
{
  "status": "error",
  "code": "validation_error",
  "message": "human-readable explanation"
}
```

## Behavioral notes

- Parsing is strict: unknown flags and malformed values fail fast.
- There are no prompts or interactive confirmations.
- Destructive behavior stays explicit through flags or direct commands.
- `google sync` is explicit and non-interactive.
- `google sync-status` and `google conflicts` expose durable sync health for automation.
- `ical import` returns structured counts plus warnings for skipped or unsupported ICS shapes.
- `tasks create` accepts required `--title`, `--duration-minutes`, and `--target-calendar-id`, plus `--priority` and `--cognitive-load` (`low|medium|high`).
- `planner settings update` requires a timezone and repeatable typed weekly `--availability DAY=HH:MM-HH:MM` values before the first optimization; its cognitive fields define one global daily window for each load level plus high-load recovery settings.
- `planner optimize` only creates a proposal. `planner proposals apply <id>` is the explicit mutation boundary and checks that inbox tasks have not changed.
- Each proposal item reports a typed `outcome` (`scheduled`, `no_hard_feasible_slot`, or `feasible_but_not_selected`; legacy rows migrated from older databases may still read `unassigned`), an optional `explanation`, and, when a task is hard-infeasible, bounded `busy_blockers` evidence naming the calendar occurrences (including recurring ones) that removed its candidate slots, with an exact `busy_blockers_omitted` count. Per-task soft penalties come from the scoring model; tasks that cannot be placed are left unassigned rather than forced into an invalid slot.
- Active Google calendars must have a successful explicit sync checkpoint before `planner optimize`; no planner command starts a Google sync.
