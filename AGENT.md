# AGENT.md

## Purpose

This repository contains a Linux-first Rust calendar with two supported entrypoints:

- `solverforge-calendar`: ratatui TUI application
- `solverforge-calendar-cli`: non-interactive JSON CLI for agents and automation

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
- `src/bin/solverforge-calendar-cli.rs`: CLI entrypoint
- `src/app.rs`: TUI state machine and worker result handling
- `src/cli.rs`: typed CLI parsing, JSON responses, and command dispatch
- `src/calendar_service.rs`: shared calendar validation and Google import rules
- `src/event_service.rs`: shared event validation, timezone normalization, and Google outbox rules
- `src/ical.rs`: `.ics` import/export
- `src/google/`: OAuth, calendar discovery, typed event API, and Google payload mapping
- `src/sync/`: sync engine, pull, push, conflicts, and persisted sync state helpers
- `src/db.rs`: SQLite schema, migrations, CRUD helpers, sync metadata tables, and planner proposal evidence
- `tests/cli.rs`: binary-level CLI integration tests, including the JSON-first planner contract
- `docs/wireframes/`: ASCII references for the CLI and TUI surfaces

## Constraints

- Keep the CLI fully non-interactive. No prompts, no confirmation flows, no choices.
- Preserve `cargo run` as the TUI default path.
- Keep agent automation explicit through `solverforge-calendar-cli` and `scripts/solverforge-calendar-cli`.
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

If you touch Google sync or `.ics` behavior, update:

- `README.md`
- `PRD.md` if the planned scope changes
- `docs/wireframes/tui.md` and `docs/wireframes/cli.md` when user-facing flows change
