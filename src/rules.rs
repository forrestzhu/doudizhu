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

    #[derive(Debug)]
    struct ComparisonCase {
        name: String,
        candidate: Vec<Card>,
        previous: Vec<Card>,
        expected: bool,
    }

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
    fn rejects_sequences_that_overlap_two_or_jokers() {
        let rules = BasicRules;
        let invalid_cases = [
            straight(&[Rank::Jack, Rank::Queen, Rank::King, Rank::Ace, Rank::Two]),
            straight(&[Rank::King, Rank::Ace, Rank::Two, Rank::Three, Rank::Four]),
            straight(&[Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five]),
            straight(&[
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::BlackJoker,
            ]),
            serial_pairs(&[Rank::Queen, Rank::King, Rank::Ace, Rank::Two]),
            serial_pairs(&[Rank::Ace, Rank::Two, Rank::Three]),
            airplane(&[Rank::Queen, Rank::King, Rank::Ace, Rank::Two]),
            airplane(&[Rank::Ace, Rank::Two]),
        ];

        for cards in invalid_cases {
            assert!(rules.classify(&cards).is_none(), "{cards:?}");
        }

        assert_kind(
            &rules,
            &straight(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
            HandKind::Straight,
            Rank::Ace,
        );
        assert_kind(
            &rules,
            &serial_pairs(&[Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
            HandKind::SerialPairs,
            Rank::Ace,
        );
        assert_kind(
            &rules,
            &airplane(&[Rank::Queen, Rank::King, Rank::Ace]),
            HandKind::Airplane,
            Rank::Ace,
        );
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

    #[test]
    fn comparison_matrix_covers_common_and_corner_cases() {
        let rules = BasicRules;
        let cases = comparison_cases();

        assert!(
            cases.len() >= 100,
            "expected at least 100 comparison cases, got {}",
            cases.len()
        );

        for case in cases {
            let candidate = rules
                .classify(&case.candidate)
                .unwrap_or_else(|| panic!("candidate should classify: {}", case.name));
            let previous = rules
                .classify(&case.previous)
                .unwrap_or_else(|| panic!("previous should classify: {}", case.name));

            assert_eq!(
                rules.can_play_over(&candidate, Some(&previous)),
                case.expected,
                "{}: candidate {:?} over previous {:?}",
                case.name,
                candidate,
                previous
            );
        }
    }

    fn assert_kind(rules: &BasicRules, cards: &[Card], kind: HandKind, strength: Rank) {
        let hand = rules.classify(cards).unwrap();

        assert_eq!(hand.kind, kind);
        assert_eq!(hand.strength, strength.strength());
    }

    fn comparison_cases() -> Vec<ComparisonCase> {
        let mut cases = Vec::new();
        let all_ranks = [
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
            Rank::Two,
            Rank::BlackJoker,
            Rank::RedJoker,
        ];
        let non_joker_ranks = [
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
            Rank::Two,
        ];

        for window in all_ranks.windows(2) {
            cases.push(compare_case(
                format!("{:?} single beats {:?}", window[1], window[0]),
                single(window[1]),
                single(window[0]),
                true,
            ));
            cases.push(compare_case(
                format!("{:?} single cannot beat {:?}", window[0], window[1]),
                single(window[0]),
                single(window[1]),
                false,
            ));
        }

        for window in non_joker_ranks.windows(2) {
            cases.push(compare_case(
                format!("{:?} pair beats {:?}", window[1], window[0]),
                pair(window[1]),
                pair(window[0]),
                true,
            ));
            cases.push(compare_case(
                format!("{:?} pair cannot beat {:?}", window[0], window[1]),
                pair(window[0]),
                pair(window[1]),
                false,
            ));
            cases.push(compare_case(
                format!("{:?} triple beats {:?}", window[1], window[0]),
                triple(window[1]),
                triple(window[0]),
                true,
            ));
            cases.push(compare_case(
                format!("{:?} triple cannot beat {:?}", window[0], window[1]),
                triple(window[0]),
                triple(window[1]),
                false,
            ));
            cases.push(compare_case(
                format!("{:?} bomb beats {:?} bomb", window[1], window[0]),
                bomb(window[1]),
                bomb(window[0]),
                true,
            ));
            cases.push(compare_case(
                format!("{:?} bomb cannot beat {:?} bomb", window[0], window[1]),
                bomb(window[0]),
                bomb(window[1]),
                false,
            ));
        }

        cases.extend([
            compare_case("rocket beats highest bomb", rocket(), bomb(Rank::Two), true),
            compare_case(
                "highest bomb cannot beat rocket",
                bomb(Rank::Two),
                rocket(),
                false,
            ),
            compare_case("rocket cannot beat rocket", rocket(), rocket(), false),
            compare_case(
                "bomb beats straight",
                bomb(Rank::Three),
                straight(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
                true,
            ),
            compare_case(
                "straight cannot beat bomb",
                straight(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
                bomb(Rank::Three),
                false,
            ),
            compare_case(
                "bomb beats airplane with pairs",
                bomb(Rank::Four),
                airplane_with_pairs(&[Rank::Six, Rank::Seven], &[Rank::Jack, Rank::Queen]),
                true,
            ),
            compare_case(
                "rocket beats airplane with singles",
                rocket(),
                airplane_with_singles(&[Rank::Six, Rank::Seven], &[Rank::Jack, Rank::Queen]),
                true,
            ),
            compare_case(
                "pair cannot beat single",
                pair(Rank::Ace),
                single(Rank::King),
                false,
            ),
            compare_case(
                "single cannot beat pair",
                single(Rank::Two),
                pair(Rank::Three),
                false,
            ),
            compare_case(
                "triple cannot beat pair",
                triple(Rank::Ace),
                pair(Rank::Two),
                false,
            ),
            compare_case(
                "triple with single cannot beat triple with pair",
                triple_with_single(Rank::Ace, Rank::Three),
                triple_with_pair(Rank::King, Rank::Four),
                false,
            ),
            compare_case(
                "straight cannot beat serial pairs",
                straight(&[Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King]),
                serial_pairs(&[Rank::Three, Rank::Four, Rank::Five]),
                false,
            ),
        ]);

        cases.extend([
            compare_case(
                "triple with pair compares triple rank not pair rank same triple",
                triple_with_pair(Rank::Five, Rank::Queen),
                triple_with_pair(Rank::Five, Rank::Jack),
                false,
            ),
            compare_case(
                "triple with pair higher triple beats higher carried pair",
                triple_with_pair(Rank::Six, Rank::Three),
                triple_with_pair(Rank::Five, Rank::Ace),
                true,
            ),
            compare_case(
                "triple with pair lower triple cannot beat lower carried pair",
                triple_with_pair(Rank::Five, Rank::Ace),
                triple_with_pair(Rank::Six, Rank::Three),
                false,
            ),
            compare_case(
                "triple with single same triple ignores larger kicker",
                triple_with_single(Rank::Seven, Rank::Ace),
                triple_with_single(Rank::Seven, Rank::Three),
                false,
            ),
            compare_case(
                "triple with single higher triple beats larger kicker",
                triple_with_single(Rank::Eight, Rank::Three),
                triple_with_single(Rank::Seven, Rank::Ace),
                true,
            ),
            compare_case(
                "four with two singles compares quad rank",
                four_with_two_singles(Rank::Nine, &[Rank::Three, Rank::Four]),
                four_with_two_singles(Rank::Eight, &[Rank::Ace, Rank::Two]),
                true,
            ),
            compare_case(
                "four with two singles ignores kickers at equal quad",
                four_with_two_singles(Rank::Nine, &[Rank::Ace, Rank::Two]),
                four_with_two_singles(Rank::Nine, &[Rank::Three, Rank::Four]),
                false,
            ),
            compare_case(
                "four with two pairs compares quad rank",
                four_with_two_pairs(Rank::Ten, &[Rank::Three, Rank::Four]),
                four_with_two_pairs(Rank::Nine, &[Rank::King, Rank::Ace]),
                true,
            ),
            compare_case(
                "four with two pairs ignores carried pairs at equal quad",
                four_with_two_pairs(Rank::Ten, &[Rank::King, Rank::Ace]),
                four_with_two_pairs(Rank::Ten, &[Rank::Three, Rank::Four]),
                false,
            ),
            compare_case(
                "four with two singles cannot beat four with two pairs",
                four_with_two_singles(Rank::Two, &[Rank::Three, Rank::Four]),
                four_with_two_pairs(Rank::Three, &[Rank::Four, Rank::Five]),
                false,
            ),
        ]);

        cases.extend([
            compare_case(
                "straight same length compares top rank",
                straight(&[Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight]),
                straight(&[Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven]),
                true,
            ),
            compare_case(
                "straight lower top cannot beat higher top",
                straight(&[Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven]),
                straight(&[Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight]),
                false,
            ),
            compare_case(
                "straight different length cannot compare upward",
                straight(&[
                    Rank::Three,
                    Rank::Four,
                    Rank::Five,
                    Rank::Six,
                    Rank::Seven,
                    Rank::Eight,
                ]),
                straight(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
                false,
            ),
            compare_case(
                "straight different length cannot compare downward",
                straight(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
                straight(&[
                    Rank::Three,
                    Rank::Four,
                    Rank::Five,
                    Rank::Six,
                    Rank::Seven,
                    Rank::Eight,
                ]),
                false,
            ),
            compare_case(
                "ace-high straight beats king-high straight",
                straight(&[Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]),
                straight(&[Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King]),
                true,
            ),
            compare_case(
                "serial pairs same length compare top rank",
                serial_pairs(&[Rank::Four, Rank::Five, Rank::Six]),
                serial_pairs(&[Rank::Three, Rank::Four, Rank::Five]),
                true,
            ),
            compare_case(
                "serial pairs different length cannot compare",
                serial_pairs(&[Rank::Three, Rank::Four, Rank::Five, Rank::Six]),
                serial_pairs(&[Rank::Jack, Rank::Queen, Rank::King]),
                false,
            ),
            compare_case(
                "airplane same length compares highest triple",
                airplane(&[Rank::Four, Rank::Five]),
                airplane(&[Rank::Three, Rank::Four]),
                true,
            ),
            compare_case(
                "airplane lower highest triple cannot beat",
                airplane(&[Rank::Three, Rank::Four]),
                airplane(&[Rank::Four, Rank::Five]),
                false,
            ),
            compare_case(
                "airplane different length cannot compare",
                airplane(&[Rank::Three, Rank::Four, Rank::Five]),
                airplane(&[Rank::Jack, Rank::Queen]),
                false,
            ),
        ]);

        cases.extend([
            compare_case(
                "airplane with singles compares triple sequence",
                airplane_with_singles(&[Rank::Five, Rank::Six], &[Rank::Three, Rank::Four]),
                airplane_with_singles(&[Rank::Three, Rank::Four], &[Rank::Ace, Rank::Two]),
                true,
            ),
            compare_case(
                "airplane with singles ignores larger wings",
                airplane_with_singles(&[Rank::Three, Rank::Four], &[Rank::Ace, Rank::Two]),
                airplane_with_singles(&[Rank::Five, Rank::Six], &[Rank::Seven, Rank::Eight]),
                false,
            ),
            compare_case(
                "airplane with singles same triple sequence ignores wings",
                airplane_with_singles(&[Rank::Five, Rank::Six], &[Rank::Ace, Rank::Two]),
                airplane_with_singles(&[Rank::Five, Rank::Six], &[Rank::Three, Rank::Four]),
                false,
            ),
            compare_case(
                "airplane with pairs compares triple sequence",
                airplane_with_pairs(&[Rank::Five, Rank::Six], &[Rank::Three, Rank::Four]),
                airplane_with_pairs(&[Rank::Three, Rank::Four], &[Rank::Ace, Rank::Two]),
                true,
            ),
            compare_case(
                "airplane with pairs ignores larger wing pairs",
                airplane_with_pairs(&[Rank::Three, Rank::Four], &[Rank::Ace, Rank::Two]),
                airplane_with_pairs(&[Rank::Five, Rank::Six], &[Rank::Seven, Rank::Eight]),
                false,
            ),
            compare_case(
                "airplane with singles cannot beat airplane with pairs",
                airplane_with_singles(&[Rank::King, Rank::Ace], &[Rank::Three, Rank::Four]),
                airplane_with_pairs(&[Rank::Three, Rank::Four], &[Rank::Five, Rank::Six]),
                false,
            ),
            compare_case(
                "airplane with pairs cannot beat airplane with singles",
                airplane_with_pairs(&[Rank::King, Rank::Ace], &[Rank::Three, Rank::Four]),
                airplane_with_singles(&[Rank::Three, Rank::Four], &[Rank::Five, Rank::Six]),
                false,
            ),
        ]);

        cases
    }

    fn compare_case(
        name: impl Into<String>,
        candidate: Vec<Card>,
        previous: Vec<Card>,
        expected: bool,
    ) -> ComparisonCase {
        ComparisonCase {
            name: name.into(),
            candidate,
            previous,
            expected,
        }
    }

    fn single(rank: Rank) -> Vec<Card> {
        match rank {
            Rank::BlackJoker | Rank::RedJoker => vec![Card::joker(rank)],
            _ => vec![card(rank, Suit::Clubs)],
        }
    }

    fn pair(rank: Rank) -> Vec<Card> {
        vec![card(rank, Suit::Clubs), card(rank, Suit::Diamonds)]
    }

    fn triple(rank: Rank) -> Vec<Card> {
        vec![
            card(rank, Suit::Clubs),
            card(rank, Suit::Diamonds),
            card(rank, Suit::Hearts),
        ]
    }

    fn bomb(rank: Rank) -> Vec<Card> {
        vec![
            card(rank, Suit::Clubs),
            card(rank, Suit::Diamonds),
            card(rank, Suit::Hearts),
            card(rank, Suit::Spades),
        ]
    }

    fn rocket() -> Vec<Card> {
        vec![Card::joker(Rank::BlackJoker), Card::joker(Rank::RedJoker)]
    }

    fn triple_with_single(triple_rank: Rank, single_rank: Rank) -> Vec<Card> {
        let mut cards = triple(triple_rank);
        cards.push(card(single_rank, Suit::Spades));
        cards
    }

    fn triple_with_pair(triple_rank: Rank, pair_rank: Rank) -> Vec<Card> {
        let mut cards = triple(triple_rank);
        cards.extend(pair(pair_rank));
        cards
    }

    fn straight(ranks: &[Rank]) -> Vec<Card> {
        ranks
            .iter()
            .enumerate()
            .map(|(index, rank)| match rank {
                Rank::BlackJoker | Rank::RedJoker => Card::joker(*rank),
                _ => card(*rank, suit_for_index(index)),
            })
            .collect()
    }

    fn serial_pairs(ranks: &[Rank]) -> Vec<Card> {
        ranks.iter().flat_map(|rank| pair(*rank)).collect()
    }

    fn airplane(ranks: &[Rank]) -> Vec<Card> {
        ranks.iter().flat_map(|rank| triple(*rank)).collect()
    }

    fn airplane_with_singles(triple_ranks: &[Rank], wing_ranks: &[Rank]) -> Vec<Card> {
        let mut cards = airplane(triple_ranks);
        for (index, rank) in wing_ranks.iter().enumerate() {
            cards.push(card(*rank, suit_for_index(index)));
        }
        cards
    }

    fn airplane_with_pairs(triple_ranks: &[Rank], pair_ranks: &[Rank]) -> Vec<Card> {
        let mut cards = airplane(triple_ranks);
        for rank in pair_ranks {
            cards.extend(pair(*rank));
        }
        cards
    }

    fn four_with_two_singles(quad_rank: Rank, single_ranks: &[Rank; 2]) -> Vec<Card> {
        let mut cards = bomb(quad_rank);
        for (index, rank) in single_ranks.iter().enumerate() {
            cards.push(card(*rank, suit_for_index(index)));
        }
        cards
    }

    fn four_with_two_pairs(quad_rank: Rank, pair_ranks: &[Rank; 2]) -> Vec<Card> {
        let mut cards = bomb(quad_rank);
        for rank in pair_ranks {
            cards.extend(pair(*rank));
        }
        cards
    }

    fn suit_for_index(index: usize) -> Suit {
        [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades][index % 4]
    }
}
