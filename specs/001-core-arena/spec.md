# Feature Specification: Core Arena And Pluggable Decisions

**Feature Branch**: `001-core-arena`  
**Created**: 2026-05-11  
**Status**: Draft  
**Input**: User wants a Dou Dizhu system architecture, Spec Kit workflow, a deterministic arena, pluggable player decision systems, rules, dealing, visibility, and future Electron UI support.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run Deterministic Self Play (Priority: P1)

As a developer, I can run a arena with fixed seeds so that rule, visibility, and
decision changes can be tested without a UI.

**Why this priority**: It gives the project a real verification loop before Electron
or advanced AI players exist.

**Independent Test**: Run the CLI arena with a fixed seed and assert that a game
finishes or returns a structured engine error.

**Acceptance Scenarios**:

1. **Given** a fixed seed, **When** the arena runs one game, **Then** it reports the winner and turn count.
2. **Given** multiple fixed seeds, **When** the arena runs multiple games, **Then** it reports aggregate win counts.

---

### User Story 2 - Enforce Player Visibility (Priority: P1)

As a decision policy, I receive only my legal game view so that humans, bots, and
LLM players share the same hidden-information boundary.

**Why this priority**: Decision plugins are only interchangeable if their inputs
are stable and fair.

**Independent Test**: Construct a known deal and assert that each player view
contains own cards, public counts, public history, and no opponent cards.

**Acceptance Scenarios**:

1. **Given** a known deal, **When** player 1 asks for a view, **Then** the view includes player 1's hand and hand counts for all players.
2. **Given** the same view, **When** inspecting hidden state, **Then** player 0 and player 2 hands are not exposed.

---

### User Story 3 - Plug In Decision Policies (Priority: P2)

As a developer, I can replace player decision logic without changing the engine so
that human, rule-based, search, and LLM players can compete through one contract.

**Why this priority**: Pluggability is the central architecture requirement.

**Independent Test**: Run the engine with three policies implementing the same
decision interface and verify the engine validates their outputs.

**Acceptance Scenarios**:

1. **Given** a policy that returns a legal hand, **When** the engine applies it, **Then** the cards are removed and history is recorded.
2. **Given** a policy that returns an illegal pass or hand, **When** the engine applies it, **Then** the engine returns a structured error.

### Edge Cases

- The leading player must not pass when there is no previous play.
- A player must not play cards that are not in their hand.
- A candidate hand must be rejected when the active rule set cannot classify it.
- A game must stop with a structured error if the turn limit is exceeded.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST model cards, ranks, and a standard 54-card deck.
- **FR-002**: System MUST support deterministic deals from a seed.
- **FR-003**: System MUST expose a rule interface that classifies candidate hands and compares them to the current previous play.
- **FR-004**: System MUST expose a player view that includes own hand, hand counts, relationships, public history, and previous play.
- **FR-005**: System MUST NOT expose hidden opponent cards in the default player view.
- **FR-006**: System MUST define a decision interface whose output is either pass or play a candidate hand.
- **FR-007**: System MUST validate decisions before mutating game state.
- **FR-008**: System MUST provide a CLI arena for repeated seeded self-play.
- **FR-009**: System MUST provide automated tests for deal counts, basic rules, visibility, and seeded self-play.

### Key Entities *(include if feature involves data)*

- **Card**: A rank plus optional suit; jokers have no suit.
- **ClassifiedHand**: A legal hand type, strength, and cards.
- **PlayerView**: The legal state visible to one player at decision time.
- **DecisionPolicy**: A replaceable player strategy implementation.
- **TurnRecord**: A public record of a pass or accepted play.
- **Game**: Mutable turn state and rule enforcement.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo test` verifies core rule, deal, visibility, and arena behavior.
- **SC-002**: `cargo run --bin arena -- --games 10 --seed 42` completes without UI dependencies.
- **SC-003**: A new policy can be added by implementing one trait without modifying game state code.
- **SC-004**: Player views never contain opponent hands in default three-player mode.

## Assumptions

- Initial implementation uses a reduced rule set to prove the architecture and arena.
- Full Dou Dizhu rule coverage will be added incrementally behind the same rule interface.
- Electron integration is out of scope for the first arena slice.
- Landlord bidding is not modeled yet; player 0 is treated as landlord and receives the bottom cards.
