# Eval Arena

The arena follows an eval-driven pattern: define expected behavior as data,
run it deterministically, and make the output machine-readable.

## Why This Exists

The project will eventually have multiple decision systems: humans, rule bots,
search players, and LLM players. Those systems need a stable evaluation loop
that is independent of Electron and independent of subjective review.

## Commands

Run seeded self-play:

```sh
cargo run --bin arena -- --games 10 --seed 42
```

Run seeded self-play with JSON:

```sh
cargo run --bin arena -- --games 10 --seed 42 --format json
```

Run one scenario:

```sh
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json
```

Run one scenario with JSON:

```sh
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json --format json
```

Run all local quality gates:

```sh
./scripts/verify.sh
```

## Scenario Contract

Scenario files live in `evals/scenarios/` and use compact card codes:

- `3C`, `10H`, `AS`, `2D`
- `BJ` for black joker
- `RJ` for red joker

### `legal_candidates`

Checks legal candidate generation for a hand and optional previous play.

```json
{
  "name": "bomb_beats_pair",
  "kind": "legal_candidates",
  "hand": ["4C", "4D", "4H", "4S", "3C"],
  "previous_play": ["7C", "7D"],
  "expect": {
    "contains": [["4C", "4D", "4H", "4S"]],
    "excludes": [["3C"]]
  }
}
```

### `visibility`

Checks that a player's projected view contains expected public state and excludes
hidden cards.

### `self_play`

Runs deterministic seeded games and checks aggregate outcomes.

## Report Contract

Reports include:

- `schema_version`: output contract version
- `deterministic`: always `true` for current arena reports
- `pass`: aggregate pass/fail
- `checks`: individual assertions
- `metrics`: numeric summary values
- `details`: kind-specific payload

Use `--format json` for future CI, dashboards, or LLM policy evals.
