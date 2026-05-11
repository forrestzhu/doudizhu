#!/usr/bin/env sh
set -eu

cargo fmt --check
cargo test
cargo clippy -- -D warnings
npm run check:ui
npm run test:e2e
cargo run --bin harness -- --deal --seed 42 --viewer 0 --format json >/dev/null
cargo run --bin harness -- --session --seed 42 --viewer 0 --format json >/dev/null
cargo run --bin harness -- --trace --seed 42 --format json >/dev/null
cargo run --bin harness -- --trace --seed 42 --allow-power --format json >/dev/null
cargo run --bin harness -- --games 10 --seed 42
cargo run --bin harness -- --scenario evals/scenarios/bomb_beats_pair.json
cargo run --bin harness -- --scenario evals/scenarios/visibility_no_leak.json
cargo run --bin harness -- --scenario evals/scenarios/seeded_self_play_smoke.json --format json >/dev/null
