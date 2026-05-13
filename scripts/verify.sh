#!/usr/bin/env sh
set -eu

cargo fmt --check
cargo test
cargo clippy -- -D warnings
npm run check:ui
npm run test:e2e
cargo run --bin arena -- --deal --seed 42 --viewer 0 --format json >/dev/null
cargo run --bin arena -- --session --seed 42 --viewer 0 --format json >/dev/null
cargo run --bin arena -- --trace --seed 42 --format json >/dev/null
cargo run --bin arena -- --trace --seed 42 --allow-power --format json >/dev/null
cargo run --bin arena -- --games 10 --seed 42
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json
cargo run --bin arena -- --scenario evals/scenarios/visibility_no_leak.json
cargo run --bin arena -- --scenario evals/scenarios/seeded_self_play_smoke.json --format json >/dev/null
