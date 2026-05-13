# Strategy Iteration Runbook

This document records the current policy-search loop so a future session can
continue after context is cleared.

## Current Champion

Use `strategies/roles_v1.json` as the current best role-specific strategy.

The v1 role config with v5 algorithmic improvements plus role-specific tuning:
- Landlord: `stranded_risk_weight: 0` (don't penalize stranded singles)
- Sender/Blocker: same as v5 (stranded_risk_weight: 1)
- Architecture: `RoleStrategyConfig` with separate landlord/sender/blocker configs

```json
{
  "landlord": { "stranded_risk_weight": 0, ... },
  "sender":   { "stranded_risk_weight": 1, ... },
  "blocker":  { "stranded_risk_weight": 1, ... }
}
```

`strategies/strategic_v3.json` is retained for historical comparison (uniform config).

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
  --strategy-file strategies/roles_v1.json \
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
  --strategy-file strategies/roles_v1.json \
  --format json
```

Reproduce a specific random tournament:

```sh
cargo run --release --quiet --bin arena -- \
  --random-tournament \
  --games 1000 \
  --threads 12 \
  --random-source <RANDOM_SOURCE> \
  --strategy-file strategies/roles_v1.json \
  --format json
```

Quickly test parameter overrides without editing a strategy file
(overrides apply to all roles):

```sh
cargo run --release --quiet --bin arena -- \
  --random-tournament \
  --games 1000 \
  --threads 12 \
  --random-source <RANDOM_SOURCE> \
  --strategy-file strategies/roles_v1.json \
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

Representative v3 to v4 (algorithmic) comparison on the same deal sets:

```text
random_source=1778580340896195000 (batch 1)
v3: landlord_strategic=0.71, farmers_strategic=0.83
v4: landlord_strategic=0.76, farmers_strategic=0.86

random_source=1778580344473397000 (batch 2)
v3: landlord_strategic=0.73, farmers_strategic=0.85
v4: landlord_strategic=0.76, farmers_strategic=0.85

random_source=1778580349487460000 (batch 3)
v3: landlord_strategic=0.70, farmers_strategic=0.82
v4: landlord_strategic=0.75, farmers_strategic=0.85

v3 averages: landlord=0.713, farmers=0.833
v4 averages: landlord=0.757, farmers=0.853
Delta: landlord +4.4pp, farmers +2.0pp
```

v4 verified on 3 additional random batches (no regressions):

```text
random_source=1778610141259568000: landlord=0.74, farmers=0.85
random_source=1778610168311022000: landlord=0.75, farmers=0.86
random_source=1778610196406412000: landlord=0.78, farmers=0.85
```

v4 to v5 comparison (MC endgame + farmer position differentiation):

```text
random_source=1778580340896195000 (batch 1)
v4: landlord_strategic=0.76, farmers_strategic=0.86
v5: landlord_strategic=0.76, farmers_strategic=0.90

random_source=1778580344473397000 (batch 2)
v4: landlord_strategic=0.76, farmers_strategic=0.85
v5: landlord_strategic=0.77, farmers_strategic=0.88

random_source=1778580349487460000 (batch 3)
v4: landlord_strategic=0.75, farmers_strategic=0.85
v5: landlord_strategic=0.75, farmers_strategic=0.88

v4 averages: landlord=0.757, farmers=0.853
v5 averages: landlord=0.760, farmers=0.887
Delta: landlord +0.3pp, farmers +3.4pp
```

v5 verified on 3 additional random batches:

```text
random_source=1778633467521542000: landlord=0.77, farmers=0.87
random_source=1778633500000000000: landlord=0.76, farmers=0.88
random_source=1778633530000000000: landlord=0.75, farmers=0.88
```

Historical v2 to v3 comparison:

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
- Strategic pass (步长最短策略) with `after_plan >= current_plan`: massive regression
  (landlord -19pp, farmers -28pp). AI passed too often, giving opponents free turns.
- Conservative strategic pass with `after_plan > current_plan`: still regressed
  (farmers -3pp).
- 顶牌 (blocking high): always responding with highest cards against opponents
  hurt both roles (landlord -7pp, farmers -7pp). Wastes strong cards early.
- Increased threat values (25/15 vs 5/3): no measurable effect.
- Extended threat assessment for 3-4 card opponents: landlord regressed -2.6pp
  (too cautious against opponents with moderate hands).
- 3x card-length preference: same as 2x within noise, 2x is the sweet spot.
- 2x shape priority bonus: slightly worse than 1x for landlord (-1pp).

v4→v5 iteration attempts (all failed to meet 2pp promotion threshold):

- `combo_break_cost` penalizing response plays that break triples/quads: no
  measurable effect. Existing `stranded_single_risk` and `plan_turns` already
  handle combo preservation through the greedy tiebreaker fix.
- Unbeatable lead bonus (`lead_unbeatable_bonus`): massive regression when
  added inside `lead_tempo_plan_weight` multiplication due to operator
  precedence bug (`a * b.saturating_sub(c)` = `a * (b-c)`). Fixed version
  with small values (3-5) still regressed: AI fixated on unbeatable plays,
  ignoring overall plan quality. Binary unbeatable check is too coarse.
- Response strength tiebreaker (moving `hand.strength` earlier in scoring
  tuple): within ±1pp noise. Exact tempo_score ties are rare enough that
  this doesn't matter.
- Straight potential in `remaining_control_quality` (bonus for consecutive
  ranks ≥5): slight regression (-1pp landlord). AI over-valued straight-forming
  remaining hands, avoiding plays that break useful sequences.
- 3-card opponent threat (threat=1 or 2): no measurable effect. 3-card
  opponents with relevant pass constraints are too rare for this to matter.
- Beam search greedy (width 3, first step only): no measurable effect, ~1.5x
  slower. The greedy's biggest-combo-first heuristic is already near-optimal
  for plan estimation.
- Proportional lead response risk (`lead_beatable_count`): massive regression
  (-23pp landlord). Even divided by 5, adding beatable count inside the
  `lead_tempo_plan_weight` parenthesis overwhelmed other scoring terms.
  Penalizing beatable leads makes the AI avoid ALL small cards.
- Reduced cards.len() weight in greedy (`cards.len() / 2`): farmer regressed
  -1pp on one batch. The greedy's bias toward biggest combo is correct.
- Reduced hand_control for farmers (×2/3): no additional effect beyond the
  opponent-ratio adjustment.

Marginal improvements retained (below 2pp promotion threshold):

- Opponent-ratio adjusted `stranded_single_risk`: scales stranded risk by
  `opponent_cards / total_other_cards`. For farmers, this reduces risk by
  ~33% (ally's cards aren't threats). +0.7pp farmers, 0pp landlord across
  6 batches. Below promotion threshold but no regression.
- MC endgame simulation alone (without farmer position): 30 samples,
  threshold ≤15 total cards. +1pp farmers, +0.3pp landlord across 6 batches.
  Within noise but no regression. Retained as foundation for future improvements.

Uniform farmer position penalty (constant 5 or 12 points regardless of hand
strength): no measurable effect. All candidates receive the same offset,
so relative ordering is unchanged. Only the strength-based version
(proportional to `hand.strength`) produced meaningful differentiation.

v5→roles_v1 iteration attempts:

- Role-specific `farmer_cooperation_weight` (sender=5, blocker=1): +1pp farmers
  on one batch, within noise. The algorithmic position differentiation in
  `farmer_cooperation_penalty` already captures this at the code level.
- Blocker lower power cost (pcn=2, pct=0): no measurable effect.
- Sender higher hand_control (4) + higher stranded_risk (2): no measurable effect.
- Landlord `lead_tempo_plan_weight=2`: within ±1pp noise.
- Landlord `opponent_urgency_weight=2`: within ±1pp noise.
- Landlord `hand_control_weight=1`: within ±1pp noise.
- `opponent_weakness_bonus` (exploit pass constraints when leading): massive
  regression (-4pp landlord). The bonus overrode plan quality, making the AI
  lead suboptimal hands just because opponents had pass constraints for that type.
  Pass constraints aren't reliable enough for active exploitation.
- Sender `stranded_risk_weight=0`, blocker `stranded_risk_weight=2`: mixed results,
  one batch regressed farmers -2pp. Blocker needs some stranded risk awareness.
- All roles `stranded_risk_weight=0`: landlord +6pp, farmers neutral. Identical
  to landlord-only change for both placements.

roles_v1 algorithmic iteration attempts (all within ±2pp noise):

- `response_overkill`: penalize response plays that use much more strength
  than needed (strength delta / 3). ~+1pp average, within noise. Retained as
  harmless nudge toward efficient responses.
- `ally_finish_assist`: when leading and ally has 1-2 cards, bonus for leading
  Single (ally=1) or Pair (ally=2) with strength ≤ 10. ~+0.7pp farmers, within
  noise. Retained as harmless cooperation nudge.
- Near-finish aggression (reduce control/stranded/threat when plan_turns ≤ 2):
  zero additional effect beyond response_overkill. Reverted.
- Inferred unbeatable bonus (use pass constraints to detect plays no opponent
  can beat): no improvement, slight farmer regression (-1pp on one batch).
  Reverted.
- MC simulation ally cooperation (simulated farmers pass on ally's plays):
  no improvement, slight farmer regression (-1pp on one batch). Reverted.
- Ally-aware `remaining_control_quality` (only count opponent-reachable cards
  as threats): changed too many decisions (5 test failures), too risky. Reverted.

## Current State

roles_v1 with minor algorithmic improvements (response_overkill, ally_finish_assist):

```text
random_source=1778649390821997000: landlord=0.83, farmers=0.89
random_source=1778649419376375000: landlord=0.84, farmers=0.89
random_source=1778649447480388000: landlord=0.83, farmers=0.88
```

Consistent with roles_v1 baseline: L=0.82-0.84, F=0.88-0.90. The rule-based
strategic policy has reached a local optimum — parameter tuning and incremental
algorithmic changes all land within ±1-2pp noise.

Further improvement requires fundamentally different approaches:
- 2-ply minimax: consider opponent responses when evaluating plays
- Probabilistic opponent hand estimation: infer opponent holdings beyond pass constraints
- Better MC simulation: pass real relationships, use strategic policy in simulation
- Learned evaluation: replace hand-crafted scoring with neural network (DouZero-style)

Keep rejected candidates out of commits. Commit only promoted strategy versions
and reusable evaluation tooling.
