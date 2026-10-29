# Planner123 effects plan

## Scope and evidence

Source audit of `solverforge-demoscene/planner123_06`, the current Planner123 TUI, the glyphfx C engine, and the existing glyphfx bridge in `hammer_05`. This is an implementation plan, not a claim that effects have been integrated, visually verified in a real terminal, or benchmarked.

All shipped effects are enabled by default. Users can persistently toggle all effects or individual effects. Disabling effects immediately restores the normal, complete UI; it never changes calendar data or planner behavior.

## What the demo actually does

Planner 06 does not use glyphfx. It draws ratatui buffers with custom Rust effects, then rasterizes them into a pixel window. Its grid, background colors, stars, reveals, task flights and ripples are terminal-portable. Its pixel-space double-size sine scroller, vector ouroboros, bloom, vignette and scanlines are not literal standard-terminal features. Terminal-cell or braille approximations are possible, not pixel-identical copies.

Sources (relative to solverforge-demoscene):

- `planner123_06/src/scenes/mod.rs:60`: launch reveal and panel wipe.
- `planner123_06/src/tui.rs:133`: pressed-key chip feedback.
- `planner123_06/src/views/week.rs:29`: copper bars, ghost and landing styles.
- `planner123_06/src/views/month.rs:38`: plasma and banner.
- `planner123_06/src/views/agenda.rs:87`: stars, scrolling list, edge fades and pixel-space greetings.
- `planner123_06/src/scenes/flights.rs:41`: scripted proposal, task flights, landing ripples and apply sweep.
- `planner123_06/src/scenes/close.rs:97`: pixel-space logo.

The demo's dates, solver outcomes, task landings and moving now line are choreography. Production effects must follow real data and completed operations; never animate invented progress or change the position of the actual current time.

## Recommended product effects

| Effect | Production behavior | Scope |
|---|---|---|
| Agenda starfield | Low-density parallax stars in empty agenda cells; protect text, selection, borders and popups. Use dynamic viewport dimensions. | First slice |
| Agenda movement | Short eased response to user scrolling; preserve the requested logical row and selected event. No automatic movement of the working agenda. Edge fades only in decorative margins, never make an event unreadable. | First slice |
| Pressed-key chips | Brief theme-colored flash after an action resolves, using the key registry's actual action/hint mapping. | First slice |
| Reveals and view wipes | Brief region-local reveal on entry or new data; fast navigation interrupts and settles immediately. Do not replay on every render or ordinary refresh. | Second slice |
| Proposal/apply feedback | Reveal a completed proposal; after successful apply and matching event reload, highlight the real newly created events with a sweep and bounded landing ripples. | Second slice |
| Copper and plasma | Theme-derived, low-amplitude backgrounds behind week/day and month content. Text and event fills win. Pause hidden layers. | Third slice |
| Task flights | Fly task labels to real proposed slots in a dedicated proposal-preview surface; retain dashed/ghost styling until apply succeeds. Skip offscreen targets. Never replace the sidebar with a fake queue. | Later slice |
| Credits flourish | Dedicated palette-accessible credits surface with cell-based sine text and a braille/block logo approximation. Normal help remains help. | Later slice |

Do not ship full-screen white flashes, a false moving now line, a banner over calendar content, startup waits, or automatic agenda scrolling as normal workflow effects. These are presentation devices, not necessary to retain the demo's character. All effects that do ship are default-on at their appropriate trigger; later slices are not hidden opt-in features.

## glyphfx integration

Use glyphfx for text-oriented effects; custom Rust layers for the demo's starfield, plasma, copper and geometry-aware task flights.

There is an existing in-process precedent: `hammer_05/csrc/gfx_shim.c:30` opens an effect, `:66` steps it, and `hammer_05/src/gfx.rs:129` decodes frames. Reuse the approach, not the demo's whole implementation:

