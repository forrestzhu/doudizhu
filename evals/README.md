# Doudizhu Evals

This directory stores deterministic eval scenarios for the core game engine.
Evals are first-class regression assets: update them with rule, visibility, or
decision-contract changes.

## Run A Scenario

```sh
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json
```

## JSON Output

```sh
cargo run --bin arena -- --scenario evals/scenarios/bomb_beats_pair.json --format json
```

Every report includes:

- `schema_version`
- `deterministic`
- `name`
- `kind`
- `pass`
- `checks`
- `metrics`
- `details`

## Scenario Kinds

- `legal_candidates`: verifies that a hand and previous play produce expected
  legal or illegal candidate hands.
- `visibility`: verifies that a player's view includes expected public state and
  excludes hidden cards.
- `self_play`: runs one or more seeded games and checks deterministic aggregate
  results.

