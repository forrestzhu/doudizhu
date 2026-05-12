use crate::cards::{Card, Rank};
use crate::rules::{ClassifiedHand, HandKind, RuleSet};
use crate::visibility::{PlayerView, Relationship};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Debug, Default)]
pub struct StrategicPolicy {
    config: StrategicPolicyConfig,
}

impl StrategicPolicy {
    pub fn new(config: RuleBasedPolicyConfig) -> Self {
        Self {
            config: StrategicPolicyConfig {
                avoid_power_hands: config.avoid_power_hands,
                ..StrategicPolicyConfig::default()
            },
        }
    }

    pub fn from_config(config: StrategicPolicyConfig) -> Self {
        Self { config }
    }
}

impl DecisionPolicy for StrategicPolicy {
    fn decide(&mut self, view: &PlayerView, rules: &dyn RuleSet) -> Decision {
        let candidates = legal_candidates(&view.hand, view.previous_play.as_ref(), rules);
        choose_strategic_candidate(candidates, view, rules, self.config)
            .map(|hand| Decision::Play(hand.cards))
            .unwrap_or(Decision::Pass)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicPolicyConfig {
    pub avoid_power_hands: bool,
    pub endgame_search_limit: usize,
    pub power_cost_normal: usize,
    pub power_cost_threat: usize,
    pub lead_longer_tiebreak: bool,
    #[serde(default = "default_lead_tempo_plan_weight")]
    pub lead_tempo_plan_weight: usize,
    #[serde(default = "default_one")]
    pub stranded_risk_weight: usize,
    #[serde(default = "default_one")]
    pub opponent_urgency_weight: usize,
    #[serde(default = "default_two")]
    pub hand_control_weight: usize,
    #[serde(default = "default_three")]
    pub farmer_cooperation_weight: usize,
}

impl Default for StrategicPolicyConfig {
    fn default() -> Self {
        Self {
            avoid_power_hands: true,
            endgame_search_limit: 10,
            power_cost_normal: 4,
            power_cost_threat: 1,
            lead_longer_tiebreak: true,
            lead_tempo_plan_weight: default_lead_tempo_plan_weight(),
            stranded_risk_weight: default_one(),
            opponent_urgency_weight: default_one(),
            hand_control_weight: default_two(),
            farmer_cooperation_weight: default_three(),
        }
    }
}

fn default_lead_tempo_plan_weight() -> usize {
    1
}

fn default_one() -> usize {
    1
}

fn default_two() -> usize {
    2
}

fn default_three() -> usize {
    3
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
    let groups = grouped_by_rank(hand);

    for card in hand {
        combos.push(vec![*card]);
    }

    for (rank, cards) in &groups {
        if cards.len() >= 2 && !rank.is_joker() {
            combos.push(cards[..2].to_vec());
        }
        if cards.len() >= 3 && !rank.is_joker() {
            combos.push(cards[..3].to_vec());
        }
        if cards.len() == 4 && !rank.is_joker() {
            combos.push(cards.clone());
        }
    }

    add_attachment_combinations(&mut combos, &groups);
    add_sequence_combinations(&mut combos, &groups);

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

fn add_attachment_combinations(combos: &mut Vec<Vec<Card>>, groups: &BTreeMap<Rank, Vec<Card>>) {
    let pairs = ranks_with_at_least(groups, 2);
    let triples = ranks_with_at_least(groups, 3);
    let quads = ranks_with_exactly(groups, 4);
    let all_cards = groups
        .values()
        .flat_map(|cards| cards.iter().copied())
        .collect::<Vec<_>>();

    for triple_rank in &triples {
        let triple = groups[triple_rank][..3].to_vec();
        for single in all_cards
            .iter()
            .copied()
            .filter(|card| card.rank != *triple_rank)
        {
            let mut combo = triple.clone();
            combo.push(single);
            combos.push(combo);
        }
        for pair_rank in pairs.iter().filter(|rank| *rank != triple_rank) {
            let mut combo = triple.clone();
            combo.extend(groups[pair_rank][..2].iter().copied());
            combos.push(combo);
        }
    }

    for quad_rank in &quads {
        let quad = groups[quad_rank].clone();
        let kickers = all_cards
            .iter()
            .copied()
            .filter(|card| card.rank != *quad_rank)
            .collect::<Vec<_>>();
        for singles in card_combinations(&kickers, 2) {
            let mut combo = quad.clone();
            combo.extend(singles);
            combos.push(combo);
        }
        let pair_ranks = pairs
            .iter()
            .copied()
            .filter(|rank| rank != quad_rank)
            .collect::<Vec<_>>();
        for pair_combo in rank_combinations(&pair_ranks, 2) {
            let mut combo = quad.clone();
            for rank in pair_combo {
                combo.extend(groups[&rank][..2].iter().copied());
            }
            combos.push(combo);
        }
    }
}

fn add_sequence_combinations(combos: &mut Vec<Vec<Card>>, groups: &BTreeMap<Rank, Vec<Card>>) {
    let single_ranks = sequence_ranks(groups, 1);
    for window in consecutive_windows(&single_ranks, 5) {
        let combo = window
            .iter()
            .map(|rank| groups[rank][0])
            .collect::<Vec<_>>();
        combos.push(combo);
    }

    let pair_ranks = sequence_ranks(groups, 2);
    for window in consecutive_windows(&pair_ranks, 3) {
        let combo = window
            .iter()
            .flat_map(|rank| groups[rank][..2].iter().copied())
            .collect::<Vec<_>>();
        combos.push(combo);
    }

    let triple_ranks = sequence_ranks(groups, 3);
    for window in consecutive_windows(&triple_ranks, 2) {
        let triple_cards = window
            .iter()
            .flat_map(|rank| groups[rank][..3].iter().copied())
            .collect::<Vec<_>>();
        combos.push(triple_cards.clone());

        let outside_cards = groups
            .iter()
            .filter(|(rank, _)| !window.contains(rank))
            .flat_map(|(_, cards)| cards.iter().copied())
            .collect::<Vec<_>>();
        for wings in card_combinations(&outside_cards, window.len()) {
            let mut combo = triple_cards.clone();
            combo.extend(wings);
            combos.push(combo);
        }

        let outside_pair_ranks = groups
            .iter()
            .filter_map(|(rank, cards)| {
                (!window.contains(rank) && cards.len() >= 2 && !rank.is_joker()).then_some(*rank)
            })
            .collect::<Vec<_>>();
        for pair_combo in rank_combinations(&outside_pair_ranks, window.len()) {
            let mut combo = triple_cards.clone();
            for rank in pair_combo {
                combo.extend(groups[&rank][..2].iter().copied());
            }
            combos.push(combo);
        }
    }
}

fn grouped_by_rank(hand: &[Card]) -> BTreeMap<Rank, Vec<Card>> {
    let mut groups: BTreeMap<Rank, Vec<Card>> = BTreeMap::new();
    for card in hand {
        groups.entry(card.rank).or_default().push(*card);
    }
    for cards in groups.values_mut() {
        cards.sort();
    }
    groups
}

fn ranks_with_at_least(groups: &BTreeMap<Rank, Vec<Card>>, count: usize) -> Vec<Rank> {
    groups
        .iter()
        .filter_map(|(rank, cards)| (cards.len() >= count && !rank.is_joker()).then_some(*rank))
        .collect()
}

fn ranks_with_exactly(groups: &BTreeMap<Rank, Vec<Card>>, count: usize) -> Vec<Rank> {
    groups
        .iter()
        .filter_map(|(rank, cards)| (cards.len() == count && !rank.is_joker()).then_some(*rank))
        .collect()
}

fn sequence_ranks(groups: &BTreeMap<Rank, Vec<Card>>, count: usize) -> Vec<Rank> {
    groups
        .iter()
        .filter_map(|(rank, cards)| {
            (cards.len() >= count && can_be_in_sequence(*rank)).then_some(*rank)
        })
        .collect()
}

fn consecutive_windows(ranks: &[Rank], min_len: usize) -> Vec<Vec<Rank>> {
    let mut windows = Vec::new();
    for start in 0..ranks.len() {
        for end in start + min_len..=ranks.len() {
            let candidate = &ranks[start..end];
            if candidate
                .windows(2)
                .all(|window| window[1].strength() == window[0].strength() + 1)
            {
                windows.push(candidate.to_vec());
            } else {
                break;
            }
        }
    }
    windows
}

fn card_combinations(cards: &[Card], count: usize) -> Vec<Vec<Card>> {
    combinations(cards, count)
}

fn rank_combinations(ranks: &[Rank], count: usize) -> Vec<Vec<Rank>> {
    combinations(ranks, count)
}

fn combinations<T: Copy>(items: &[T], count: usize) -> Vec<Vec<T>> {
    if count > items.len() {
        return Vec::new();
    }

    let mut combinations = Vec::new();
    let mut current = Vec::with_capacity(count);
    collect_combinations(items, count, 0, &mut current, &mut combinations);
    combinations
}

fn collect_combinations<T: Copy>(
    items: &[T],
    count: usize,
    start: usize,
    current: &mut Vec<T>,
    combinations: &mut Vec<Vec<T>>,
) {
    if current.len() == count {
        combinations.push(current.clone());
        return;
    }

    let needed = count - current.len();
    for index in start..=items.len() - needed {
        current.push(items[index]);
        collect_combinations(items, count, index + 1, current, combinations);
        current.pop();
    }
}

fn can_be_in_sequence(rank: Rank) -> bool {
    !matches!(rank, Rank::Two | Rank::BlackJoker | Rank::RedJoker)
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

fn remaining_control_quality(remaining: &[Card], outside: &[Card]) -> usize {
    let groups = grouped_by_rank(remaining);
    let outside_groups = grouped_by_rank(outside);
    let mut quality = 0;

    for (rank, cards) in &groups {
        let strength = rank.strength();
        match cards.len() {
            4 => quality += 5,
            3 => quality += 2,
            2 => {
                let has_higher = outside_groups
                    .iter()
                    .any(|(r, c)| c.len() >= 2 && r.strength() > strength);
                if !has_higher {
                    quality += 2;
                }
            }
            1 => {
                let has_higher = outside.iter().any(|c| c.rank.strength() > strength);
                if !has_higher {
                    quality += 1;
                }
            }
            _ => {}
        }
    }

    quality
}

fn ally_id(view: &PlayerView) -> Option<usize> {
    for (player_id, rel) in view.relationships.iter().enumerate() {
        if *rel == Relationship::Ally {
            return Some(player_id);
        }
    }
    None
}

fn last_play_player_id(view: &PlayerView) -> Option<usize> {
    for record in view.history.iter().rev() {
        if let Decision::Play(_) = &record.decision {
            return Some(record.player.0);
        }
    }
    None
}

fn farmer_cooperation_penalty(hand: &ClassifiedHand, view: &PlayerView) -> usize {
    let ally = match ally_id(view) {
        Some(id) => id,
        None => return 0,
    };

    let landlord_id = (0..view.hand_counts.len())
        .find(|&p| is_opponent(view, p))
        .unwrap_or(0);
    let landlord_cards = view
        .hand_counts
        .get(landlord_id)
        .copied()
        .unwrap_or(usize::MAX);
    let ally_cards = view.hand_counts.get(ally).copied().unwrap_or(usize::MAX);

    let mut penalty: usize = 0;

    if view.previous_play.is_some() {
        if let Some(last_player) = last_play_player_id(view) {
            if last_player == ally && ally_cards <= 3 {
                penalty += 50;
            }
            if last_player == landlord_id && landlord_cards <= 2 {
                penalty = penalty.saturating_sub(20);
            }
        }
    }

    if landlord_cards == 1 && is_power_hand(hand) {
        penalty = penalty.saturating_sub(30);
    }

    penalty
}

fn choose_strategic_candidate(
    mut candidates: Vec<ClassifiedHand>,
    view: &PlayerView,
    rules: &dyn RuleSet,
    config: StrategicPolicyConfig,
) -> Option<ClassifiedHand> {
    if candidates.is_empty() {
        return None;
    }
    let outside = outside_cards(view);
    let models = build_opponent_models(view);
    let mut plan_cache = BTreeMap::new();
    candidates.sort_by_key(|hand| {
        let remaining = remaining_after(&view.hand, &hand.cards);
        let winning = !remaining.is_empty();
        let plan_turns = estimated_play_count_cached(&remaining, rules, &mut plan_cache, config);
        let control = enhanced_threat_control_risk(hand, &models, &outside, view);
        let stranded = stranded_single_risk(&remaining, &outside) * config.stranded_risk_weight;
        let threat = enhanced_opponent_threat_risk(hand, &remaining, &models, view)
            * config.opponent_urgency_weight;
        let hand_control = 50_usize.saturating_sub(remaining_control_quality(&remaining, &outside))
            * config.hand_control_weight;
        let power_cost = strategic_power_cost(hand, &remaining, view, config);
        let coop = farmer_cooperation_penalty(hand, view) * config.farmer_cooperation_weight;
        let tempo_score = if view.previous_play.is_none() {
            (plan_turns + control + stranded + threat + power_cost + coop)
                * config.lead_tempo_plan_weight
                + view
                    .hand
                    .len()
                    .saturating_sub(hand.cards.len())
                    // Bonus for harder-to-beat hand kinds when leading
                    .saturating_sub(shape_priority(hand.kind) as usize)
        } else {
            plan_turns + control + stranded + threat + power_cost + coop
        };
        let length_tiebreak = if config.lead_longer_tiebreak && view.previous_play.is_none() {
            usize::MAX - hand.cards.len()
        } else {
            hand.cards.len()
        };
        (
            winning,
            tempo_score,
            control,
            stranded,
            hand_control,
            power_cost,
            config.avoid_power_hands == is_power_hand(hand),
            length_tiebreak,
            hand.strength,
            hand.cards
                .iter()
                .map(|card| card.rank.strength())
                .sum::<u8>(),
        )
    });

    candidates.into_iter().next()
}

fn remaining_after(hand: &[Card], played: &[Card]) -> Vec<Card> {
    let mut remaining = hand.to_vec();
    for card in played {
        if let Some(index) = remaining.iter().position(|candidate| candidate == card) {
            remaining.remove(index);
        }
    }
    remaining.sort();
    remaining
}

fn estimated_play_count_cached(
    hand: &[Card],
    rules: &dyn RuleSet,
    cache: &mut BTreeMap<String, usize>,
    config: StrategicPolicyConfig,
) -> usize {
    if hand.len() > config.endgame_search_limit {
        return estimated_play_count_greedy(hand, rules);
    }
    if hand.is_empty() {
        return 0;
    }

    let key = cards_cache_key(hand);
    if let Some(cached) = cache.get(&key) {
        return *cached;
    }

    let candidates = legal_candidates(hand, None, rules);
    let best = candidates
        .into_iter()
        .map(|candidate| {
            1 + estimated_play_count_cached(
                &remaining_after(hand, &candidate.cards),
                rules,
                cache,
                config,
            )
        })
        .min()
        .unwrap_or(hand.len());

    cache.insert(key, best);
    best
}

fn cards_cache_key(cards: &[Card]) -> String {
    let mut sorted = cards.to_vec();
    sorted.sort();
    sorted
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn estimated_play_count_greedy(hand: &[Card], rules: &dyn RuleSet) -> usize {
    if hand.is_empty() {
        return 0;
    }

    let mut remaining = hand.to_vec();
    let mut turns = 0;

    while !remaining.is_empty() {
        let candidates = legal_candidates(&remaining, None, rules);
        let Some(best) = candidates.into_iter().max_by_key(|hand| {
            plan_candidate_value(hand, &remaining_after(&remaining, &hand.cards))
        }) else {
            return turns + remaining.len();
        };
        remaining = remaining_after(&remaining, &best.cards);
        turns += 1;
    }

    turns
}

fn plan_candidate_value(
    hand: &ClassifiedHand,
    remaining: &[Card],
) -> (usize, u8, usize, u8, usize) {
    let remaining_groups = grouped_by_rank(remaining)
        .values()
        .map(|cards| match cards.len() {
            4 => 4,
            3 => 3,
            2 => 2,
            _ => 0,
        })
        .sum::<usize>();
    (
        hand.cards.len(),
        shape_priority(hand.kind),
        usize::from(is_power_hand(hand)),
        hand.strength,
        remaining_groups,
    )
}

fn shape_priority(kind: HandKind) -> u8 {
    match kind {
        HandKind::Rocket => 13,
        HandKind::Bomb => 12,
        HandKind::AirplaneWithPairs => 11,
        HandKind::AirplaneWithSingles => 10,
        HandKind::Airplane => 9,
        HandKind::FourWithTwoPairs => 8,
        HandKind::FourWithTwoSingles => 7,
        HandKind::Straight => 6,
        HandKind::SerialPairs => 5,
        HandKind::TripleWithPair => 4,
        HandKind::TripleWithSingle => 3,
        HandKind::Triple => 2,
        HandKind::Pair => 1,
        HandKind::Single => 0,
    }
}

fn stranded_single_risk(hand: &[Card], outside: &[Card]) -> usize {
    let groups = grouped_by_rank(hand);
    let outside_max = outside
        .iter()
        .map(|card| card.rank.strength())
        .max()
        .unwrap_or(0);

    groups
        .iter()
        .filter(|(rank, cards)| {
            cards.len() == 1 && rank.strength() < outside_max && !rank.is_joker()
        })
        .map(|(rank, _)| {
            if rank.strength() <= Rank::Ten.strength() {
                3
            } else {
                1
            }
        })
        .sum()
}

fn enhanced_threat_control_risk(
    hand: &ClassifiedHand,
    models: &[Option<OpponentModel>],
    outside: &[Card],
    view: &PlayerView,
) -> usize {
    let mut total_threat = 0;
    for (player_id, model_opt) in models.iter().enumerate() {
        if !is_opponent(view, player_id) {
            continue;
        }
        let model = match model_opt {
            Some(m) => m,
            None => continue,
        };
        let opp_cards = view.hand_counts[player_id];
        match (opp_cards, hand.kind) {
            (1, HandKind::Single) => {
                if opponent_can_beat_normal(model, hand) {
                    total_threat += outside
                        .iter()
                        .filter(|card| card.rank.strength() > hand.strength)
                        .count();
                }
            }
            (2, HandKind::Pair) => {
                if opponent_can_beat_normal(model, hand) {
                    let groups = grouped_by_rank(outside);
                    total_threat += groups
                        .iter()
                        .filter(|(rank, cards)| cards.len() >= 2 && rank.strength() > hand.strength)
                        .count();
                }
            }
            _ => {}
        }
    }
    total_threat
}

fn strategic_power_cost(
    hand: &ClassifiedHand,
    remaining: &[Card],
    view: &PlayerView,
    config: StrategicPolicyConfig,
) -> usize {
    if !config.avoid_power_hands || !is_power_hand(hand) || remaining.is_empty() {
        return 0;
    }

    let shortest_opponent = view
        .hand_counts
        .iter()
        .enumerate()
        .filter(|(player, _)| is_opponent(view, *player))
        .map(|(_, count)| *count)
        .min()
        .unwrap_or(usize::MAX);

    if shortest_opponent <= 2 {
        config.power_cost_threat
    } else {
        config.power_cost_normal
    }
}

fn enhanced_opponent_threat_risk(
    hand: &ClassifiedHand,
    remaining: &[Card],
    models: &[Option<OpponentModel>],
    view: &PlayerView,
) -> usize {
    if remaining.is_empty() {
        return 0;
    }

    let mut max_threat = 0;
    for (player_id, model_opt) in models.iter().enumerate() {
        if !is_opponent(view, player_id) {
            continue;
        }
        let model = match model_opt {
            Some(m) => m,
            None => continue,
        };
        let opp_cards = view.hand_counts[player_id];
        let threat = match (opp_cards, hand.kind) {
            (1, HandKind::Single) => {
                if opponent_can_beat_normal(model, hand) {
                    5
                } else {
                    0
                }
            }
            (2, HandKind::Pair) => {
                if opponent_can_beat_normal(model, hand) {
                    3
                } else {
                    0
                }
            }
            _ => 0,
        };
        max_threat = max_threat.max(threat);
    }
    max_threat
}

fn is_opponent(view: &PlayerView, player: usize) -> bool {
    match view.relationships.get(player) {
        Some(rel) => *rel == Relationship::Opponent,
        None => {
            if view.self_id.0 == 0 {
                player != 0
            } else {
                player == 0
            }
        }
    }
}

#[derive(Clone)]
struct OpponentModel {
    played_cards: BTreeSet<Card>,
    pass_constraints: Vec<PassConstraint>,
    #[allow(dead_code)]
    unknown_count: usize,
}

#[derive(Clone)]
struct PassConstraint {
    hand_kind: HandKind,
    strength: u8,
}

fn build_opponent_models(view: &PlayerView) -> Vec<Option<OpponentModel>> {
    let player_count = view.hand_counts.len();
    let mut models: Vec<Option<OpponentModel>> = vec![None; player_count];

    for (player_id, slot) in models.iter_mut().enumerate() {
        if player_id == view.self_id.0 {
            continue;
        }
        *slot = Some(OpponentModel {
            played_cards: BTreeSet::new(),
            pass_constraints: Vec::new(),
            unknown_count: view.hand_counts[player_id],
        });
    }

    let mut current_play_to_beat: Option<&ClassifiedHand> = None;

    for record in &view.history {
        let pid = record.player.0;
        if pid >= player_count {
            continue;
        }

        match &record.decision {
            Decision::Play(cards) => {
                if let Some(ref mut model) = models[pid] {
                    for card in cards {
                        model.played_cards.insert(*card);
                    }
                }
                if let Some(ref hand) = record.accepted_hand {
                    current_play_to_beat = Some(hand);
                }
            }
            Decision::Pass => {
                if let Some(to_beat) = current_play_to_beat {
                    if let Some(ref mut model) = models[pid] {
                        model.pass_constraints.push(PassConstraint {
                            hand_kind: to_beat.kind,
                            strength: to_beat.strength,
                        });
                    }
                }
            }
        }
    }

    models
}

fn opponent_can_beat_normal(model: &OpponentModel, hand: &ClassifiedHand) -> bool {
    for constraint in &model.pass_constraints {
        if constraint.hand_kind == hand.kind && hand.strength >= constraint.strength {
            return false;
        }
    }
    true
}

fn outside_cards(view: &PlayerView) -> Vec<Card> {
    let mut known = BTreeSet::new();
    for card in &view.hand {
        known.insert(*card);
    }
    for record in &view.history {
        if let Decision::Play(cards) = &record.decision {
            for card in cards {
                known.insert(*card);
            }
        }
    }

    Card::standard_deck()
        .into_iter()
        .filter(|card| !known.contains(card))
        .collect()
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

    #[test]
    fn strategic_policy_does_not_break_pair_when_opening() {
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
            ],
            hand_counts: vec![6, 17, 17],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();

        assert_ne!(
            policy.decide(&view, &rules),
            Decision::Play(vec![card(Rank::Three, Suit::Clubs)])
        );
    }

    #[test]
    fn strategic_policy_responds_with_single_that_preserves_pair() {
        let rules = BasicRules;
        let previous_play = rules.classify(&[card(Rank::Three, Suit::Clubs)]);
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Five, Suit::Clubs),
            ],
            hand_counts: vec![3, 17, 17],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play,
        };
        let mut policy = StrategicPolicy::default();

        assert_eq!(
            policy.decide(&view, &rules),
            Decision::Play(vec![card(Rank::Five, Suit::Clubs)])
        );
    }

    #[test]
    fn strategic_policy_uses_high_single_when_opponent_is_almost_out() {
        let rules = BasicRules;
        let previous_play = rules.classify(&[card(Rank::Seven, Suit::Clubs)]);
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![card(Rank::Eight, Suit::Clubs), card(Rank::Ace, Suit::Clubs)],
            hand_counts: vec![2, 1, 5],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play,
        };
        let mut policy = StrategicPolicy::default();

        assert_eq!(
            policy.decide(&view, &rules),
            Decision::Play(vec![card(Rank::Ace, Suit::Clubs)])
        );
    }

