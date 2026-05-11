# Implementation Plan: Core Harness And Pluggable Decisions

**Branch**: `001-core-harness` | **Date**: 2026-05-11 | **Spec**: `specs/001-core-harness/spec.md`

## Summary

Create a UI-independent Rust core that can model a reduced Dou Dizhu game,
project player-specific visibility, accept pluggable decision policies, and run
deterministic self-play through a CLI harness.

## Technical Context

**Language/Version**: Rust 1.93  
**Primary Dependencies**: Rust standard library for core logic; `serde` and
`serde_json` for scenario fixtures and machine-readable harness reports  
**Storage**: In-memory state  
**Testing**: `cargo test`; seeded CLI harness runs  
**Target Type**: Library plus CLI binary; future Electron adapter  
**Constraints**: No UI dependency in core, deterministic seeds, no opponent-hand
leaks in default visibility  
**Scope**: Initial reduced rules: single, pair, triple, bomb, rocket

## Constitution Check

- Core-first game logic: satisfied by `src/lib.rs` and modules under `src/`.
- Pluggable decisions: satisfied by `DecisionPolicy`.
- Visibility contract: satisfied by `PlayerView` projection tests.
- Executable harness before UI: satisfied by `src/bin/harness.rs`.
- Spec-anchored development: this plan and `spec.md` describe the current slice.

## Project Structure

```text
src/
  cards.rs        # card/rank/suit/deck model
  rules.rs        # rule classification and comparison
  visibility.rs   # player view contract
  decision.rs     # decision trait and baseline bot
  engine.rs       # deal, turn engine, history, validation
  bin/harness.rs  # seeded self-play CLI
docs/
  architecture.md
  eval-harness.md
evals/
  scenarios/      # deterministic eval fixtures
specs/001-core-harness/
  spec.md
  plan.md
  data-model.md
  quickstart.md
```

## Phase 0 Decisions

- Use a reduced rule set first to prove contracts and verification loop.
- Keep full Dou Dizhu rule coverage as future expansion behind `RuleSet`.
- Treat player 0 as landlord for the initial relationship model and give player 0
  the bottom cards immediately.
- Do not model landlord bidding or bottom-card reveal in this slice.

## Phase 1 Design

- `Card` is value-type state and can be freely copied.
- `RuleSet` validates hand semantics and comparison.
- `DecisionPolicy` sees only `PlayerView` and returns `Decision`.
- `Game` owns hidden state and validates every decision before mutation.
- CLI harness runs N seeded games and exits non-zero on engine error.
- Eval scenarios run through the same CLI and produce stable JSON reports.

## Verification

```sh
cargo fmt --check
cargo test
cargo run --bin harness -- --games 10 --seed 42
cargo run --bin harness -- --scenario evals/scenarios/bomb_beats_pair.json --format json
```

## Next Slices

- Add complete Dou Dizhu hand recognition.
- Add explicit landlord bidding and bottom-card visibility.
- Add scripted scenario files for regression decks.
- Add Electron main/preload bridge over the core engine.
- Add JSON contract for out-of-process LLM decision policies.
