# SolverForge Calendar Makefile

GREEN := \033[92m
CYAN := \033[96m
YELLOW := \033[93m
RED := \033[91m
BOLD := \033[1m
RESET := \033[0m

CHECK := ✓
CROSS := ✗
ARROW := ▸

VERSION := $(shell grep -m1 '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
CARGO_HOME ?= $(HOME)/.cargo

.PHONY: help build build-release install uninstall run run-cli test lint fmt fmt-check clippy ci-local pre-release clean version
.DEFAULT_GOAL := help

help:
	@printf "$(CYAN)$(BOLD)SolverForge Calendar$(RESET) v$(VERSION)\n\n"
	@printf "$(ARROW) build          Build the TUI and CLI binaries\n"
	@printf "$(ARROW) build-release  Build optimized binaries\n"
	@printf "$(ARROW) install        Install both binaries into $(CARGO_HOME)/bin\n"
	@printf "$(ARROW) uninstall      Remove both installed binaries\n"
	@printf "$(ARROW) run            Launch the TUI\n"
	@printf "$(ARROW) run-cli        Run the agent CLI, pass ARGS='...'\n"
	@printf "$(ARROW) test           Run the full test suite\n"
	@printf "$(ARROW) lint           Run formatting and clippy checks\n"
	@printf "$(ARROW) fmt            Format the repo\n"
	@printf "$(ARROW) fmt-check      Check formatting without rewriting\n"
	@printf "$(ARROW) clippy         Run clippy with CI flags\n"
	@printf "$(ARROW) ci-local       Simulate the GitHub Actions workflow\n"
	@printf "$(ARROW) pre-release    Run release-oriented validation\n"
	@printf "$(ARROW) clean          Remove build artifacts\n"
	@printf "$(ARROW) version        Print the current crate version\n\n"

build:
	@printf "$(ARROW) Building binaries...\n"
	@cargo build --bins && printf "$(GREEN)$(CHECK) Build passed$(RESET)\n" || (printf "$(RED)$(CROSS) Build failed$(RESET)\n" && exit 1)

build-release:
	@printf "$(ARROW) Building release binaries...\n"
	@cargo build --release --bins && printf "$(GREEN)$(CHECK) Release build passed$(RESET)\n" || (printf "$(RED)$(CROSS) Release build failed$(RESET)\n" && exit 1)

install:
	@printf "$(ARROW) Installing solverforge-calendar and solverforge-calendar-cli into $(CARGO_HOME)/bin...\n"
	@cargo install --path . --locked --bins --force && printf "$(GREEN)$(CHECK) Install passed$(RESET)\n" || (printf "$(RED)$(CROSS) Install failed$(RESET)\n" && exit 1)

uninstall:
	@printf "$(ARROW) Removing installed binaries from $(CARGO_HOME)/bin...\n"
	@cargo uninstall solverforge-calendar && printf "$(GREEN)$(CHECK) Uninstall passed$(RESET)\n" || (printf "$(RED)$(CROSS) Uninstall failed$(RESET)\n" && exit 1)

run:
	@cargo run

run-cli:
	@cargo run --bin solverforge-calendar-cli -- $(ARGS)

test:
	@printf "$(ARROW) Running tests...\n"
	@cargo test && printf "$(GREEN)$(CHECK) Tests passed$(RESET)\n" || (printf "$(RED)$(CROSS) Tests failed$(RESET)\n" && exit 1)

fmt:
	@cargo fmt --all

fmt-check:
	@printf "$(ARROW) Checking formatting...\n"
	@cargo fmt --all --check && printf "$(GREEN)$(CHECK) Formatting valid$(RESET)\n" || (printf "$(RED)$(CROSS) Formatting issues found$(RESET)\n" && exit 1)

clippy:
	@printf "$(ARROW) Running clippy...\n"
	@cargo clippy --bins --tests -- -D warnings && printf "$(GREEN)$(CHECK) Clippy passed$(RESET)\n" || (printf "$(RED)$(CROSS) Clippy failed$(RESET)\n" && exit 1)

lint: fmt-check clippy
	@printf "$(GREEN)$(CHECK) Lint checks passed$(RESET)\n"

ci-local: fmt-check clippy
	@printf "$(ARROW) Building binaries...\n"
	@cargo build --bins
	@printf "$(ARROW) Running tests...\n"
	@cargo test
	@printf "$(GREEN)$(BOLD)$(CHECK) Local CI passed$(RESET)\n"

pre-release: fmt-check clippy
	@printf "$(ARROW) Building release binaries...\n"
	@cargo build --release --bins
	@printf "$(ARROW) Running release validation tests...\n"
	@cargo test
	@printf "$(GREEN)$(BOLD)$(CHECK) Pre-release checks passed for v$(VERSION)$(RESET)\n"

clean:
	@cargo clean

version:
	@printf "$(YELLOW)$(BOLD)$(VERSION)$(RESET)\n"
