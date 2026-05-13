# Quickstart: Core Arena

## Run Tests

```sh
cargo test
```

## Run One Seeded Game

```sh
cargo run --bin arena -- --seed 42
```

## Run Multiple Simulations

```sh
cargo run --bin arena -- --games 10 --seed 42
```

## Run Eval Scenarios

```sh
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json
cargo run --bin arena -- --scenario evals/scenarios/visibility_no_leak.json --format json
```

## Run All Local Checks

```sh
./scripts/verify.sh
```

## Run Electron Play Prototype

```sh
npm install
npm start
```

The Electron shell starts a deterministic game, renders the selected player view
without opponent hand data, can request a rule-policy hint, and can step through
automatic play driven by the Rust trace report.

## Run Electron E2E Tests

```sh
npm run test:e2e
```

The E2E tests launch Electron with Playwright and exercise the real Rust arena
session/trace flow.

## Inspect Session And Trace JSON

```sh
cargo run --bin arena -- --session --seed 42 --viewer 0 --format json
cargo run --bin arena -- --trace --seed 42 --format json
cargo run --bin arena -- --trace --seed 42 --allow-power --format json
```

`--session` is player-facing and only includes the selected viewer's hand.
`--trace` is trusted evaluation data and includes complete initial hands,
bottom cards, public turn history, policy configuration, and outcome for
strategy optimization. By default the rule policy avoids spending bombs and
rockets when a normal response exists; `--allow-power` records a configuration
that permits spending those hands.

## Expected Output Shape

```text
game=1 seed=42 winner=0 turns=...
summary games=10 wins=[..., ..., ...] avg_turns=...
```

The exact winners and turn counts are deterministic for a given code version and
seed. Use them as regression signals after rule or policy changes.
