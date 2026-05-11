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
        let mut candidates = legal_candidates(&view.hand, view.previous_play.as_ref(), rules);
        candidates.sort_by_key(|hand| {
            (
                is_power_hand(hand),
                hand.cards.len(),
                hand.strength,
                hand.cards
                    .iter()
                    .map(|card| card.rank.strength())
                    .sum::<u8>(),
            )
        });

        candidates
            .into_iter()
            .next()
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
