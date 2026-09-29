# Keymap consolidation — Planner123 ≤ v0.6.0 → target

Status: **phases 1–4 landed.** The keymap is consolidated; what remains open is
listed in §0b.
Scope: the interactive surface only (TUI keymap, status bar, help overlay, modal
behaviour). The JSON CLI contract is untouched.

---

## 0a. What shipped

Every commit green: `make lint && make test`.

| Commit | What it did |
| --- | --- |
| `refactor(keys)` | `src/keys/registry` is the one table; `resolve` is a lookup over it; the status bar chips and the help overlay are derived from it; `src/keys/{navigation,forms,google,hints}.rs` are gone |
| `fix(tui)` | the status bar has a width budget: the status block is capped at a third of the row, chips drop from the tail of a priority order, the help chip is reserved before anything else, quick add earns a chip |
| `refactor(tui)` | the help overlay is generated per surface, opens on the surface you are in, and resets its scroll |
| `test(tui)` | the help fit test derives the box height from `centered_rect` instead of hardcoding it |
| `fix(tui)` | overlays return to the view they were opened from, not to the month grid |
| `feat(tui)` | `:` opens a command palette (19 commands) over the actions that do not earn a key, and `g`'s dead binding becomes a real go-to-date prompt (`YYYY-MM-DD` or `+21` / `-7`) |
| `refactor(keys)` | `g`, `i`, `x` and the duplicate bindings leave the flat namespace: `i` meant `.ics` import in one place and Google import in another, `s` meant sync in one place and planner settings in another |
| `refactor(keys)` | one way to change a select value: `←`/`→` (aliased to `h`/`l`), no more `+`/`-`, no more "type any key to cycle" |
| `feat(keys)` | `A` applies a planner proposal; the one bulk write to the calendar gets a deliberate keystroke |
| `feat(tui)` | `d` and export arm first: the bar shows the question, `y` commits, any other key cancels, and nothing else is bound while it is up |
| `fix(tui)` | the status message expires after six seconds instead of owning the right-hand block for the session |
| `fix(tui)` | the project progress bars are gone: they read a completion set nothing ever wrote |

Measured effect at 80 columns, month view:

```text
 before |  h/l  day   H/L  month   j/k  row   n  today   c  crea  google off 14:22
 after  |  c create  e edit  d del  / quick  ? help      |  google off  14:22
```

End state: **167 registry rows, 195 key aliases, 79 bar chips, 79 help rows in
14 sections**, all from one table. Per-surface keys: month 26 → 24, calendar
sidebar 12 → 9, Google management 11 → 9. Three letters retired outright (`g`,
`i`, `x`), `a` shifted to `A`, and `:` added; the long tail lives in the palette.

Two deliberate deviations from the proposal in §3–§7:

- **The palette has 19 commands, not the ten sketched.** Views, today, new
  event and quick add are in it too: a command palette that hides half the app
  is a worse index than a slightly longer one.
- **The sidebar Projects panel was not cut.** The bars and the `0/N` fraction
  were the lie — the state behind them (`completed_event_ids`) had no writer —
  so the panel now shows the project and its event count in the loaded window,
  and the field and the progress-bar helper are gone.

## 0b. Still open

- On an 80 column terminal the palette chip and the view switcher are the first
  chips to drop (the verbs and `?` stay). The palette is documented in the
  global section of help, but it is not on the bar at that width.
- Help scrolls one section at a time, current surface first; it has no
  type-to-filter of its own. The palette covers the action long tail, not the
  help text.
- The right-hand status zone still holds one message; there is no queue, so a
  worker result and a validation error overwrite each other (both expire now).
- The `p` planner key is lowercase while the other surface switches (`Tab`,
  digits) are structural: §4.3 documents it as an exception rather than fixing
  it.
- The planner task form's timing fields (earliest start, deadline) remain
  CLI-only; the TUI form collects title, duration, calendar, priority and load.

## 0. The decision in one paragraph

