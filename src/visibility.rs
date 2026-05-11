use crate::cards::Card;
use crate::engine::{PlayerId, TurnRecord};
use crate::rules::ClassifiedHand;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relationship {
    SelfPlayer,
    Ally,
    Opponent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerView {
    pub self_id: PlayerId,
    pub hand: Vec<Card>,
    pub hand_counts: Vec<usize>,
    pub relationships: Vec<Relationship>,
    pub history: Vec<TurnRecord>,
    pub previous_play: Option<ClassifiedHand>,
}
