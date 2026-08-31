# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## Unreleased

- make Planner Inbox availability configuration accessible from the Ratatui TUI
- replace raw planner availability JSON CLI input with typed weekly windows
- persist proposal blocker evidence and invalidate proposals after settings or dependency changes

### Features

- compliant desktop Google OAuth, calendar discovery/import, and writable vs read-only calendar handling
- true two-way Google sync with a durable outbox, ETag-based conflict detection, and CLI conflict resolution
- timezone-correct event flows shared across the TUI, CLI, `.ics` import, and Google sync
- real `.ics` import in both the CLI and the TUI, with warnings for unsupported ICS shapes

### Documentation

- align README, wireframes, help text, and contributor guidance with the current shipped behavior

## 0.3.0 (2026-04-06)


### Features

* finish CLI coverage and add mascot ccf8949
* harden google calendar sync workflows 44e9c97


### Bug Fixes

* fix event cards not spanning full duration in week/day view 9ebe1f8
* harden agent CLI and sync paths ea092de
