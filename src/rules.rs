use crate::cards::{Card, Rank};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandKind {
    Single,
    Pair,
    Triple,
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
        let mut sorted = cards.to_vec();
        sorted.sort();

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
            _ => None,
        };

        kind.map(|(kind, strength)| ClassifiedHand {
            kind,
            strength,
            cards: sorted,
        })
    }
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
    }
}
