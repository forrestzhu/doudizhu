# Data Model: Core Arena And Pluggable Decisions

## Card

- `rank`: ordered Dou Dizhu rank, with jokers above 2.
- `suit`: one of four suits for normal cards; absent for jokers.

## ClassifiedHand

- `kind`: single, pair, triple, bomb, or rocket in the initial slice.
- `strength`: comparable rank strength for same-kind comparisons.
- `cards`: normalized card list used by the engine and history.

## Decision

- `Pass`: decline to play when responding to a previous play.
- `Play(cards)`: candidate cards the engine must classify and validate.

## PlayerView

- `self_id`: requesting player.
- `hand`: only that player's cards.
- `hand_counts`: visible count for each player.
- `relationships`: self, ally, or opponent from the viewer's perspective.
- `history`: public turn records.
- `previous_play`: current play that must be beaten, if any.

## Game

- Owns all hidden hands and public history.
- Tracks current player, previous accepted play, previous player, and pass count.
- Produces `GameOutcome` when a hand is empty.
- Produces `GameError` for illegal decisions or turn limit exhaustion.

## DecisionPolicy

- Receives `PlayerView` and active `RuleSet`.
- Returns exactly one `Decision`.
- Must not mutate game state directly.