The hey.com iteration was the right instinct — verbs on the focused object, a
triage mode with a review-before-commit gate — but it was bolted onto a flat
global letter namespace that already had a calendar's worth of keys in it. We
ended up with **155 key→action bindings across 11 contexts**, a status bar that
renders **138 columns of hints into one row**, and a help overlay that needs
**90 rows** to say what the app does. The consolidation is not "delete some
keys": it is **one registry, two tiers, and a palette for the long tail**.
Keep ~34 keys that live in muscle memory, move ~8 rare actions to a `:`
palette, and generate the status bar and the help overlay from the same table
the resolver reads, so they can never drift again.

---

## 1. Evidence (measured from the tree, not from memory)

### 1.1 Inventory

| Context | key→action rules | individual bindings | status-bar hints | help rows |
| --- | ---: | ---: | ---: | ---: |
| Month | 26 | 30 | 11 | 9 |
| Week/Day (shared resolver) | 25 | 29 | 10 | 7 |
| Agenda | 23 | 25 | 8 | 5 |
| Calendar sidebar | 11 | 14 | 4 | 3 |
| Event form | 5 | 7 | 3 | 6 |
| Quick add | 3 | 3 | 2 | 3 |
| `.ics` import | 5 | 7 | 3 | 4 |
| Help | 5 | 9 | 2 | 3 |
| Google auth | 5 | 8 | 3 | — (inlined into Google management) |
| Google management | 8 | 13 | 7 | 7 |
| Planner inbox | 7 | 10 | 6 | 8 |
| **total** | **123** | **155** | **65 hint entries** | **66 bindings + 11 headings = ~90 rows** |

Source: `src/keys.rs`, `src/keys/{navigation,forms,google,hints}.rs`,
`src/ui/help.rs`.

### 1.2 The status bar cannot hold the hints it ships

`render_status_bar` (`src/ui/status_bar.rs:64-157`) draws the hint paragraph
into the full-width area with no truncation and then paints the right-hand
status block **on top of the last N columns**. Measured hint width per view
(`len(key)+len(desc)+6` per hint):

| View | hint chars | fits 80? | at 80 cols visible | at 80 cols invisible |
| --- | ---: | --- | --- | --- |
| Month | **138** | no | `h/l`, `H/L`, `j/k`, `n` | `c`\*, `e`, `d`, `1-4`, `Tab`, `p`, `?` |
| Week/Day | **125** | no | `h/l`, `j/k`, `n`, `c` | `e`\*, `d`, `1-4`, `Tab`, `p`, `?` |
| Agenda | 99 | no | `j/k`, `n`, `c`, `e` | `d`\*, `1-4`, `p`, `?` |
| Google management | 89 | no | `j/k`, `i`, `r`, `l` | `o`\*, `s`, `Esc` |
| Planner inbox | 92 | no | `j/k`, `n`, `o` | `a`\*, `s`, `Esc` |

\* = the hint is cut mid-word by the right-hand status block.

Consequences, in the plainest terms:

- **At 80×24 the app hides its own verbs.** You see history navigation and
  "today"; you do not see create, edit, delete, the view switcher, the sidebar,
  the planner, or help.
- At 100 cols help and the planner are still invisible; only at ≥160 cols does
  the whole bar fit.
- The right-hand zone is not reserved. A single error message displaces the
  entire hint line: `"Correct the planner timezone: use an IANA name such as
  Europe/Rome or UTC."` is 76 columns, i.e. the whole bar on a small terminal.
- `status_message` is **never expired** (`src/app/query.rs:44`). One export or
  one validation error owns the right-hand zone for the rest of the session.
  The bar is doing three jobs — verbs, health, transient messages — with no
  priority policy.

### 1.3 The help overlay is a scroll, not a lookup

`build_help_lines()` emits ~90 rendered rows into `centered_rect(70, 85, …)`:
~40 usable rows on a 50-row terminal, ~22 on a 24-row terminal. The published
screenshot (`assets/screenshot-help.png`) shows exactly this — GLOBAL, MONTH,
WEEK/DAY and one row of AGENDA, and the remaining eight sections are below the
fold. Reading the cheat sheet requires the cheat sheet's own keys.
`help_scroll` is also never reset when the overlay is reopened
(`src/app/state.rs:224` — set once at startup).