1. Pin/vendor the C engine with its MIT license and NOTICE; build a static bridge, not an external executable dependency. Add the Rust build dependency explicitly.
2. Wrap creation/stepping/destruction behind a safe Rust owner. Step on demand; never pre-render every frame at startup as Hammer does.
3. Composite into a bounded ratatui scratch buffer. Ratatui alone owns stdout, cursor, alternate screen and terminal restoration. Do not pipe glyphfx ANSI directly to the terminal.
4. The present frame interface returns ANSI strings, not ratatui cells. First spike must verify bounded decoding, background/style preservation, Unicode widths, terminal row orientation, cancellation, and exact final-frame restoration. Hammer's parser counts chars as columns and retains only foreground; it is not production-ready for arbitrary calendar text.
5. The engine uses global render-log state. Serialize glyphfx clips initially; do not assume concurrent contexts are safe. Native background layers can still run alongside a single glyphfx clip.
6. On resize or content invalidation, cancel and render the current canonical UI. Disabling or finishing effects has the same invariant. On engine failure, retain the normal UI and report a nonblocking diagnostic.

Initial glyphfx palette:

- `wipe`, `slide`, `middleout`: panel entry and transitions.
- `print`: completed proposal reveal; no artificial solver-progress typing.
- `highlight`: save, sync, import and apply confirmation on affected content only.
- `beams`, `sweep`, `colorshift`: bounded success/heading accents, preserving semantic colors.
- `decrypt`, `laseretch`, `matrix`, `synthgrid`: credits/brand surfaces, not event titles or error messages.
- Particle/destructive families (`fireworks`, `blackhole`, `crumble`, `rings`, `swarm`, etc.) are technically compositable, but not proposed for the working calendar. Evaluate them in a preview/credits surface rather than randomly applying them to daily navigation.

## Implementation order and acceptance gates

### 1. Foundation and agenda vertical slice

- Add a small TUI-local effects state and persisted preferences, separate from solver settings. Master `enabled = true`; each shipped effect also defaults true. Expose master and individual on/off commands through the existing command palette, with visible current values and persistence across restart.
- Give animation an elapsed-time clock and explicit next-frame deadline. Planner123 currently uses a 250 ms inactivity poll (`src/main.rs:52`, `src/event.rs:30`); that is unsuitable for smooth animation and can defer ticks under continuous input. Separate animation deadlines from worker/status ticks. Target 30 fps initially, not an unconditional high-frequency redraw loop.
- Composite background, canonical content, bounded foreground accents, then overlays. Preserve selected rows and error states. Keep every source file under the repository's 300-line limit.
- Deliver stars, eased agenda scrolling and pressed-chip feedback first.
- Gate: real-terminal recording at narrow, normal and wide sizes; normal and selected text readable; inputs handled during animation; resize and rapid navigation clean; default-on verified; master and individual toggles persist and stop active motion immediately.

### 2. glyphfx bridge and transactional feedback

- Spike `highlight` and `wipe` on real styled Unicode text before committing to the bridge shape.
- Add view transitions, proposal reveal and post-apply sweep/ripples, driven by worker results (`src/app/worker_results.rs:78` and `:86`) and the matching loaded event IDs. Effects never delay or initiate mutations. A stale proposal must not leave a ghost preview.
- Gate: canonical final buffer matches effects-off rendering exactly; failed apply never celebrates; refreshes do not replay old feedback; cancellation/failure leaks neither memory nor terminal state.

### 3. Ambient calendar layers

- Add low-amplitude copper and plasma, using live layout rectangles and theme colors rather than demo constants. Ensure existing event backgrounds do not erase the intended layer, and effects never replace event glyphs.
- Gate: inspect real terminal renders, measure frame time/CPU/terminal bytes for each layer and combined effects; reduce decorative update rate if needed. Inactive views do no animation work; master-off stops animation-only redraws.

### 4. Geometry-aware preview and credits

- Add proposal-preview flights only after real task-to-slot geometry and unapplied proposal semantics are established. Avoid forced view changes merely to play an effect.
- Add a credits surface for the more theatrical glyphfx effects and terminal approximations of the sine scroller/logo. These effects play by default when that surface is opened.
- Gate: real proposal data, overlap lanes, recurrence, offscreen targets and resize; no hardcoded dates, fake tasks, soundtrack dependency or pixel-renderer dependency.

For each implementation slice: deterministic frame-time tests and toggle persistence tests, `make test`, `make lint`, `make build`, plus real PTY interaction and visual inspection. Planning did not exercise these gates; they are acceptance criteria for implementation.
