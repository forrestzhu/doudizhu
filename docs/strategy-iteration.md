# Strategy Iteration Runbook

This document records the current policy-search loop so a future session can
continue after context is cleared.

## Current Champion

Use `strategies/strategic_v3.json` as the current best strategic policy.

```json
{
  "avoid_power_hands": true,
  "endgame_search_limit": 10,
  "power_cost_normal": 4,
  "power_cost_threat": 1,
  "lead_longer_tiebreak": true,
  "lead_tempo_plan_weight": 1,
  "stranded_risk_weight": 1,
  "opponent_urgency_weight": 1,
  "hand_control_weight": 2,
  "farmer_cooperation_weight": 3
}
```

`strategies/strategic_v2.json` is retained for historical comparison.

Relevant commits:

- `3028676 feat(decision): add strategic policy tournament`
- `ee45b34 chore(arena): add strategy tuning controls`
- `f1b8034 feat(decision): add strategic policy v2`

## Evaluation Goal

Evaluate three placements on the same random deal set:

1. `all_rule_based`: all players use the default rule-based policy.
2. `landlord_strategic`: landlord uses the candidate strategy, farmers use the
   default rule-based policy.
3. `farmers_strategic`: farmers use the candidate strategy, landlord uses the
   default rule-based policy.

The arena generates random deal seeds by default, but records `random_source`
and `deal_seeds` in JSON so runs can be reproduced.

## Commands

Use `--release` for benchmarking. Parallel 1000-game tournament (~33s on 12 cores):

```sh
cargo run --release --quiet --bin arena -- \
  --random-tournament \
  --games 1000 \
  --threads 12 \
  --strategy-file strategies/strategic_v3.json \
  --format json
```

With early termination (stops at pilot games if both roles regress by threshold):

```sh
cargo run --release --quiet --bin arena -- \
  --random-tournament \
  --games 1000 \
  --threads 12 \
  --early-stop 100 \
  --early-stop-regression 0.15 \
  --strategy-file strategies/strategic_v3.json \
  --format json
```

Reproduce a specific random tournament:

```sh
cargo run --release --quiet --bin arena -- \
  --random-tournament \
  --games 1000 \
  --threads 12 \
  --random-source <RANDOM_SOURCE> \
  --strategy-file strategies/strategic_v3.json \
  --format json
```

Quickly test parameter overrides without editing a strategy file:

```sh
cargo run --release --quiet --bin arena -- \
  --random-tournament \
  --games 1000 \
  --threads 12 \
  --random-source <RANDOM_SOURCE> \
  --strategy-file strategies/strategic_v3.json \
  --endgame-search-limit 8 \
  --power-cost-normal 2 \
  --power-cost-threat 0 \
  --prefer-short-leads \
  --format json
```

Only keep a candidate if verification passes:

```sh
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

## Promotion Rule

For the first strategic policy, use a clear threshold against `all_rule_based`:

- `landlord_strategic.landlord_win_rate - all_rule_based.landlord_win_rate >= 0.10`
- `farmers_strategic.farmer_win_rate - all_rule_based.farmer_win_rate >= 0.10`

For iterative versions after `strategic_v2`, compare candidate against the
current champion on the same `random_source` batches:

- Run at least three independent 1000-game random-source batches (with
  `--threads 12 --release` for speed).
- Do not accept a role-side regression greater than 2 percentage points on any
  batch.
- Prefer candidates that improve either landlord or farmer placement by at least
  2 percentage points on average across the batches.
- If the behavior change is broad or risky, require a 10 percentage point gain
  against `all_rule_based` to remain true on the same batches.
- Use `--early-stop 100 --early-stop-regression 0.15` to quickly reject clearly
  worse candidates without waiting for the full 1000-game run.

When a candidate passes the promotion rule:

1. Save it as `strategies/strategic_v<N>.json`.
2. Keep older strategy files unchanged for reproducibility.
3. Add targeted unit tests for the behavior that changed.
4. Run the verification commands above.
5. Commit with `feat(decision): add strategic policy v<N>`.

## Known Results

Representative v2 to v3 comparison on the same deal sets:

```text
random_source=1778580340896195000 (batch 1)
v2: landlord_strategic=0.63, farmers_strategic=0.91
v3: landlord_strategic=0.65, farmers_strategic=0.91

random_source=1778580344473397000 (batch 2)
v2: landlord_strategic=0.72, farmers_strategic=0.88
v3: landlord_strategic=0.77, farmers_strategic=0.89

random_source=1778580349487460000 (batch 3)
v2: landlord_strategic=0.59, farmers_strategic=0.79
v3: landlord_strategic=0.60, farmers_strategic=0.80

v2 averages: landlord=0.647, farmers=0.860
v3 averages: landlord=0.673, farmers=0.867
Delta: landlord +2.7pp, farmers +0.7pp
```

Historical v1 to v2 comparison:

```text
random_source=1778566394333033000
v1: landlord=0.66, farmers=0.82
v2: landlord=0.70, farmers=0.84
```

## Failed Or Reverted Experiments

These were tested and should not be reintroduced without a new reason:

- Broad farmer rule "do not beat ally's play" reduced farmer performance badly.
- Narrow farmer rule "do not beat ally with one card" still reduced farmer
  performance on 100-game checks.
- Leading a pair whenever any opponent has two cards did not improve the
  tournament result.
- Replacing stranded-single risk with a count of all outside higher cards made
  both landlord and farmer placements worse.
- Increasing endgame search from 10 to 12 cards gave no win-rate improvement and
  made evaluation much slower.
- Removing long-lead preference (`--prefer-short-leads`) materially hurt results.
- `remaining_group_penalty` penalizing plays that break pairs/triples: improved
  landlord by ~2pp but hurt farmer by 3-5pp per batch (too conservative).
- `response_strength_weight` adding strength cost when responding: hurt farmer
  by 6-8pp per batch (made farmers too passive, landlord runs unchecked).
- Weight tuning (stranded_risk_weight 2x, opponent_urgency_weight 2x, both 2x):
  all within ±1pp noise, no significant improvement.
- Power cost tuning (pcn=2-4, pct=0): within ±1pp noise. Bomb decisions are
  already correct in most game situations.

## Current Next Step

v3 promoted with algorithmic improvements over v2:
- Opponent card inference from pass history (记牌)
- Enhanced per-opponent threat assessment using pass constraints
- Hand control quality scoring (控场) with `hand_control_weight`
- Farmer cooperation logic (农民配合) with `farmer_cooperation_weight`

v3 improved landlord by 2.7pp on average with no regressions. Farmer side
neutral-to-positive (+0.7pp average).

Suggested directions for future sessions:
- 2-ply minimax: consider opponent responses when evaluating plays
- Probabilistic opponent hand estimation: infer opponent holdings beyond pass constraints
- Adaptive strategy: different scoring weights based on game phase
- Learned evaluation: replace hand-crafted scoring with ML model

Infrastructure improvements available for future tuning:
- `--hand-control-weight` and `--farmer-cooperation-weight` CLI overrides
- `--stranded-risk-weight` and `--opponent-urgency-weight` CLI overrides

Keep rejected candidates out of commits. Commit only promoted strategy versions
and reusable evaluation tooling.