### 1.4 Three hand-maintained sources of truth, zero tests

`resolve()` decides what a key does; `hints()` decides what the bar says;
`build_help_lines()` decides what help says. Nothing links them and **no test
in `tests/` or `src/` touches any of the three**. Every divergence below is
therefore invisible to CI:

| # | Divergence | Where |
| --- | --- | --- |
| 1 | `g` (jump to date) is bound in month view and dispatches to a no-op | `src/keys/navigation.rs:20` → `src/app/dispatch.rs:174` |
| 2 | Day view hints say `h/l week`; `h/l` moves one **day** | `src/keys/hints.rs:23-33` |
| 3 | Help claims Agenda `Enter` = "detail"; `Enter` is dead outside month view | `src/ui/help.rs:97` vs `src/app/integrations.rs:206-211` |
| 4 | Sidebar's `c`, `G`, `S`, `i`, `x` bindings appear in neither hints nor help | `src/keys/forms.rs:11-15` |
| 5 | Planner task form: "type any key to cycle" select fields — documented nowhere | `src/app/planner.rs:21-33`, `src/ui/planner.rs:206-214` |
| 6 | Event-form selects accept `+`/`-` as well as `h`/`l`; only `h`/`l` documented | `src/app/event_form.rs:129-147` |
| 7 | `i` = `.ics` import globally, `i` = Google-calendar import inside Google management | `src/keys/navigation.rs:28` vs `src/keys/google.rs:23` |
| 8 | `s` = planner settings (inbox) / Google sync (Google management) / nothing (elsewhere) | `src/keys/google.rs:26,38` |
| 9 | `n` = today globally, `n` = new task in the planner inbox | `src/keys/navigation.rs:19` vs `src/keys/google.rs:35` |
| 10 | `q` = quit at top level, close inside overlays; `Esc` in month view is a no-op | `src/keys/navigation.rs:6`, `src/app/navigation.rs:188-208` |
| 11 | Every overlay returns to **Month**, so `c` from Week view costs you your context | `src/app/navigation.rs:195-202` |
| 12 | `x` exports every event (including hidden calendars) to a fixed `~/planner123.ics` with no confirm | `src/app/integrations.rs:189-196` |
| 13 | `d` deletes immediately — no confirmation, no undo | `src/app/integrations.rs:200-204` |
| 14 | Sidebar Projects progress bars can never move: `completed_event_ids` is initialised empty and never written | `src/app/state.rs:101,199`, `src/ui/calendar_list.rs:131-138` |

### 1.5 Also found while measuring

- `src/ui/mod.rs` and `src/ui/status_bar.rs` carried the same header comment
  repeated 11× and 2× (a tooling artifact). Both are fixed; `ui/mod.rs` now
  sketches the layout it actually renders.
- The TUI has no dependency/DAG surface at all (dependencies are CLI-only),
  yet day view and the sidebar read DAG/completion state that can never change
  in the TUI.

---

## 2. What the hey.com iteration actually gave us — **keep**

Judged on "does this make the calendar faster to drive", not on novelty:

1. **Verbs on the focused object.** `c`/`e`/`d`/`Enter` acting on the selected
   day or event, with `/` as the capture-first verb. This is the whole product
   in five keys. Keep, unchanged.
2. **A triage mode with its own small verb set.** `p` → inbox with
   `n`/`o`/`s`/`a`, inspecting one task at a time with `j`/`k`. Five keys, one
   screen, one mental model. Keep — this is the good part of the hey.com
   inheritance, and it is already correctly scoped.
3. **Review-before-commit.** The proposal exists in a `ready` state; nothing
   touches the calendar until `a`. This is the best UX decision in the
   codebase. Keep, and let it set the house rule for destructive verbs (§4.5).
