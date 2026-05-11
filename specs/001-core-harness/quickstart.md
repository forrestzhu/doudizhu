# Quickstart: Core Harness

## Run Tests

```sh
cargo test
```

## Run One Seeded Game

```sh
cargo run --bin harness -- --seed 42
```

## Run Multiple Simulations

```sh
cargo run --bin harness -- --games 10 --seed 42
```

## Run Eval Scenarios

```sh
cargo run --bin harness -- --scenario evals/scenarios/bomb_beats_pair.json
cargo run --bin harness -- --scenario evals/scenarios/visibility_no_leak.json --format json
```

## Run All Local Checks

```sh
./scripts/verify.sh
```

## Run Electron Deal Prototype

```sh
npm install
npm start
```

The Electron shell calls the Rust harness deal endpoint and renders the selected
player view without opponent hand data.

## Run Electron E2E Tests

```sh
npm run test:e2e
```

The E2E tests launch Electron with Playwright and exercise the real Rust harness
deal endpoint.

## Expected Output Shape

```text
game=1 seed=42 winner=0 turns=...
summary games=10 wins=[..., ..., ...] avg_turns=...
```

The exact winners and turn counts are deterministic for a given code version and
seed. Use them as regression signals after rule or policy changes.
