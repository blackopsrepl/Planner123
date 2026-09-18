# AGENT.md

## Purpose

This repository contains a Linux-first Rust calendar with two supported entrypoints:

- `planner123`: ratatui TUI application
- `planner123-cli`: non-interactive JSON CLI for agents and automation

## Commands

- `make build`: build both binaries
- `make run`: launch the TUI
- `make run-cli ARGS="calendars list"`: run the automation CLI
- `make test`: run all tests
- `make lint`: run formatting and clippy checks
- `make ci-local`: match the GitHub Actions CI workflow locally
- `make pre-release`: run release-oriented validation before tagging

Direct cargo commands used in CI:

- `cargo fmt --all --check`
- `cargo clippy --bins --tests -- -D warnings`
- `cargo build --release --bins`
- `cargo test`

## Repo map

- `PRD.md`: current product requirements for the v0.4.x milestone
- `src/main.rs`: TUI entrypoint
- `src/bin/planner123-cli.rs`: CLI entrypoint
- `src/app.rs` and `src/app/`: stable TUI facade plus state, dispatch, navigation, forms, planner, integrations, and worker-result modules
- `src/cli.rs` and `src/cli/`: stable typed CLI facade plus arguments, handlers, runtime, validation, and backend modules
- `src/calendar_service.rs` and `src/calendar_service/`: shared calendar facade plus validation, mutation, and test modules
- `src/event_service.rs` and `src/event_service/`: shared event facade plus validation, mutation, and test modules
- `src/ical.rs` and `src/ical/`: `.ics` facade plus parsing, candidate, time, import/export, and test modules
- `src/google/`: OAuth, calendar discovery, typed event API, and Google payload mapping; its larger concerns use the same facade-plus-fragments pattern
- `src/sync/`: sync engine, pull, push, conflicts, and persisted sync-state helpers, each split by responsibility
- `src/db.rs` and `src/db/`: SQLite facade plus schema migrations, CRUD families, sync metadata, planner evidence, and tests
- `src/planner.rs` and `src/planner/`: planner facade plus settings, tasks, availability, optimization, proposal lifecycle, evidence, and tests
- `src/models.rs` and `src/models/`: stable data-model facade plus calendar, event, dependency, planning, and proposal types
- `tests/cli.rs` and `tests/cli/`: binary-level CLI integration facade plus command-family test modules
- `docs/wireframes/`: ASCII references for the CLI and TUI surfaces

## Constraints

- Keep the CLI fully non-interactive. No prompts, no confirmation flows, no choices.
- Preserve `cargo run` as the TUI default path.
- Keep agent automation explicit through `planner123-cli` and `scripts/planner123-cli`.
- Route event mutation behavior through shared services, not UI-local rules.
- Treat Google sync as explicit and deterministic. No hidden auto-sync startup behavior.
- Tests must stay deterministic. Do not add live Google API or real keyring dependencies to automated tests.

## Change checklist

Before pushing changes:

1. Run `make lint`
2. Run `make test`
3. If binaries changed materially, run `make build`
4. Before tagging or pushing a release version, run `make pre-release`

If you touch the CLI contract, update:

- `README.md`
- `tests/cli.rs`
- `docs/wireframes/cli.md`

## Source size and module boundaries

Keep every tracked Rust source and test file below 300 lines. Large public
subsystems retain their existing root module as a stable facade and place
cohesive implementation fragments in the matching directory. Move comments
with the code they explain; do not delete them while restructuring.

If you touch Google sync or `.ics` behavior, update:

- `README.md`
- `PRD.md` if the planned scope changes
- `docs/wireframes/tui.md` and `docs/wireframes/cli.md` when user-facing flows change
