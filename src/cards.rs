use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rank {
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
    Two = 16,
    BlackJoker = 18,
    RedJoker = 19,
}

impl Rank {
    pub fn strength(self) -> u8 {
        self as u8
    }

    pub fn is_joker(self) -> bool {
        matches!(self, Rank::BlackJoker | Rank::RedJoker)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Card {
    pub rank: Rank,
    pub suit: Option<Suit>,
}

impl Card {
    pub const fn suited(rank: Rank, suit: Suit) -> Self {
        Self {
            rank,
            suit: Some(suit),
        }
    }

    pub const fn joker(rank: Rank) -> Self {
        Self { rank, suit: None }
    }

    pub fn standard_deck() -> Vec<Card> {
        let ranks = [
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
        let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
        let mut deck = Vec::with_capacity(54);
        for rank in ranks {
            for suit in suits {
                deck.push(Card::suited(rank, suit));
            }
        }
        deck.push(Card::joker(Rank::BlackJoker));
        deck.push(Card::joker(Rank::RedJoker));
        deck
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rank, self.suit).cmp(&(other.rank, other.suit))
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rank {
            Rank::BlackJoker => write!(f, "BJ"),
            Rank::RedJoker => write!(f, "RJ"),
            rank => {
                let rank = match rank {
                    Rank::Three => "3",
                    Rank::Four => "4",
                    Rank::Five => "5",
                    Rank::Six => "6",
                    Rank::Seven => "7",
                    Rank::Eight => "8",
                    Rank::Nine => "9",
                    Rank::Ten => "10",
                    Rank::Jack => "J",
                    Rank::Queen => "Q",
                    Rank::King => "K",
                    Rank::Ace => "A",
                    Rank::Two => "2",
                    Rank::BlackJoker | Rank::RedJoker => unreachable!(),
                };
                let suit = match self.suit {
                    Some(Suit::Clubs) => "C",
                    Some(Suit::Diamonds) => "D",
                    Some(Suit::Hearts) => "H",
                    Some(Suit::Spades) => "S",
                    None => "?",
                };
                write!(f, "{rank}{suit}")
            }
        }
    }
}

impl FromStr for Card {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim().to_ascii_uppercase();
        match value.as_str() {
            "BJ" => return Ok(Card::joker(Rank::BlackJoker)),
            "RJ" => return Ok(Card::joker(Rank::RedJoker)),
            _ => {}
        }

        if value.len() < 2 {
            return Err(format!("invalid card: {value}"));
        }

        let (rank_text, suit_text) = value.split_at(value.len() - 1);
        let rank = match rank_text {
            "3" => Rank::Three,
            "4" => Rank::Four,
            "5" => Rank::Five,
            "6" => Rank::Six,
            "7" => Rank::Seven,
            "8" => Rank::Eight,
            "9" => Rank::Nine,
            "10" => Rank::Ten,
            "J" => Rank::Jack,
            "Q" => Rank::Queen,
            "K" => Rank::King,
            "A" => Rank::Ace,
            "2" => Rank::Two,
            _ => return Err(format!("invalid rank in card: {value}")),
        };
        let suit = match suit_text {
            "C" => Suit::Clubs,
            "D" => Suit::Diamonds,
            "H" => Suit::Hearts,
            "S" => Suit::Spades,
            _ => return Err(format!("invalid suit in card: {value}")),
        };

        Ok(Card::suited(rank, suit))
    }
}
