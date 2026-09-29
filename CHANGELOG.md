# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## [0.6.6](https://github.com/blackopsrepl/Planner123/compare/v0.6.5...v0.6.6) (2026-09-29)


### Bug Fixes

* **keys:** make Enter mean something, and stop binding keys that do nothing c12cb8f

## [0.6.5](https://github.com/blackopsrepl/Planner123/compare/v0.6.4...v0.6.5) (2026-09-29)


### Features

* **tui:** filter the help overlay by typing, and open it on the right section 5ea8dcf
* **tui:** set a planner task's timing in the form d2a7484

## [0.6.4](https://github.com/blackopsrepl/Planner123/compare/v0.6.3...v0.6.4) (2026-09-29)


### Features

* **tui:** ask before deleting or exporting 17773a0


### Bug Fixes

* **tui:** expire the status message 1de7ded
* **tui:** stop showing project progress that can never move f310a29

## [0.6.3](https://github.com/blackopsrepl/Planner123/compare/v0.6.2...v0.6.3) (2026-09-29)


### Features

* **keys:** apply a planner proposal with A ea7dc92
* **keys:** one way to change a select value, on every form bd8e80c
* **keys:** take the rare actions off the top-level letters 5a96565
* **tui:** add a command palette and a working go-to-date 5746b46


### Bug Fixes

* **tui:** return overlays to the view they were opened from 8bc26ca

## [0.6.2](https://github.com/blackopsrepl/Planner123/compare/v0.6.1...v0.6.2) (2026-09-29)


### Features

* **tui:** build the help overlay from the keymap registry 2a9f7c2

## [0.6.1](https://github.com/blackopsrepl/Planner123/compare/v0.6.0...v0.6.1) (2026-09-29)


### Bug Fixes

* **tui:** give the status bar a width budget and a priority order 3594dde

## [0.6.0](https://github.com/blackopsrepl/Planner123/compare/v0.5.0...v0.6.0) (2026-09-28)

### ⚠ BREAKING CHANGES

* rename application to Planner123

### Features

* rename application to Planner123 7b86806
* **skill:** rename agent skill to planner123 07d0e52

### Bug Fixes

* **app:** reload events from the loaded window, not the display month 51367db, closes #2
* **app:** route returns to the month view through the reload path 809808a

## [0.5.0](https://github.com/blackopsrepl/Planner123/compare/v0.4.0...v0.5.0) (2026-09-16)

### Features

* **agent:** add portable agent skill with installer f628994

### Bug Fixes

* **planner:** bound lateness above the slot grid edge c2274f3
* **planner:** enforce unified recovery thresholds 0788b44
* **planner:** keep horizon overrides applicable ac0e1b8
* **planner:** make proposal penalties exact and additive b8fdb58
* **planner:** migrate legacy proposal diagnostics safely 5098964
* **planner:** preserve assignment over deadline lateness dace2c0
* **planner:** reach the whole horizon when availability is sparse eec9443
* **planner:** reject unsafe scoring ranges dcaa50e
* **planner:** require assigned dependency predecessors 82289d3
* **planner:** restore applied dependency conflicts and blocker evidence 716242c
* **planner:** restore deadline and recovery semantics faec51d

## [0.4.0](https://github.com/blackopsrepl/Planner123/compare/v0.3.0...v0.4.0) (2026-09-02)

### Features

* **google:** add compliant auth and calendar import flows ec98978
* **ical:** add real .ics import flows 56f3800
* **planner:** persist proposal applicability evidence e26dffa
* **planner:** schedule task inbox with SolverForge 75569a4
* **sync:** add durable sync state and outbox tables dadf378
* **sync:** implement two-way Google sync and conflict resolution 51ef740
* **tui:** configure planner availability in-app 23155aa
* **tui:** surface google sync health in the interface 87a6420

### Bug Fixes

* **planner:** block recurring calendar occurrences 7e16b06
* **planner:** clear applied tasks from inbox fcb1efc
* **planner:** enforce proposal lifecycle integrity 688c1d9
* **planner:** explain weekly availability 8a7544b
* **planner:** guide timezone configuration 500fd2c
* **sync:** canonicalize recurrence and planner horizons c78697a
* **sync:** correct conflict retries and sync status reporting c10d0f3
* **sync:** preserve Google event identity 055d891
* **time:** use canonical timezone filter c50da3f
* **tui:** expose and exit planner workspace 8ec8aca

## [0.3.0](https://github.com/blackopsrepl/Planner123/compare/9ebe1f8be071ac2d1ab0bdcd511160e5f2b1239e...v0.3.0) (2026-04-06)

### Features

* finish CLI coverage and add mascot ccf8949
* harden google calendar sync workflows 44e9c97

### Bug Fixes

* fix event cards not spanning full duration in week/day view 9ebe1f8
* harden agent CLI and sync paths ea092de