4. **Explicit, non-implicit sync.** `S` only, no startup sync, no background
   surprises. Keep.
5. **Overlay surfaces for rare, stateful tasks** (Google management, `.ics`
   import, forms) rather than keys that mutate a hidden selection. Keep the
   pattern; trim the key surface.
6. **Numeric view switching** `1`–`4`. Cheap, teachable, four keys for the
   entire spatial model. Keep.

## 3. What we do **not** keep

| Cut | Why | Where it goes |
| --- | --- | --- |
| `g` jump-to-date | bound and wired to nothing | palette, implemented |
| `x` export `.ics` | fixed-path bulk dump of all events, no confirm, silently makes `x` a footgun next to `c`/`d` | palette, with a confirm + path prompt |
| `i` import `.ics` | monthly-cadence action occupying a global letter; also overloaded inside Google management | palette |
| `i` + `s` inside Google management | duplicates of a global action with a different meaning (`s` = sync, `i` = import) — the exact overload pattern we're killing | keep `Enter` = import, delete the rest |
| `G`/`S`/`i`/`x` duplicated in the sidebar resolver | four redundant rows that no surface documents | global keys only, defined once |
| "type any key to cycle" (planner task form selects) | makes every letter key ambiguous inside a form and is undiscoverable by construction | `←`/`→` |
| `+`/`-` in event-form selects | a third mechanism for the same job | `←`/`→` (with `h`/`l` as the documented Vim alias) |
| `h`/`l` as *documented-but-wrong* labels in Day view | help says week, key moves a day | label generated per view |
| Agenda `Enter` claim | asserted in help, dispatches to nothing | implement (open the event's day), or delete the claim — implementing wins, "Enter opens the thing" is worth protecting |
| Esc-dumps-to-Month | a modal that loses your context is a modal you learn to avoid | restore the previous view (§4.4) |
| Sidebar Projects progress bars | can never move (`completed_event_ids` is never written) | cut the panel; project badges in day view and the CLI stay. Corollary: the DAG stays an agent/CLI feature, not a TUI workflow |

## 4. The consolidated model

### 4.1 One registry, three renderings

A single table is the only source of truth:

```rust
struct Binding {
    context: Context,      // Global, Month, Week, Day, Agenda, Sidebar, EventForm, Inbox, Help, …
    key: KeySpec,          // Char('c') | Ctrl('k') | Tab | Enter | …
    action: Action,
    label: &'static str,   // "create"
    tier: Tier,            // Primary | Secondary | Palette
    group: &'static str,   // help section title
}
```

`resolve()`, `hints()` and the help overlay are all derived from it:

- `resolve(view, key)` = registry lookup by `(context, key)`, nothing else.
- `hints(view)` = rows with `tier == Primary`, in table order, then width-budgeted.
- help = rows grouped by `group`, current context expanded first.

Drift stops being a code-review problem and becomes a test problem (§9).

### 4.2 Two tiers + a width budget (the status bar fix)

Three changes, in order of value:

1. **Reserve the right zone.** Compute `hint_budget = area.width - right_width - 1`
   and never let a hint enter it. If the status message is long, truncate the
   *message*, not the hints.
2. **Drop from the tail, never clip mid-hint.** Build the line, sum widths, and
   pop trailing hints until it fits the budget. A bar that shows six honest
   hints beats one that shows four and cuts two in half.
3. **Pin `?` — reverse-anchor it.** The last chip before the right zone is
   always `? keys` (or `: cmd` + `? keys`). Help must never be off-screen; it
   is the recovery path for every other hint.
4. **`Primary` means "verbs for the focused object", not "everything you can
   press".** Navigation (`h`/`j`/`k`/`l`, `H`/`L`, `PgUp`/`PgDn`, arrows) drops
   to `Secondary`: it is discoverable by trying it and by the help overlay, and
   it is the part that eats the budget.

Result at 80×24, month view:

```
 old  |  h/l  day   H/L  month   j/k  row   n  today   c  crea  google off  14:22
 new  |  c create  e edit  / quick  1-4 view          ? keys  |  google synced 14:22
```

and at 120×24 the new bar adds `n today`, `Tab lists` and `p planner` — with
`?` still pinned. Note what each bar teaches: the old one teaches four
navigation keys and hides every verb; the new one teaches the four things you
came to the app to do.

### 4.3 Key naming rules (so the next feature can't re-create the problem)

1. **Uppercase = global, lowercase = local to the focused object.**
   `S` sync, `G` Google, `p` planner (surface), `n` today (navigation) are the
   named exceptions, and there are only these three.
2. **A mode may override a lowercase key only if it is a verb on that mode's own
   object, and only if the status bar shows the override.** (Inbox `n` = new
   task qualifies; `s`=sync inside Google management does not, and dies.)
3. **Digits and `Tab` are structural** (views, focus) and are never reused.
4. **`/` `?` `:` `Esc` are reserved punctuation** (capture, help, palette, back).
5. **New feature ⇒ palette, unless it is a verb you use daily.** Adding a letter
   to the flat namespace requires deleting one.

### 4.4 Modal discipline

- Every mode and overlay stores `previous_view` and `Esc`/`q` restores it.
  `Esc` from a form opened in Week view returns to Week view.
- `Esc` in a top-level view is a no-op (as today) — never a hidden "go home".
- `q` = close the current surface; at top level, quit the app. One rule, both
  behaviours.
- Forms own their letters: inside any text field, every printable key types.

### 4.5 Destructive verbs

Deletion (`d`) and bulk export (`x`) route through the same gate the planner
already uses: **nothing irreversible without an explicit second confirmation.**
Cheapest honest implementation: an inline status-bar confirm (`Delete
"Standup"? y/N`) for `d`; a path + confirm step for export. Not a modal, not a
dialog fleet — one line, one keystroke, no silent data loss.

## 5. Target keymap

**Global (live in every view, defined once) — 17 rows**

| Key | Action | Tier |
| --- | --- | --- |
| `1` `2` `3` `4` | Month / Week / Day / Agenda | Primary |
| `Tab` | focus calendar list | Primary |
| `n` | today / now | Primary |
| `/` | quick-add event | Primary |
| `c` / `e` / `d` | create / edit / delete focused event | Primary |
| `p` | planner inbox | Primary |
| `S` | sync now | Primary |
| `G` | Google account & calendars | Secondary |
| `?` | help (searchable) | Primary (pinned) |
| `:` | command palette | Primary (pinned) |
| `q` / `Ctrl+C` | close surface / quit | Secondary |
| `Esc` | back, cancel, restore previous view | Secondary |

**Month — 4 rows**: `h`/`l` ±1 day, `j`/`k` ±1 week row, `H`/`L` ±1 month,
`Enter` open day view.

**Week / Day — 3 rows**: `h`/`l` ±1 period, `j`/`k` next/previous event,
`PgUp`/`PgDn` scroll the grid.

**Agenda — 2 rows**: `j`/`k` scroll, `Enter` open the event's day (implemented).

**Calendar list — 3 rows**: `j`/`k` calendar, `Space` show/hide, `Tab`/`Esc` back.

**Event / `.ics` / settings forms — 5 rows**: `Tab` / `Shift+Tab` field,
`←`/`→` cycle a select value (`h`/`l` alias), `Space` toggle, `Enter` save,
`Esc` cancel.

**Planner inbox — 6 rows**: `j`/`k` select task, `n` new task, `o` optimize,
`s` settings, `A` apply (shifted: it is the one irreversible write),
`Esc` back.

**Help — 2 rows**: type to filter, `Esc` close.

Totals: **42 rows in the registry**, of which **~23 are Primary** and the rest
are navigation/back that belong in help — from 155 bindings and 65 hint
entries. Navigation moves out of the status bar and into a help overlay you can
actually read, and seven rare actions move to `:`.

## 6. Help: from wall to lookup

1. **Context first.** Open with the current view's bindings fully expanded —
   that is ≤10 rows, no scrolling at 80×24. Other contexts are collapsed
   headers you can expand.
2. **Filter.** Any printable key filters rows by action or key ("del", "sync",
   "S"). This is what turns 66 rows into a lookup instead of a scroll.
3. **Two columns above ~100 cols**, one below. Generated from the registry.
4. **Reset the scroll** on open, and cap the overlay at the content height so
   an empty box never looks like a broken one.

## 7. The palette (`:`)

The long tail, fuzzy-filtered, one row per action with its current key shown if
it has one — this is also the discovery surface for keys you never learned:

| Palette entry | Was |
| --- | --- |
| Jump to date… | `g` (dead) |
| Import `.ics`… | `i` |
| Export `.ics`… | `x` |
| Google management | `G` (kept global, also here for discoverability) |
| Discover Google calendars | (Google-management-local) |
| Log in / log out of Google | (Google-management-local) |
| Planner settings | `s` (inbox-local, also here) |
| Return task to inbox | (currently CLI-only) |
| Sync now | `S` |
| Toggle calendar visibility | `Space` (contextual) |

Cost: ~120 lines of a filtered `List` over the existing `Action` enum. Benefit:
seven flat letters deleted from a namespace where every entry costs a person's
muscle memory.

## 8. Migration phases

**Phase 1 — registry + budgets, no key moves (no user-visible rebinding).**
Port the existing bindings into the registry verbatim; generate `hints()` and
help from it; reserve the right zone; drop hints from the tail; pin `?`; reset
`help_scroll`; make help context-first. Ship the five tests in §9. Everything
in §1.2 and §1.3 disappears and nothing rebinds. This phase alone answers the
complaint that started this document.

**Phase 2 — key moves.** Delete `g`, `i`, `x`, the Google-management `s`/`i`
duplicates and the sidebar duplicates; add `:` palette; `←`/`→` in forms; `A`
apply in the inbox; fix the Day-view label, the Agenda `Enter` claim and
`previous_view` restore. Update README/wireframes/PRD keybinding tables in the
same commit — PRD already has two open items about stale keybinding docs.

**Phase 3 — destructive-verb gate.** Confirm for `d`, path + confirm for `x`:
the same pattern the planner already uses.

**Phase 4 (decide, not automatic) — Projects panel.** Either wire
`completed_event_ids` for real (a `Space` = done verb, persisted, DAG-aware) or
cut the sidebar panel. As it stands it is decoration that renders `0/N` forever.

## 9. The tests that lock it (none exist today)

1. **No collision**: for every context, `(context, key)` appears at most once.
2. **Completeness**: every `Action` variant is either registry-bound or in an
   explicit `INTERNAL` list — new actions cannot ship unreachable.
3. **Bar budget**: for each view at 80/100/120/160 cols, the rendered hint line
   is ≤ budget after tail-dropping, ends with the pinned `?`, and never overlaps
   the reserved right zone.
4. **Help fits**: for each view at 80×24, the current context's rows render
   without scrolling.
5. **Truthfulness**: every `Primary` row's help row exists and its label matches
   the registry (this is what would have caught divergences #1–#6 above).

---

## 10. Calls I am explicitly making (override me)

1. **`p` stays lowercase while `S`/`G` are uppercase.** `p` is a surface switch,
   not a mutation; grouping it with `Tab`/`1-4` is more honest than forcing the
   case rule to be total.
2. **`S` survives as a global key.** Sync is the one integration action with a
   daily cadence for Google users and it has a visible health consequence.
   `G` demotes to Secondary (discoverable via palette + the header badge).
3. **`A` (shifted) applies a proposal.** It is the only action that writes to a
   user's calendar in bulk; it deserves a deliberate keystroke.
4. **The Projects panel goes to Phase 4, not Phase 2.** Cutting UI in a keymap
   consolidation invites the argument "you changed what I see". It is a
   separate, explicit decision — but it should be made, not deferred forever.
