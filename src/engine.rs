use crate::cards::Card;
use crate::decision::{Decision, DecisionPolicy};
use crate::rules::{BasicRules, ClassifiedHand, RuleSet};
use crate::visibility::{PlayerView, Relationship};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnRecord {
    pub player: PlayerId,
    pub decision: Decision,
    pub accepted_hand: Option<ClassifiedHand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deal {
    pub hands: Vec<Vec<Card>>,
    pub bottom_cards: Vec<Card>,
}

impl Deal {
    pub fn from_seed(seed: u64, players: usize) -> Self {
        let mut deck = Card::standard_deck();
        shuffle(&mut deck, seed);
        Self::from_deck(deck, players, 3)
    }

    pub fn from_deck(mut deck: Vec<Card>, players: usize, bottom_count: usize) -> Self {
        assert!(players > 0, "players must be greater than zero");
        assert!(
            deck.len() >= bottom_count + players,
            "deck must contain enough cards"
        );
        let bottom_cards = deck.split_off(deck.len() - bottom_count);
        let mut hands = vec![Vec::new(); players];
        for (index, card) in deck.into_iter().enumerate() {
            hands[index % players].push(card);
        }
        hands[0].extend(bottom_cards.iter().copied());
        for hand in &mut hands {
            hand.sort();
        }
        Self {
            hands,
            bottom_cards,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameConfig {
    pub relationships: Vec<Vec<Relationship>>,
    pub max_turns: usize,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            relationships: landlord_relationships(3),
            max_turns: 1_000,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct GameOutcome {
    pub winner: PlayerId,
    pub turns: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GameError {
    InvalidPlayerCount,
    InvalidRelationshipMatrix,
    TurnLimitExceeded,
    IllegalPass(PlayerId),
    IllegalHand(PlayerId),
    MissingCard(PlayerId, Card),
}

pub struct Game {
    hands: Vec<Vec<Card>>,
    history: Vec<TurnRecord>,
    previous_play: Option<ClassifiedHand>,
    previous_player: Option<PlayerId>,
    current_player: PlayerId,
    passes_since_play: usize,
    config: GameConfig,
    rules: BasicRules,
}

impl Game {
    pub fn new(deal: Deal, config: GameConfig) -> Result<Self, GameError> {
        let players = deal.hands.len();
        if players == 0 {
            return Err(GameError::InvalidPlayerCount);
        }
        if config.relationships.len() != players
            || config.relationships.iter().any(|row| row.len() != players)
        {
            return Err(GameError::InvalidRelationshipMatrix);
        }

        Ok(Self {
            hands: deal.hands,
            history: Vec::new(),
            previous_play: None,
            previous_player: None,
            current_player: PlayerId(0),
            passes_since_play: 0,
            config,
            rules: BasicRules,
        })
    }

    pub fn player_view(&self, player: PlayerId) -> PlayerView {
        PlayerView {
            self_id: player,
            hand: self.hands[player.0].clone(),
            hand_counts: self.hands.iter().map(Vec::len).collect(),
            relationships: self.config.relationships[player.0].clone(),
            history: self.history.clone(),
            previous_play: self.previous_play.clone(),
        }
    }

    pub fn run(
        &mut self,
        policies: &mut [Box<dyn DecisionPolicy>],
    ) -> Result<GameOutcome, GameError> {
        if policies.len() != self.hands.len() {
            return Err(GameError::InvalidPlayerCount);
        }

        while self.history.len() < self.config.max_turns {
            let player = self.current_player;
            let view = self.player_view(player);
            let decision = policies[player.0].decide(&view, &self.rules);
            self.apply_decision(player, decision)?;

            if self.hands[player.0].is_empty() {
                return Ok(GameOutcome {
                    winner: player,
                    turns: self.history.len(),
                });
            }
        }

        Err(GameError::TurnLimitExceeded)
    }

    pub fn history(&self) -> &[TurnRecord] {
        &self.history
    }

    fn apply_decision(&mut self, player: PlayerId, decision: Decision) -> Result<(), GameError> {
        match decision {
            Decision::Pass => {
                if self.previous_play.is_none() {
                    return Err(GameError::IllegalPass(player));
                }
                self.history.push(TurnRecord {
                    player,
                    decision: Decision::Pass,
                    accepted_hand: None,
                });
                self.passes_since_play += 1;
                if self.passes_since_play >= self.hands.len() - 1 {
                    self.previous_play = None;
                    self.passes_since_play = 0;
                    self.current_player = self.previous_player.expect("previous player exists");
                } else {
                    self.advance_turn();
                }
                Ok(())
            }
            Decision::Play(cards) => {
                ensure_cards_available(&self.hands[player.0], &cards, player)?;
                let Some(classified) = self.rules.classify(&cards) else {
                    return Err(GameError::IllegalHand(player));
                };
                if !self
                    .rules
                    .can_play_over(&classified, self.previous_play.as_ref())
                {
                    return Err(GameError::IllegalHand(player));
                }

                remove_cards(&mut self.hands[player.0], &cards);
                self.history.push(TurnRecord {
                    player,
                    decision: Decision::Play(cards),
                    accepted_hand: Some(classified.clone()),
                });
                self.previous_play = Some(classified);
                self.previous_player = Some(player);
                self.passes_since_play = 0;
                self.advance_turn();
                Ok(())
            }
        }
    }

    fn advance_turn(&mut self) {
        self.current_player = PlayerId((self.current_player.0 + 1) % self.hands.len());
    }
}

pub fn landlord_relationships(players: usize) -> Vec<Vec<Relationship>> {
    (0..players)
        .map(|viewer| {
            (0..players)
                .map(|subject| {
                    if viewer == subject {
                        Relationship::SelfPlayer
                    } else if viewer == 0 || subject == 0 {
                        Relationship::Opponent
                    } else {
                        Relationship::Ally
                    }
                })
                .collect()
        })
        .collect()
}

fn remove_cards(hand: &mut Vec<Card>, cards: &[Card]) {
    for card in cards {
        let index = hand
            .iter()
            .position(|candidate| candidate == card)
            .expect("card presence checked before removal");
        hand.remove(index);
    }
}

fn ensure_cards_available(
    hand: &[Card],
    requested: &[Card],
    player: PlayerId,
) -> Result<(), GameError> {
    let mut remaining = hand.to_vec();
    for card in requested {
        let Some(index) = remaining.iter().position(|candidate| candidate == card) else {
            return Err(GameError::MissingCard(player, *card));
        };
        remaining.remove(index);
    }
    Ok(())
}

fn shuffle(deck: &mut [Card], seed: u64) {
    let mut state = seed;
    for i in (1..deck.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state as usize) % (i + 1);
        deck.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Rank, Suit};
    use crate::decision::{Decision, LowestLegalPolicy};
    use crate::rules::RuleSet;
    use crate::visibility::PlayerView;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card::suited(rank, suit)
    }

    struct FixedDecisionPolicy {
        decision: Decision,
    }

    impl DecisionPolicy for FixedDecisionPolicy {
        fn decide(&mut self, _view: &PlayerView, _rules: &dyn RuleSet) -> Decision {
            self.decision.clone()
        }
    }

    fn fixed_policies(first_decision: Decision) -> Vec<Box<dyn DecisionPolicy>> {
        vec![
            Box::new(FixedDecisionPolicy {
                decision: first_decision,
            }),
            Box::<LowestLegalPolicy>::default(),
            Box::<LowestLegalPolicy>::default(),
        ]
    }

    fn fixed_policy(decision: Decision) -> Box<dyn DecisionPolicy> {
        Box::new(FixedDecisionPolicy { decision })
    }

    fn small_game() -> Game {
        let deal = Deal::from_deck(
            vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Eight, Suit::Clubs),
            ],
            3,
            0,
        );
        Game::new(deal, GameConfig::default()).unwrap()
    }

    #[test]
    fn deterministic_deal_has_expected_counts() {
        let deal = Deal::from_seed(7, 3);

        assert_eq!(deal.hands.len(), 3);
        assert_eq!(
            deal.hands.iter().map(Vec::len).collect::<Vec<_>>(),
            [20, 17, 17]
        );
        assert_eq!(deal.bottom_cards.len(), 3);
    }

    #[test]
    fn player_view_exposes_only_own_hand_and_public_state() {
        let deal = Deal::from_deck(
            vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Eight, Suit::Clubs),
            ],
            3,
            0,
        );
        let game = Game::new(deal, GameConfig::default()).unwrap();

        let view = game.player_view(PlayerId(1));

        assert_eq!(view.self_id, PlayerId(1));
        assert_eq!(view.hand_counts, [2, 2, 2]);
        assert_eq!(
            view.hand,
            vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs)
            ]
        );
        assert!(!view.hand.contains(&card(Rank::Three, Suit::Clubs)));
        assert_eq!(view.relationships[0], Relationship::Opponent);
        assert_eq!(view.relationships[2], Relationship::Ally);
    }

    #[test]
    fn harness_can_run_a_seeded_self_play_game() {
        let deal = Deal::from_seed(42, 3);
        let mut game = Game::new(deal, GameConfig::default()).unwrap();
        let mut policies: Vec<Box<dyn DecisionPolicy>> = vec![
            Box::<LowestLegalPolicy>::default(),
            Box::<LowestLegalPolicy>::default(),
            Box::<LowestLegalPolicy>::default(),
        ];

        let outcome = game.run(&mut policies).unwrap();

        assert!(outcome.turns > 0);
        assert!(outcome.turns <= GameConfig::default().max_turns);
        assert!(outcome.winner.0 < 3);
        assert!(!game.history().is_empty());
    }

    #[test]
    fn legal_policy_play_records_history_and_winner() {
        let deal = Deal {
            hands: vec![
                vec![card(Rank::Three, Suit::Clubs)],
                vec![card(Rank::Four, Suit::Clubs)],
                vec![card(Rank::Five, Suit::Clubs)],
            ],
            bottom_cards: Vec::new(),
        };
        let mut game = Game::new(deal, GameConfig::default()).unwrap();
        let mut policies = vec![
            fixed_policy(Decision::Play(vec![card(Rank::Three, Suit::Clubs)])),
            Box::<LowestLegalPolicy>::default(),
            Box::<LowestLegalPolicy>::default(),
        ];

        let outcome = game.run(&mut policies).unwrap();

        assert_eq!(
            outcome,
            GameOutcome {
                winner: PlayerId(0),
                turns: 1
            }
        );
        assert_eq!(game.history().len(), 1);
        assert_eq!(game.history()[0].player, PlayerId(0));
        assert!(game.history()[0].accepted_hand.is_some());
        assert_eq!(game.player_view(PlayerId(0)).hand_counts[0], 0);
    }

    #[test]
    fn leading_player_cannot_pass() {
        let mut game = small_game();
        let mut policies = fixed_policies(Decision::Pass);

        let error = game.run(&mut policies).unwrap_err();

        assert_eq!(error, GameError::IllegalPass(PlayerId(0)));
        assert!(game.history().is_empty());
    }

    #[test]
    fn policy_cannot_play_card_not_in_hand() {
        let mut game = small_game();
        let missing = card(Rank::Ace, Suit::Spades);
        let mut policies = fixed_policies(Decision::Play(vec![missing]));

        let error = game.run(&mut policies).unwrap_err();

        assert_eq!(error, GameError::MissingCard(PlayerId(0), missing));
        assert!(game.history().is_empty());
    }

    #[test]
    fn policy_cannot_reuse_the_same_physical_card() {
        let mut game = small_game();
        let owned = card(Rank::Three, Suit::Clubs);
        let mut policies = fixed_policies(Decision::Play(vec![owned, owned]));

        let error = game.run(&mut policies).unwrap_err();

        assert_eq!(error, GameError::MissingCard(PlayerId(0), owned));
        assert!(game.history().is_empty());
    }

    #[test]
    fn policy_cannot_play_unclassified_hand() {
        let mut game = small_game();
        let mut policies = fixed_policies(Decision::Play(vec![
            card(Rank::Three, Suit::Clubs),
            card(Rank::Six, Suit::Clubs),
        ]));

        let error = game.run(&mut policies).unwrap_err();

        assert_eq!(error, GameError::IllegalHand(PlayerId(0)));
        assert!(game.history().is_empty());
    }

    #[test]
    fn policy_cannot_play_lower_same_kind_over_previous_play() {
        let mut game = small_game();
        let mut policies = vec![
            fixed_policy(Decision::Play(vec![card(Rank::Six, Suit::Clubs)])),
            fixed_policy(Decision::Play(vec![card(Rank::Four, Suit::Clubs)])),
            Box::<LowestLegalPolicy>::default(),
        ];

        let error = game.run(&mut policies).unwrap_err();

        assert_eq!(error, GameError::IllegalHand(PlayerId(1)));
        assert_eq!(game.history().len(), 1);
        assert_eq!(game.history()[0].player, PlayerId(0));
    }
}
