# Doudizhu Architecture

## Goal

Build the game as a testable core engine first, then attach Tauri as one possible
player interface. The core must support deterministic harness runs and pluggable
decision systems before the UI exists.

## Layers

1. `core` Rust library
   - cards, ranks, deals, turn state
   - rule classification and comparison
   - visibility projection per player
   - decision input/output contract
   - deterministic self-play harness

2. `adapters`
   - Tauri commands translate UI events into core actions.
   - CLI harness runs seeded simulations and regression scenarios.
   - Future LLM/search adapters implement the same decision trait.

3. `specs`
   - Spec Kit artifacts describe user-visible behavior and implementation slices.
   - Architecture decisions that change contracts should be reflected in specs and
     this document.

## Core Contracts

### Decision Input

A player decision receives `PlayerView`:

- `self_id`
- own `hand`
- visible `hand_counts`
- public `history`
- current `previous_play`
- relationship map from the viewer to each player

The view intentionally omits opponent hands. Two-player perfect-information modes
can be modeled later as a separate visibility policy rather than by weakening the
default contract.

### Decision Output

A decision returns exactly one of:

- `Pass`
- `Play(Vec<Card>)`

The engine validates the output against the active rule set and current state.
Invalid decisions are engine errors, not implicit corrections.

### Rule System

`RuleSet` owns hand classification and comparison. The initial `BasicRules`
supports only single, pair, triple, bomb, and rocket so the harness can run now.
Full Dou Dizhu rule expansion should add straights, consecutive pairs, three-with,
airplanes, and rule variants behind the same trait.

The initial deal model treats player 0 as landlord and immediately adds the
three bottom cards to player 0's hand. The bottom cards are still retained on the
`Deal` for future UI reveal/history behavior.

## Harness Strategy

The harness must answer two questions after every development slice:

- Can the engine execute a deterministic game without hidden UI state?
- Can tests assert specific rule, visibility, and decision-contract behavior?
- Can scenario files catch regressions in rules, visibility, and policy outputs?

Current commands:

```sh
cargo test
cargo run --bin harness -- --games 10 --seed 42
cargo run --bin harness -- --scenario evals/scenarios/bomb_beats_pair.json --format json
```

## Open Design Questions

- How should landlord bidding be modeled: as a pre-game phase or a separate game
  mode selected before dealing?
- Should bottom cards become visible to all players after landlord assignment in
  v1?
- Do we want separate rule profiles for learning modes, two-player perfect
  information, full Dou Dizhu, and custom reduced decks?
- What format should an LLM decision adapter use: plain JSON contract, tool call,
  or local strategy process over stdin/stdout?
