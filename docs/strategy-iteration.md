# Strategy Iteration Runbook

This document records the current policy-search loop so a future session can
continue after context is cleared.

## Current Champion

Use `strategies/strategic_v2.json` as the current best strategic policy.

```json
{
  "avoid_power_hands": true,
  "endgame_search_limit": 10,
  "power_cost_normal": 4,
  "power_cost_threat": 1,
  "lead_longer_tiebreak": true,
  "lead_tempo_plan_weight": 1
}
```

`strategies/strategic_v1.json` is retained for historical comparison. Its
`lead_tempo_plan_weight` is `4`.

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

Fresh random 100-game tournament:

```sh
cargo run --quiet --bin arena -- \
  --random-tournament \
  --games 100 \
  --strategy-file strategies/strategic_v2.json \
  --format json
```

Reproduce a specific random tournament:

```sh
cargo run --quiet --bin arena -- \
  --random-tournament \
  --games 100 \
  --random-source <RANDOM_SOURCE> \
  --strategy-file strategies/strategic_v2.json \
  --format json
```

Quickly test parameter overrides without editing a strategy file:

```sh
cargo run --quiet --bin arena -- \
  --random-tournament \
  --games 100 \
  --random-source <RANDOM_SOURCE> \
  --strategy-file strategies/strategic_v2.json \
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

- Run at least three independent 100-game random-source batches.
- Do not accept a role-side regression greater than 2 percentage points on any
  batch.
- Prefer candidates that improve either landlord or farmer placement by at least
  2 percentage points on average across the batches.
- If the behavior change is broad or risky, require a 10 percentage point gain
  against `all_rule_based` to remain true on the same batches.

When a candidate passes the promotion rule:

1. Save it as `strategies/strategic_v<N>.json`.
2. Keep older strategy files unchanged for reproducibility.
3. Add targeted unit tests for the behavior that changed.
4. Run the verification commands above.
5. Commit with `feat(decision): add strategic policy v<N>`.

## Known Results

Representative v1 to v2 comparison on the same deal set:

```text
random_source=1778566394333033000

v1:
all_rule_based      wins=[29, 43, 28], landlord=0.29, farmers=0.71
landlord_strategic  wins=[66, 14, 20], landlord=0.66
farmers_strategic   wins=[18, 51, 31], farmers=0.82

v2:
all_rule_based      wins=[29, 43, 28], landlord=0.29, farmers=0.71
landlord_strategic  wins=[70, 13, 17], landlord=0.70
farmers_strategic   wins=[16, 54, 30], farmers=0.84
```

Additional v2 checks:

```text
random_source=1778568404891843000
landlord_strategic landlord=0.72
farmers_strategic farmers=0.80

random_source=1778567919819516000
landlord_strategic landlord=0.66
farmers_strategic farmers=0.83
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

Continue from `strategic_v2.json`. Weight tuning and scoring penalties tested in
this session were within ±1pp noise. The v2 strategy appears near a local optimum
for the current heuristic scoring approach.

Suggested directions for future sessions:

- 2-ply minimax: consider opponent responses when evaluating plays
- Opponent card modeling: track revealed cards across history for better outside_cards
- Adaptive strategy: different scoring weights based on game phase (early/mid/late)
- Learned evaluation: replace hand-crafted scoring with ML model

Infrastructure improvements available for future tuning:
- `--stranded-risk-weight` and `--opponent-urgency-weight` CLI overrides
- Improved greedy endgame planner (remaining group quality tiebreaker)

Keep rejected candidates out of commits. Commit only promoted strategy versions
and reusable evaluation tooling.
