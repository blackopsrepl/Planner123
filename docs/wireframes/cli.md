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
