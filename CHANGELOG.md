# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## [0.6.11](https://github.com/blackopsrepl/Planner123/compare/v0.6.10...v0.6.11) (2026-10-30)


### Bug Fixes

* **effects:** bring the background layers back, tinted evenly ([b4c6fee](https://github.com/blackopsrepl/Planner123/commit/b4c6fee3d46ee2dcd15506d88bc3a300d860a0b0))
* **tui:** stop reading the keyring on every frame ([ccdae1e](https://github.com/blackopsrepl/Planner123/commit/ccdae1e5b959963c50562eacaa57a6d40d1012b9))


## [0.6.10](https://github.com/blackopsrepl/Planner123/compare/v0.6.9...v0.6.10) (2026-10-30)


### Bug Fixes

* **effects:** drop the background layers from calendar views ([2de51c1](https://github.com/blackopsrepl/Planner123/commit/2de51c1bd46f6f8ffbed37191f2e5f8ad1d6c43d))


## [0.6.9](https://github.com/blackopsrepl/Planner123/compare/v0.6.8...v0.6.9) (2026-10-30)


### Bug Fixes

* **effects:** give glyphfx copper its own toggle ([1150f39](https://github.com/blackopsrepl/Planner123/commit/1150f39e6b645c3eca1a114928ce76601c04eb14))


## [0.6.8](https://github.com/blackopsrepl/Planner123/compare/v0.6.7...v0.6.8) (2026-10-30)


### Features

* **effects:** render glyphfx text effects into ratatui buffers ([baae6a2](https://github.com/blackopsrepl/Planner123/commit/baae6a23b977477abc6697b1d10be224a1c3e99c))
* **tui:** bring the Planner06 demoscene effects into the real app ([ecc114a](https://github.com/blackopsrepl/Planner123/commit/ecc114a227560074b73979839340bf43104cbec2))


### Bug Fixes

* **db:** serialize concurrent first-launch initialization ([757ee60](https://github.com/blackopsrepl/Planner123/commit/757ee60d08c958d8b1d37ff2db66dfd54500f668))
* **tui:** keep worker ticks independent of input and frame deadlines ([e149f6f](https://github.com/blackopsrepl/Planner123/commit/e149f6ffbd3d1dc735f84176ca42ac6dbfdd018e))


## [0.6.7](https://github.com/blackopsrepl/Planner123/compare/v0.6.6...v0.6.7) (2026-10-29)


### Features

* **planner:** move to SolverForge 0.23.0 and schedule what fits ([63982f6](https://github.com/blackopsrepl/Planner123/commit/63982f6f773e8e520d8ee2f610207151d6152f58))


### Bug Fixes

* **calendar:** draw every occurrence of a recurring event ([ef1d3c0](https://github.com/blackopsrepl/Planner123/commit/ef1d3c03f95aac2359e92a461d0fb194bd7db502))
* **week:** draw every event, side by side when they overlap ([d5572ad](https://github.com/blackopsrepl/Planner123/commit/d5572ad346581f4d97e02b9bb126033d10bcef64))


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
