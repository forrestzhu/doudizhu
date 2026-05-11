#!/usr/bin/env sh
set -eu

cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo run --bin harness -- --games 10 --seed 42
cargo run --bin harness -- --scenario evals/scenarios/bomb_beats_pair.json
cargo run --bin harness -- --scenario evals/scenarios/visibility_no_leak.json
cargo run --bin harness -- --scenario evals/scenarios/seeded_self_play_smoke.json --format json >/dev/null
