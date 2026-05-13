# Strategic Policy v6: Enhanced Rule Engine

## Goal
Improve overall decision quality for both landlord and farmer roles. Measured by all_strategic tournament win rates against v5 baseline (landlord 40%, farmer 60%, avg 35.2 turns).

## Approach
Add 5 independent heuristic rules to the scoring system. Each rule is a penalty/bonus function integrated into `choose_strategic_candidate`. Each rule verified independently via 500-game tournament before merging.

## Rules

### 1. Hand Decomposition Penalty (`decomposition_penalty`)
Evaluate fragmentation of remaining cards after play. Greedy decomposition prioritizing larger combos (airplane > straight > serial_pairs > triple+X > pair > single). Count orphan singles that can't join any combo. Penalize plays that leave fragmented remains.

New config: `decomposition_weight` (default 1).

### 2. Pass Value Assessment (`pass_value`)
When responding to opponent's play, evaluate whether passing is strategically better. Current policy always tries to beat if possible. Add implicit "pass candidate" that competes with actual plays:
- Self hand ≤ 5 cards and can likely regain lead next turn → reward pass
- Opponent played strong card, beating it requires burning big cards → reward pass
- Farmer sender position vs landlord → reward pass (let blocker handle)

New config: `pass_value_weight` (default 2).

### 3. Opponent Strength Inference (`opponent_deduction`)
Enhance OpponentModel with play pattern tracking:
- Track hand kinds played per opponent (single/pair/triple/straight/bomb counts)
- Multiple passes on same hand kind → infer weakness in that kind
- Early bomb usage → infer poor hand structure (desperate to clear)
- Use inference to adjust threat/control scores

Extends existing `OpponentModel` struct, no new config needed.

### 4. Sequence Lookahead (`sequence_planning`)
Two-step lookahead for lead plays. For top candidates, evaluate best next play from remaining cards. If remaining can finish in 1 more play → strong bonus. If remaining fragments badly → penalty. Upgrades `estimated_play_count_cached` with structural awareness.

New config: `sequence_lookahead` (default true).

### 5. Bomb Timing Refinement (`bomb_timing`)
Fine-grained bomb usage scoring beyond phase-aware cost:
- Control retake: opponent has played 2+ consecutive hands and has moderate cards → bonus 5
- Stop finisher: opponent ≤ 2 cards and inferred they can play → bonus 10
- Finisher insurance: already handled by `bomb_finisher_bonus`, unchanged

New config: `bomb_control_bonus` (default 5).

## Config Changes
Add to `StrategicPolicyConfig`:
```json
{
  "decomposition_weight": 1,
  "pass_value_weight": 2,
  "sequence_lookahead": true,
  "bomb_control_bonus": 5
}
```

## Verification
Baseline: `cargo run --release --bin arena -- --random-tournament --games 500 --strategy-file strategies/roles_v1.json`
Each rule: same command, compare all_strategic placement win rates.
Keep rule if win rate improves, revert if it regresses.

## Files Modified
- `src/decision.rs` — all 5 rules + config fields + integration into `choose_strategic_candidate`
- `strategies/roles_v1.json` — add new config fields with default values
