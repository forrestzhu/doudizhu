use crate::cards::{Card, Rank};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandKind {
    Single,
    Pair,
    Triple,
    TripleWithSingle,
    TripleWithPair,
    Straight,
    SerialPairs,
    Airplane,
    AirplaneWithSingles,
    AirplaneWithPairs,
    FourWithTwoSingles,
    FourWithTwoPairs,
    Bomb,
    Rocket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassifiedHand {
    pub kind: HandKind,
    pub strength: u8,
    pub cards: Vec<Card>,
}

pub trait RuleSet {
    fn classify(&self, cards: &[Card]) -> Option<ClassifiedHand>;

    fn can_play_over(&self, candidate: &ClassifiedHand, previous: Option<&ClassifiedHand>) -> bool {
        let Some(previous) = previous else {
            return true;
        };

        match (candidate.kind, previous.kind) {
            (HandKind::Rocket, HandKind::Rocket) => false,
            (HandKind::Rocket, _) => true,
            (_, HandKind::Rocket) => false,
            (HandKind::Bomb, HandKind::Bomb) => candidate.strength > previous.strength,
            (HandKind::Bomb, _) => true,
            (_, HandKind::Bomb) => false,
            (candidate_kind, previous_kind) if candidate_kind == previous_kind => {
                candidate.cards.len() == previous.cards.len()
                    && candidate.strength > previous.strength
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BasicRules;

impl RuleSet for BasicRules {
    fn classify(&self, cards: &[Card]) -> Option<ClassifiedHand> {
        if has_duplicate_cards(cards) {
            return None;
        }

        let mut sorted = cards.to_vec();
        sorted.sort();

        let counts = rank_counts(&sorted);
        let kind = match sorted.as_slice() {
            [one] => Some((HandKind::Single, one.rank.strength())),
            [a, b] if is_rocket(*a, *b) => Some((HandKind::Rocket, Rank::RedJoker.strength())),
            [a, b] if a.rank == b.rank && !a.rank.is_joker() => {
                Some((HandKind::Pair, a.rank.strength()))
            }
            [a, b, c] if same_non_joker_rank(&[*a, *b, *c]) => {
                Some((HandKind::Triple, a.rank.strength()))
            }
            [a, b, c, d] if same_non_joker_rank(&[*a, *b, *c, *d]) => {
                Some((HandKind::Bomb, a.rank.strength()))
            }
            _ => classify_by_counts(&counts, sorted.len()),
        };

        kind.map(|(kind, strength)| ClassifiedHand {
            kind,
            strength,
            cards: sorted,
        })
    }
}

fn classify_by_counts(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<(HandKind, u8)> {
    if let Some(strength) = classify_straight(counts, card_count) {
        return Some((HandKind::Straight, strength));
    }
    if let Some(strength) = classify_serial_pairs(counts, card_count) {
        return Some((HandKind::SerialPairs, strength));
    }
    if let Some(strength) = classify_airplane(counts, card_count) {
        return Some((HandKind::Airplane, strength));
    }
    if let Some(strength) = classify_triple_with_single(counts, card_count) {
        return Some((HandKind::TripleWithSingle, strength));
    }
    if let Some(strength) = classify_triple_with_pair(counts, card_count) {
        return Some((HandKind::TripleWithPair, strength));
    }
    if let Some(strength) = classify_airplane_with_singles(counts, card_count) {
        return Some((HandKind::AirplaneWithSingles, strength));
    }
    if let Some(strength) = classify_airplane_with_pairs(counts, card_count) {
        return Some((HandKind::AirplaneWithPairs, strength));
    }
    if let Some(strength) = classify_four_with_two_singles(counts, card_count) {
        return Some((HandKind::FourWithTwoSingles, strength));
    }
    if let Some(strength) = classify_four_with_two_pairs(counts, card_count) {
        return Some((HandKind::FourWithTwoPairs, strength));
    }
    None
}

fn classify_straight(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count < 5 || counts.values().any(|count| *count != 1) {
        return None;
    }
    consecutive_strength(counts.keys().copied())
}

fn classify_serial_pairs(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count < 6 || !card_count.is_multiple_of(2) || counts.values().any(|count| *count != 2) {
        return None;
    }
    consecutive_strength(counts.keys().copied())
}

fn classify_airplane(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count < 6 || !card_count.is_multiple_of(3) || counts.values().any(|count| *count != 3) {
        return None;
    }
    consecutive_strength(counts.keys().copied())
}

fn classify_triple_with_single(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count != 4 {
        return None;
    }
    primary_rank_with_counts(counts, &[(3, 1), (1, 1)])
}

fn classify_triple_with_pair(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count != 5 {
        return None;
    }
    primary_rank_with_counts(counts, &[(3, 1), (2, 1)])
}

fn classify_airplane_with_singles(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count < 8 || !card_count.is_multiple_of(4) {
        return None;
    }
    let wing_count = card_count / 4;
    let triple_ranks = ranks_with_count(counts, 3);
    if triple_ranks.len() != wing_count {
        return None;
    }
    if counts.values().filter(|count| **count == 1).count() != wing_count {
        return None;
    }
    if counts.iter().any(|(_, count)| !matches!(*count, 1 | 3)) {
        return None;
    }
    consecutive_strength(triple_ranks)
}

fn classify_airplane_with_pairs(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count < 10 || !card_count.is_multiple_of(5) {
        return None;
    }
    let wing_count = card_count / 5;
    let triple_ranks = ranks_with_count(counts, 3);
    if triple_ranks.len() != wing_count {
        return None;
    }
    if counts.values().filter(|count| **count == 2).count() != wing_count {
        return None;
    }
    if counts.iter().any(|(_, count)| !matches!(*count, 2 | 3)) {
        return None;
    }
    consecutive_strength(triple_ranks)
}

fn classify_four_with_two_singles(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count != 6 {
        return None;
    }
    primary_rank_with_counts(counts, &[(4, 1), (1, 2)])
}

fn classify_four_with_two_pairs(counts: &BTreeMap<Rank, usize>, card_count: usize) -> Option<u8> {
    if card_count != 8 {
        return None;
    }
    primary_rank_with_counts(counts, &[(4, 1), (2, 2)])
}

fn primary_rank_with_counts(
    counts: &BTreeMap<Rank, usize>,
    expected: &[(usize, usize)],
) -> Option<u8> {
    for (count, amount) in expected {
        if counts.values().filter(|value| **value == *count).count() != *amount {
            return None;
        }
    }
    let total_groups: usize = expected.iter().map(|(_, amount)| amount).sum();
    if counts.len() != total_groups {
        return None;
    }
    counts
        .iter()
        .find_map(|(rank, count)| (*count == expected[0].0).then_some(rank.strength()))
}

fn consecutive_strength(ranks: impl IntoIterator<Item = Rank>) -> Option<u8> {
    let ranks: Vec<Rank> = ranks.into_iter().collect();
    if ranks.len() < 2 || ranks.iter().any(|rank| !can_be_in_sequence(*rank)) {
        return None;
    }
    if ranks
        .windows(2)
        .all(|window| window[1].strength() == window[0].strength() + 1)
    {
        ranks.last().map(|rank| rank.strength())
    } else {
        None
    }
}

fn can_be_in_sequence(rank: Rank) -> bool {
    !matches!(rank, Rank::Two | Rank::BlackJoker | Rank::RedJoker)
}

fn ranks_with_count(counts: &BTreeMap<Rank, usize>, target: usize) -> Vec<Rank> {
    counts
        .iter()
        .filter_map(|(rank, count)| (*count == target).then_some(*rank))
        .collect()
}

fn rank_counts(cards: &[Card]) -> BTreeMap<Rank, usize> {
    let mut counts = BTreeMap::new();
    for card in cards {
        *counts.entry(card.rank).or_insert(0) += 1;
    }
    counts
}

fn has_duplicate_cards(cards: &[Card]) -> bool {
    let mut seen = BTreeSet::new();
    cards.iter().any(|card| !seen.insert(*card))
}

fn is_rocket(a: Card, b: Card) -> bool {
    matches!(
        (a.rank, b.rank),
        (Rank::BlackJoker, Rank::RedJoker) | (Rank::RedJoker, Rank::BlackJoker)
    )
}

fn same_non_joker_rank(cards: &[Card]) -> bool {
    cards.first().is_some_and(|first| {
        !first.rank.is_joker() && cards.iter().all(|card| card.rank == first.rank)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Suit;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card::suited(rank, suit)
    }

    #[test]
    fn classifies_basic_hands() {
        let rules = BasicRules;

        assert_eq!(
            rules
                .classify(&[card(Rank::Three, Suit::Clubs)])
                .unwrap()
                .kind,
            HandKind::Single
        );
        assert_eq!(
            rules
                .classify(&[
                    card(Rank::Ace, Suit::Clubs),
                    card(Rank::Ace, Suit::Diamonds),
                ])
                .unwrap()
                .kind,
            HandKind::Pair
        );
        assert_eq!(
            rules
                .classify(&[Card::joker(Rank::BlackJoker), Card::joker(Rank::RedJoker)])
                .unwrap()
                .kind,
            HandKind::Rocket
        );
    }

    #[test]
    fn compares_bombs_and_same_kind_hands() {
        let rules = BasicRules;
        let low_single = rules.classify(&[card(Rank::Three, Suit::Clubs)]).unwrap();
        let high_single = rules.classify(&[card(Rank::Ace, Suit::Clubs)]).unwrap();
        let bomb = rules
            .classify(&[
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Four, Suit::Spades),
            ])
            .unwrap();

        assert!(rules.can_play_over(&high_single, Some(&low_single)));
        assert!(!rules.can_play_over(&low_single, Some(&high_single)));
        assert!(rules.can_play_over(&bomb, Some(&high_single)));
    }

    #[test]
    fn rocket_beats_everything_and_cannot_be_beaten() {
        let rules = BasicRules;
        let rocket = rules
            .classify(&[Card::joker(Rank::BlackJoker), Card::joker(Rank::RedJoker)])
            .unwrap();
        let bomb = rules
            .classify(&[
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Four, Suit::Spades),
            ])
            .unwrap();

        assert!(rules.can_play_over(&rocket, Some(&bomb)));
        assert!(!rules.can_play_over(&bomb, Some(&rocket)));
        assert!(!rules.can_play_over(&rocket, Some(&rocket)));
    }

    #[test]
    fn rejects_unsupported_or_invalid_hands() {
        let rules = BasicRules;

        assert!(rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
            ])
            .is_none());
        assert!(rules
            .classify(&[Card::joker(Rank::BlackJoker), Card::joker(Rank::BlackJoker)])
            .is_none());
        assert!(rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Clubs)
            ])
            .is_none());
    }

    #[test]
    fn classifies_attachment_and_sequence_hands() {
        let rules = BasicRules;

        assert_kind(
            &rules,
            &[
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
                card(Rank::Five, Suit::Hearts),
                card(Rank::Nine, Suit::Clubs),
            ],
            HandKind::TripleWithSingle,
            Rank::Five,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Six, Suit::Clubs),
                card(Rank::Six, Suit::Diamonds),
                card(Rank::Six, Suit::Hearts),
                card(Rank::Jack, Suit::Clubs),
                card(Rank::Jack, Suit::Diamonds),
            ],
            HandKind::TripleWithPair,
            Rank::Six,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
            ],
            HandKind::Straight,
            Rank::Seven,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
            ],
            HandKind::SerialPairs,
            Rank::Five,
        );
    }

    #[test]
    fn classifies_airplanes_and_four_with_two() {
        let rules = BasicRules;

        assert_kind(
            &rules,
            &[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
            ],
            HandKind::Airplane,
            Rank::Four,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Nine, Suit::Clubs),
            ],
            HandKind::AirplaneWithSingles,
            Rank::Four,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Six, Suit::Clubs),
                card(Rank::Six, Suit::Diamonds),
                card(Rank::Six, Suit::Hearts),
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Seven, Suit::Diamonds),
                card(Rank::Seven, Suit::Hearts),
                card(Rank::Jack, Suit::Clubs),
                card(Rank::Jack, Suit::Diamonds),
                card(Rank::Queen, Suit::Clubs),
                card(Rank::Queen, Suit::Diamonds),
            ],
            HandKind::AirplaneWithPairs,
            Rank::Seven,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Eight, Suit::Clubs),
                card(Rank::Eight, Suit::Diamonds),
                card(Rank::Eight, Suit::Hearts),
                card(Rank::Eight, Suit::Spades),
                card(Rank::Three, Suit::Clubs),
                card(Rank::King, Suit::Clubs),
            ],
            HandKind::FourWithTwoSingles,
            Rank::Eight,
        );
        assert_kind(
            &rules,
            &[
                card(Rank::Nine, Suit::Clubs),
                card(Rank::Nine, Suit::Diamonds),
                card(Rank::Nine, Suit::Hearts),
                card(Rank::Nine, Suit::Spades),
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::King, Suit::Clubs),
                card(Rank::King, Suit::Diamonds),
            ],
            HandKind::FourWithTwoPairs,
            Rank::Nine,
        );
    }

    #[test]
    fn rejects_invalid_sequences_and_attachments() {
        let rules = BasicRules;

        assert!(rules
            .classify(&[
                card(Rank::Ten, Suit::Clubs),
                card(Rank::Jack, Suit::Clubs),
                card(Rank::Queen, Suit::Clubs),
                card(Rank::King, Suit::Clubs),
                card(Rank::Ace, Suit::Clubs),
                card(Rank::Two, Suit::Clubs),
            ])
            .is_none());
        assert!(rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
            ])
            .is_none());
        assert!(rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Seven, Suit::Clubs),
            ])
            .is_none());
    }

    #[test]
    fn compares_same_shape_by_primary_strength_and_length() {
        let rules = BasicRules;
        let low_straight = rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
            ])
            .unwrap();
        let high_straight = rules
            .classify(&[
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Five, Suit::Diamonds),
                card(Rank::Six, Suit::Diamonds),
                card(Rank::Seven, Suit::Diamonds),
                card(Rank::Eight, Suit::Diamonds),
            ])
            .unwrap();
        let longer_straight = rules
            .classify(&[
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Five, Suit::Hearts),
                card(Rank::Six, Suit::Hearts),
                card(Rank::Seven, Suit::Hearts),
                card(Rank::Eight, Suit::Hearts),
            ])
            .unwrap();

        assert!(rules.can_play_over(&high_straight, Some(&low_straight)));
        assert!(!rules.can_play_over(&low_straight, Some(&high_straight)));
        assert!(!rules.can_play_over(&longer_straight, Some(&low_straight)));
    }

    #[test]
    fn different_shapes_do_not_compare_without_bomb_or_rocket() {
        let rules = BasicRules;
        let triple_with_single = rules
            .classify(&[
                card(Rank::Eight, Suit::Clubs),
                card(Rank::Eight, Suit::Diamonds),
                card(Rank::Eight, Suit::Hearts),
                card(Rank::Three, Suit::Clubs),
            ])
            .unwrap();
        let triple_with_pair = rules
            .classify(&[
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Seven, Suit::Diamonds),
                card(Rank::Seven, Suit::Hearts),
                card(Rank::Ace, Suit::Clubs),
                card(Rank::Ace, Suit::Diamonds),
            ])
            .unwrap();
        let straight = rules
            .classify(&[
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Five, Suit::Diamonds),
                card(Rank::Six, Suit::Diamonds),
                card(Rank::Seven, Suit::Diamonds),
            ])
            .unwrap();

        assert!(!rules.can_play_over(&triple_with_single, Some(&triple_with_pair)));
        assert!(!rules.can_play_over(&triple_with_pair, Some(&triple_with_single)));
        assert!(!rules.can_play_over(&straight, Some(&triple_with_pair)));
    }

    #[test]
    fn bomb_and_rocket_override_normal_shapes() {
        let rules = BasicRules;
        let airplane = rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
            ])
            .unwrap();
        let bomb = rules
            .classify(&[
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
                card(Rank::Five, Suit::Hearts),
                card(Rank::Five, Suit::Spades),
            ])
            .unwrap();
        let rocket = rules
            .classify(&[Card::joker(Rank::BlackJoker), Card::joker(Rank::RedJoker)])
            .unwrap();

        assert!(rules.can_play_over(&bomb, Some(&airplane)));
        assert!(!rules.can_play_over(&airplane, Some(&bomb)));
        assert!(rules.can_play_over(&rocket, Some(&bomb)));
        assert!(!rules.can_play_over(&bomb, Some(&rocket)));
    }

    #[test]
    fn rejects_smaller_response_for_every_supported_shape() {
        let rules = BasicRules;
        let cases = vec![
            (
                vec![card(Rank::Eight, Suit::Clubs)],
                vec![card(Rank::Seven, Suit::Clubs)],
            ),
            (
                vec![
                    card(Rank::Eight, Suit::Clubs),
                    card(Rank::Eight, Suit::Diamonds),
                ],
                vec![
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Seven, Suit::Diamonds),
                ],
            ),
            (
                vec![
                    card(Rank::Eight, Suit::Clubs),
                    card(Rank::Eight, Suit::Diamonds),
                    card(Rank::Eight, Suit::Hearts),
                ],
                vec![
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Seven, Suit::Diamonds),
                    card(Rank::Seven, Suit::Hearts),
                ],
            ),
            (
                vec![
                    card(Rank::Eight, Suit::Clubs),
                    card(Rank::Eight, Suit::Diamonds),
                    card(Rank::Eight, Suit::Hearts),
                    card(Rank::Three, Suit::Clubs),
                ],
                vec![
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Seven, Suit::Diamonds),
                    card(Rank::Seven, Suit::Hearts),
                    card(Rank::Ace, Suit::Clubs),
                ],
            ),
            (
                vec![
                    card(Rank::Nine, Suit::Clubs),
                    card(Rank::Nine, Suit::Diamonds),
                    card(Rank::Nine, Suit::Hearts),
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::Three, Suit::Diamonds),
                ],
                vec![
                    card(Rank::Eight, Suit::Clubs),
                    card(Rank::Eight, Suit::Diamonds),
                    card(Rank::Eight, Suit::Hearts),
                    card(Rank::King, Suit::Clubs),
                    card(Rank::King, Suit::Diamonds),
                ],
            ),
            (
                vec![
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Five, Suit::Clubs),
                    card(Rank::Six, Suit::Clubs),
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Eight, Suit::Clubs),
                ],
                vec![
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Five, Suit::Diamonds),
                    card(Rank::Six, Suit::Diamonds),
                    card(Rank::Seven, Suit::Diamonds),
                ],
            ),
            (
                vec![
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Five, Suit::Clubs),
                    card(Rank::Five, Suit::Diamonds),
                    card(Rank::Six, Suit::Clubs),
                    card(Rank::Six, Suit::Diamonds),
                ],
                vec![
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::Three, Suit::Diamonds),
                    card(Rank::Four, Suit::Hearts),
                    card(Rank::Four, Suit::Spades),
                    card(Rank::Five, Suit::Hearts),
                    card(Rank::Five, Suit::Spades),
                ],
            ),
            (
                vec![
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Four, Suit::Hearts),
                    card(Rank::Five, Suit::Clubs),
                    card(Rank::Five, Suit::Diamonds),
                    card(Rank::Five, Suit::Hearts),
                ],
                vec![
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::Three, Suit::Diamonds),
                    card(Rank::Three, Suit::Hearts),
                    card(Rank::Four, Suit::Spades),
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                ],
            ),
            (
                vec![
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Four, Suit::Hearts),
                    card(Rank::Five, Suit::Clubs),
                    card(Rank::Five, Suit::Diamonds),
                    card(Rank::Five, Suit::Hearts),
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Eight, Suit::Clubs),
                ],
                vec![
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::Three, Suit::Diamonds),
                    card(Rank::Three, Suit::Hearts),
                    card(Rank::Four, Suit::Spades),
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Nine, Suit::Clubs),
                    card(Rank::Ten, Suit::Clubs),
                ],
            ),
            (
                vec![
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Four, Suit::Hearts),
                    card(Rank::Five, Suit::Clubs),
                    card(Rank::Five, Suit::Diamonds),
                    card(Rank::Five, Suit::Hearts),
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Seven, Suit::Diamonds),
                    card(Rank::Eight, Suit::Clubs),
                    card(Rank::Eight, Suit::Diamonds),
                ],
                vec![
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::Three, Suit::Diamonds),
                    card(Rank::Three, Suit::Hearts),
                    card(Rank::Four, Suit::Spades),
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Nine, Suit::Clubs),
                    card(Rank::Nine, Suit::Diamonds),
                    card(Rank::Ten, Suit::Clubs),
                    card(Rank::Ten, Suit::Diamonds),
                ],
            ),
            (
                vec![
                    card(Rank::Nine, Suit::Clubs),
                    card(Rank::Nine, Suit::Diamonds),
                    card(Rank::Nine, Suit::Hearts),
                    card(Rank::Nine, Suit::Spades),
                    card(Rank::Three, Suit::Clubs),
                    card(Rank::King, Suit::Clubs),
                ],
                vec![
                    card(Rank::Eight, Suit::Clubs),
                    card(Rank::Eight, Suit::Diamonds),
                    card(Rank::Eight, Suit::Hearts),
                    card(Rank::Eight, Suit::Spades),
                    card(Rank::Four, Suit::Clubs),
                    card(Rank::Ace, Suit::Clubs),
                ],
            ),
        ];

        for (higher, lower) in cases {
            let higher = rules.classify(&higher).unwrap();
            let lower = rules.classify(&lower).unwrap();

            assert!(rules.can_play_over(&higher, Some(&lower)));
            assert!(!rules.can_play_over(&lower, Some(&higher)));
        }
    }

    fn assert_kind(rules: &BasicRules, cards: &[Card], kind: HandKind, strength: Rank) {
        let hand = rules.classify(cards).unwrap();

        assert_eq!(hand.kind, kind);
        assert_eq!(hand.strength, strength.strength());
    }
}
