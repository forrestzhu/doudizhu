use crate::cards::{Card, Rank};
use crate::rules::{ClassifiedHand, RuleSet};
use crate::visibility::PlayerView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Play(Vec<Card>),
}

pub trait DecisionPolicy {
    fn decide(&mut self, view: &PlayerView, rules: &dyn RuleSet) -> Decision;
}

#[derive(Debug, Default)]
pub struct LowestLegalPolicy;

impl DecisionPolicy for LowestLegalPolicy {
    fn decide(&mut self, view: &PlayerView, rules: &dyn RuleSet) -> Decision {
        RuleBasedPolicy::default().decide(view, rules)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleBasedPolicyConfig {
    pub avoid_power_hands: bool,
}

impl Default for RuleBasedPolicyConfig {
    fn default() -> Self {
        Self {
            avoid_power_hands: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct RuleBasedPolicy {
    config: RuleBasedPolicyConfig,
}

impl RuleBasedPolicy {
    pub fn new(config: RuleBasedPolicyConfig) -> Self {
        Self { config }
    }
}

impl DecisionPolicy for RuleBasedPolicy {
    fn decide(&mut self, view: &PlayerView, rules: &dyn RuleSet) -> Decision {
        let candidates = legal_candidates(&view.hand, view.previous_play.as_ref(), rules);
        choose_candidate(candidates, self.config)
            .map(|hand| Decision::Play(hand.cards))
            .unwrap_or(Decision::Pass)
    }
}

pub fn legal_candidates(
    hand: &[Card],
    previous: Option<&ClassifiedHand>,
    rules: &dyn RuleSet,
) -> Vec<ClassifiedHand> {
    let mut sorted = hand.to_vec();
    sorted.sort();

    let mut candidates = Vec::new();
    for combo in simple_combinations(&sorted) {
        if let Some(classified) = rules.classify(&combo) {
            if rules.can_play_over(&classified, previous) {
                candidates.push(classified);
            }
        }
    }
    candidates
}

fn simple_combinations(hand: &[Card]) -> Vec<Vec<Card>> {
    let mut combos = Vec::new();

    for card in hand {
        combos.push(vec![*card]);
    }

    for rank in distinct_ranks(hand) {
        let cards: Vec<Card> = hand
            .iter()
            .copied()
            .filter(|card| card.rank == rank)
            .collect();
        if cards.len() >= 2 && !rank.is_joker() {
            combos.push(cards[..2].to_vec());
        }
        if cards.len() >= 3 && !rank.is_joker() {
            combos.push(cards[..3].to_vec());
        }
        if cards.len() == 4 && !rank.is_joker() {
            combos.push(cards);
        }
    }

    let black_joker = hand
        .iter()
        .copied()
        .find(|card| card.rank == Rank::BlackJoker);
    let red_joker = hand
        .iter()
        .copied()
        .find(|card| card.rank == Rank::RedJoker);
    if let (Some(black_joker), Some(red_joker)) = (black_joker, red_joker) {
        combos.push(vec![black_joker, red_joker]);
    }

    combos
}

fn distinct_ranks(hand: &[Card]) -> Vec<Rank> {
    let mut ranks: Vec<Rank> = hand.iter().map(|card| card.rank).collect();
    ranks.sort();
    ranks.dedup();
    ranks
}

fn is_power_hand(hand: &ClassifiedHand) -> bool {
    matches!(
        hand.kind,
        crate::rules::HandKind::Bomb | crate::rules::HandKind::Rocket
    )
}

fn choose_candidate(
    mut candidates: Vec<ClassifiedHand>,
    config: RuleBasedPolicyConfig,
) -> Option<ClassifiedHand> {
    if config.avoid_power_hands && candidates.iter().any(|hand| !is_power_hand(hand)) {
        candidates.retain(|hand| !is_power_hand(hand));
    }

    candidates.sort_by_key(|hand| {
        (
            config.avoid_power_hands == is_power_hand(hand),
            hand.cards.len(),
            hand.strength,
            hand.cards
                .iter()
                .map(|card| card.rank.strength())
                .sum::<u8>(),
        )
    });

    candidates.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Suit;
    use crate::rules::{BasicRules, RuleSet};

    fn card(rank: Rank, suit: Suit) -> Card {
        Card::suited(rank, suit)
    }

    #[test]
    fn rule_based_default_matches_lowest_legal_policy() {
        let rules = BasicRules;
        let previous_play = rules.classify(&[
            card(Rank::Seven, Suit::Clubs),
            card(Rank::Seven, Suit::Diamonds),
        ]);
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Four, Suit::Spades),
                card(Rank::Eight, Suit::Clubs),
                card(Rank::Eight, Suit::Diamonds),
            ],
            hand_counts: vec![6, 2, 2],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play,
        };
        let mut lowest = LowestLegalPolicy;
        let mut rule_based = RuleBasedPolicy::default();

        assert_eq!(
            lowest.decide(&view, &rules),
            rule_based.decide(&view, &rules)
        );
    }

    #[test]
    fn rule_based_avoids_power_hand_when_normal_play_can_beat() {
        let rules = BasicRules;
        let previous_play = rules.classify(&[card(Rank::Seven, Suit::Clubs)]);
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Four, Suit::Spades),
                card(Rank::Eight, Suit::Clubs),
            ],
            hand_counts: vec![5, 2, 2],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play,
        };
        let mut policy = RuleBasedPolicy::new(RuleBasedPolicyConfig {
            avoid_power_hands: true,
        });

        assert_eq!(
            policy.decide(&view, &rules),
            Decision::Play(vec![card(Rank::Eight, Suit::Clubs)])
        );
    }

    #[test]
    fn rule_based_can_be_configured_to_spend_power_hand() {
        let rules = BasicRules;
        let previous_play = rules.classify(&[card(Rank::Seven, Suit::Clubs)]);
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Four, Suit::Spades),
                card(Rank::Eight, Suit::Clubs),
            ],
            hand_counts: vec![5, 2, 2],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play,
        };
        let mut policy = RuleBasedPolicy::new(RuleBasedPolicyConfig {
            avoid_power_hands: false,
        });

        assert_eq!(
            policy.decide(&view, &rules),
            Decision::Play(vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Four, Suit::Spades),
            ])
        );
    }
}
