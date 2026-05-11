<!--
Sync Impact Report
Version change: N/A -> 0.1.0
Modified principles: template placeholders -> concrete doudizhu engineering principles
Added sections: Architecture Constraints, Spec Kit Workflow
Removed sections: none
Templates requiring updates: pending; default templates remain usable for the initial project phase
Follow-up TODOs: none
-->

# Doudizhu Constitution

## Core Principles

### I. Core-First Game Logic
All game rules, state transitions, visibility calculations, and decision contracts
MUST live in a UI-independent core library. Tauri and any future frontends consume
the same public contracts used by automated tests and harnesses.

### II. Pluggable Decision Systems
Player decision logic MUST be behind a stable input/output boundary. A decision
policy receives only that player's legal view of the game and returns either pass
or a candidate hand to play. Human UI control, rule-based bots, search agents, and
LLM agents MUST all be replaceable without changing the game engine.

### III. Visibility Is a Contract
Each player view MUST expose the player's own hand, public turn history, visible
hand counts, relationships, and the current required response. It MUST NOT expose
hidden opponent cards or future deck information unless the selected game mode
explicitly declares that information public.

### IV. Executable Harness Before UI
Every lasting behavior change MUST be testable without Tauri. The project MUST
maintain a deterministic CLI/self-play harness that can deal cards, run decisions,
validate rule enforcement, and report outcomes for repeated simulations.

### V. Spec-Anchored Development
Non-trivial features MUST start from a Spec Kit spec and plan before implementation.
Specs define externally observable behavior; plans define implementation slices and
verification commands. Code, tests, and docs MUST stay aligned with the active spec.

## Architecture Constraints

- Rust is the source of truth for domain logic.
- Tauri is an adapter over the core library, not the owner of game rules.
- Rule engines must support incremental expansion from reduced rule sets to full
  Dou Dizhu rules.
- Harness scenarios must support deterministic seeds and hand-authored decks so
  regressions are reproducible.

## Spec Kit Workflow

- Use `$speckit-specify` for new user-visible capabilities.
- Use `$speckit-plan` before multi-step implementation.
- Use `$speckit-tasks` when work needs task-level sequencing.
- Run fresh verification commands before claiming implementation work is complete.

## Governance
This constitution governs architecture, specs, and verification for the repository.
Amendments require updating this file, documenting the version change in the sync
impact report, and checking affected specs/templates for consistency.

Versioning follows semantic versioning:
- MAJOR: incompatible changes to project principles or development workflow.
- MINOR: new principles, new mandatory quality gates, or new architectural sections.
- PATCH: clarifications that do not change required behavior.

**Version**: 0.1.0 | **Ratified**: 2026-05-11 | **Last Amended**: 2026-05-11
