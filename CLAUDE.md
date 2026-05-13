# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```sh
# Rust checks
cargo fmt --check          # format check
cargo test                 # all unit + inline tests
cargo clippy -- -D warnings  # lint (warnings as errors)

# Single test (by name or module)
cargo test test_name
cargo test --lib rules     # all tests in src/rules.rs
cargo test --lib decision  # all tests in src/decision.rs

# Arena CLI
cargo run --bin arena -- --seed 42                           # single seeded game
cargo run --bin arena -- --games 10 --seed 42                # tournament simulation
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json  # eval scenario
cargo run --bin arena -- --trace --seed 42 --format json     # full trace JSON
cargo run --bin arena -- --strategy-file strategies/strategic_v1.json --games 50  # custom strategy

# Electron
npm install
npm start                  # launch desktop app
npm run check:ui           # syntax-check all JS files
npm run test:e2e           # Playwright E2E tests (launches real Electron)

# Full verification gate
./scripts/verify.sh
```

## Architecture

Rust core library (`src/`) with Electron desktop shell (`electron/` + `renderer/`). The Rust core is the sole source of truth; Electron is a thin JSON-over-stdout adapter.

### Rust Core (`src/`)

Module dependency: `lib.rs` -> `{cards, rules, visibility, decision, engine, arena}`

- **`cards.rs`** — `Card`, `Rank`, `Suit`, 54-card deck, `Display`/`FromStr`
- **`rules.rs`** — `RuleSet` trait, `BasicRules` impl. All 14 `HandKind` variants (Single through Rocket). `ClassifiedHand` for classified comparisons. ~500 lines of inline tests covering classification and comparison
- **`visibility.rs`** — `PlayerView` (hidden-information-safe projection), `Relationship` enum (SelfPlayer/Ally/Opponent). Never contains opponent hands
- **`decision.rs`** — `DecisionPolicy` trait. Three policies: `LowestLegalPolicy`, `RuleBasedPolicy` (configurable power-hand avoidance), `StrategicPolicy` (multi-factor scoring with configurable weights via JSON). `legal_candidates` generator for combo enumeration
- **`engine.rs`** — `Deal` (seeded shuffle), `Game` (mutable turn engine). Invalid decisions are errors, never silent corrections. `landlord_relationships` defines ally/opponent matrix
- **`arena.rs`** — Eval arena: scenario runner (legal_candidates/visibility/self_play), session/trace generation, tournament mode, report structs
- **`bin/arena.rs`** — CLI entry point parsing args and dispatching to arena

### Electron Layer

- **`electron/main.js`** — Spawns `cargo run --bin arena` via `execFile` for every IPC call. In-memory session map. No game logic in JS
- **`electron/preload.js`** — `contextBridge` exposing `window.doudizhu`
- **`renderer/`** — Vanilla JS/CSS (no framework). Chinese-language UI

### Key Design Decisions

- Player 0 is always landlord (no bidding phase yet)
- Seeded LCG shuffle for deterministic replays
- All arena output is JSON with `deterministic: true` and `schema_version`
- `--session` is player-facing (respects visibility); `--trace` is trusted (full hands for strategy optimization)
- `--allow-power` flag controls whether the rule policy spends bombs/rockets

## Commit Conventions

Conventional Commits with typed scopes: `type(scope): summary`. Scopes: `core`, `rules`, `visibility`, `decision`, `arena`, `electron`, `spec`, `docs`. See `docs/git-commit-conventions.md` for full spec. Include arena updates in the same commit as behavior changes.

## Critical Invariants

- `PlayerView` must never leak opponent hands. E2E tests scan for leaked card codes
- Invalid decisions must produce `GameError`, not silent corrections
- Deterministic output: same seed + same code = same game. Regressions signal rule or policy bugs
- Eval scenarios under `evals/scenarios/` are regression fixtures — update them intentionally

## Specs

Spec Kit artifacts live under `specs/`. Current plan: `specs/001-core-arena/plan.md`. Prefer Spec Kit for non-trivial feature design.