    #[test]
    fn strategic_policy_farmer_does_not_treat_ally_as_threat() {
        let rules = BasicRules;
        let previous_play = rules.classify(&[card(Rank::Seven, Suit::Clubs)]);
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![card(Rank::Eight, Suit::Clubs), card(Rank::Ace, Suit::Clubs)],
            hand_counts: vec![5, 2, 1],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play,
        };
        let mut policy = StrategicPolicy::default();

        assert_eq!(
            policy.decide(&view, &rules),
            Decision::Play(vec![card(Rank::Eight, Suit::Clubs)])
        );
    }

    #[test]
    fn strategic_policy_farmer_blocks_single_when_landlord_has_one_card() {
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
            ],
            hand_counts: vec![1, 3, 5],
            relationships: Vec::new(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();

        assert_eq!(
            policy.decide(&view, &rules),
            Decision::Play(vec![
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
            ])
        );
    }

    #[test]
    fn legal_candidates_include_complex_rule_shapes() {
        let rules = BasicRules;
        let hand = vec![
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Four, Suit::Hearts),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Six, Suit::Clubs),
            card(Rank::Seven, Suit::Clubs),
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Jack, Suit::Clubs),
            card(Rank::Jack, Suit::Diamonds),
        ];

        let candidates = legal_candidates(&hand, None, &rules);

        assert!(candidates
            .iter()
            .any(|hand| hand.kind == crate::rules::HandKind::Straight && hand.cards.len() == 5));
        assert!(candidates
            .iter()
            .any(|hand| hand.kind == crate::rules::HandKind::TripleWithSingle));
        assert!(candidates
            .iter()
            .any(|hand| hand.kind == crate::rules::HandKind::TripleWithPair));
        assert!(candidates
            .iter()
            .any(|hand| hand.kind == crate::rules::HandKind::Airplane));
        assert!(candidates
            .iter()
            .any(|hand| hand.kind == crate::rules::HandKind::AirplaneWithSingles));
    }

    #[test]
    fn legal_candidates_are_filtered_by_previous_play() {
        let rules = BasicRules;
        let previous = rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
            ])
            .unwrap();
        let hand = vec![
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Five, Suit::Diamonds),
            card(Rank::Six, Suit::Diamonds),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Eight, Suit::Diamonds),
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Nine, Suit::Diamonds),
            card(Rank::Nine, Suit::Hearts),
            card(Rank::Nine, Suit::Spades),
        ];

        let candidates = legal_candidates(&hand, Some(&previous), &rules);

        assert!(candidates
            .iter()
            .all(|candidate| rules.can_play_over(candidate, Some(&previous))));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.kind == crate::rules::HandKind::Straight));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.kind == crate::rules::HandKind::Bomb));
    }

    #[test]
    fn legal_candidates_find_airplane_wings_past_invalid_early_kickers() {
        let rules = BasicRules;
        let previous = rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Seven, Suit::Diamonds),
                card(Rank::Eight, Suit::Diamonds),
            ])
            .unwrap();
        let hand = vec![
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Five, Suit::Diamonds),
            card(Rank::Five, Suit::Hearts),
            card(Rank::Six, Suit::Clubs),
            card(Rank::Six, Suit::Diamonds),
            card(Rank::Six, Suit::Hearts),
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Ten, Suit::Clubs),
        ];

        let candidates = legal_candidates(&hand, Some(&previous), &rules);

        assert!(candidates.iter().any(|candidate| {
            candidate.kind == crate::rules::HandKind::AirplaneWithSingles
                && candidate.strength == Rank::Six.strength()
        }));
        assert!(candidates
            .iter()
            .all(|candidate| rules.can_play_over(candidate, Some(&previous))));
    }

    #[test]
    fn build_opponent_models_tracks_played_cards() {
        use crate::engine::{PlayerId, TurnRecord};

        let view = PlayerView {
            self_id: PlayerId(0),
            hand: vec![card(Rank::Ace, Suit::Clubs)],
            hand_counts: vec![1, 2, 2],
            relationships: vec![
                Relationship::SelfPlayer,
                Relationship::Opponent,
                Relationship::Opponent,
            ],
            history: vec![
                TurnRecord {
                    player: PlayerId(1),
                    decision: Decision::Play(vec![
                        card(Rank::Three, Suit::Clubs),
                        card(Rank::Three, Suit::Diamonds),
                    ]),
                    accepted_hand: BasicRules.classify(&[
                        card(Rank::Three, Suit::Clubs),
                        card(Rank::Three, Suit::Diamonds),
                    ]),
                },
                TurnRecord {
                    player: PlayerId(2),
                    decision: Decision::Play(vec![
                        card(Rank::Five, Suit::Clubs),
                        card(Rank::Five, Suit::Diamonds),
                    ]),
                    accepted_hand: BasicRules.classify(&[
                        card(Rank::Five, Suit::Clubs),
                        card(Rank::Five, Suit::Diamonds),
                    ]),
                },
            ],
            previous_play: None,
        };

        let models = build_opponent_models(&view);

        assert!(models[0].is_none()); // SelfPlayer
        let p1 = models[1].as_ref().unwrap();
        assert!(p1.played_cards.contains(&card(Rank::Three, Suit::Clubs)));
        assert!(p1.played_cards.contains(&card(Rank::Three, Suit::Diamonds)));
        assert_eq!(p1.unknown_count, 2);
        let p2 = models[2].as_ref().unwrap();
        assert!(p2.played_cards.contains(&card(Rank::Five, Suit::Clubs)));
    }

    #[test]
    fn build_opponent_models_records_pass_constraints() {
        use crate::engine::{PlayerId, TurnRecord};

        let pair_seven = BasicRules.classify(&[
            card(Rank::Seven, Suit::Clubs),
            card(Rank::Seven, Suit::Diamonds),
        ]);

        let view = PlayerView {
            self_id: PlayerId(0),
            hand: vec![card(Rank::Ace, Suit::Clubs)],
            hand_counts: vec![1, 3, 2],
            relationships: vec![
                Relationship::SelfPlayer,
                Relationship::Opponent,
                Relationship::Opponent,
            ],
            history: vec![
                TurnRecord {
                    player: PlayerId(1),
                    decision: Decision::Play(vec![
                        card(Rank::Seven, Suit::Clubs),
                        card(Rank::Seven, Suit::Diamonds),
                    ]),
                    accepted_hand: pair_seven.clone(),
                },
                TurnRecord {
                    player: PlayerId(2),
                    decision: Decision::Pass,
                    accepted_hand: None,
                },
            ],
            previous_play: pair_seven,
        };

        let models = build_opponent_models(&view);

        let p2 = models[2].as_ref().unwrap();
        assert_eq!(p2.pass_constraints.len(), 1);
        assert_eq!(p2.pass_constraints[0].hand_kind, HandKind::Pair);
        assert_eq!(p2.pass_constraints[0].strength, Rank::Seven.strength());
    }

    #[test]
    fn opponent_can_beat_normal_uses_pass_constraints() {
        let model = OpponentModel {
            played_cards: BTreeSet::new(),
            pass_constraints: vec![PassConstraint {
                hand_kind: HandKind::Pair,
                strength: Rank::Seven.strength(),
            }],
            unknown_count: 3,
        };

        let pair_five = ClassifiedHand {
            kind: HandKind::Pair,
            strength: Rank::Five.strength(),
            cards: vec![
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
            ],
        };
        // passed on Pair(7) → lacks Pair(8+), but might have Pair(6) or Pair(7) → can beat Pair(5)
        assert!(opponent_can_beat_normal(&model, &pair_five));

        let pair_nine = ClassifiedHand {
            kind: HandKind::Pair,
            strength: Rank::Nine.strength(),
            cards: vec![
                card(Rank::Nine, Suit::Clubs),
                card(Rank::Nine, Suit::Diamonds),
            ],
        };
        // passed on Pair(7) → lacks Pair(8+) → cannot beat Pair(9)
        assert!(!opponent_can_beat_normal(&model, &pair_nine));

        let single_five = ClassifiedHand {
            kind: HandKind::Single,
            strength: Rank::Five.strength(),
            cards: vec![card(Rank::Five, Suit::Clubs)],
        };
        // different kind → constraint doesn't apply
        assert!(opponent_can_beat_normal(&model, &single_five));
    }
}
