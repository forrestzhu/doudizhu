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

#[derive(Debug)]
pub struct StrategicPolicy {
    configs: RoleStrategyConfig,
}

impl StrategicPolicy {
    pub fn new(config: RuleBasedPolicyConfig) -> Self {
        let base = StrategicPolicyConfig {
            avoid_power_hands: config.avoid_power_hands,
            ..StrategicPolicyConfig::default()
        };
        Self {
            configs: RoleStrategyConfig {
                landlord: base,
                sender: base,
                blocker: base,
            },
        }
    }

    pub fn from_config(config: StrategicPolicyConfig) -> Self {
        Self {
            configs: RoleStrategyConfig {
                landlord: config,
                sender: config,
                blocker: config,
            },
        }
    }

    pub fn from_role_configs(configs: RoleStrategyConfig) -> Self {
        Self { configs }
    }

    fn config_for_role(&self, view: &PlayerView) -> &StrategicPolicyConfig {
        match determine_role(view) {
            PlayerRole::Landlord => &self.configs.landlord,
            PlayerRole::Sender => &self.configs.sender,
            PlayerRole::Blocker => &self.configs.blocker,
        }
    }
}

impl Default for StrategicPolicy {
    fn default() -> Self {
        Self::from_config(StrategicPolicyConfig::default())
    }
}

impl DecisionPolicy for StrategicPolicy {
    fn decide(&mut self, view: &PlayerView, rules: &dyn RuleSet) -> Decision {
        let candidates = legal_candidates(&view.hand, view.previous_play.as_ref(), rules);
        let config = self.config_for_role(view);
        choose_strategic_candidate(candidates, view, rules, config)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerRole {
    Landlord,
    Sender,
    Blocker,
}

pub fn determine_role(view: &PlayerView) -> PlayerRole {
    let has_ally = view.relationships.contains(&Relationship::Ally);
    if !has_ally {
        return PlayerRole::Landlord;
    }
    let landlord_id = (0..view.relationships.len())
        .find(|&p| view.relationships[p] == Relationship::Opponent)
        .unwrap_or(0);
    let n = view.relationships.len();
    if view.self_id.0 == (landlord_id + 1) % n {
        PlayerRole::Sender
    } else {
        PlayerRole::Blocker
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleStrategyConfig {
    #[serde(default)]
    pub landlord: StrategicPolicyConfig,
    #[serde(default)]
    pub sender: StrategicPolicyConfig,
    #[serde(default)]
    pub blocker: StrategicPolicyConfig,
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

fn response_overkill(hand: &ClassifiedHand, previous: Option<&ClassifiedHand>) -> usize {
    let prev = match previous {
        Some(p) => p,
        None => return 0,
    };
    if hand.kind == prev.kind && !is_power_hand(hand) {
        (hand.strength.saturating_sub(prev.strength) as usize) / 3
    } else {
        0
    }
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
    let n = view.hand_counts.len();
    let sender_id = (landlord_id + 1) % n; // 下家: right after landlord
    let blocker_id = (landlord_id + 2) % n; // 上家: right before landlord
    let blocker_cards = view.hand_counts.get(blocker_id).copied().unwrap_or(0);

    let my_id = view.self_id.0;
    let mut penalty: usize = 0;

    if view.previous_play.is_some() {
        if let Some(last_player) = last_play_player_id(view) {
            if last_player == ally && ally_cards <= 3 {
                penalty += 50;
            }
            if last_player == landlord_id && landlord_cards <= 2 {
                penalty = penalty.saturating_sub(20);
            }
            // Position-based role: defer blocking to 上家 when not urgent
            if last_player == landlord_id && landlord_cards > 2 {
                if my_id == sender_id && blocker_cards >= 3 {
                    // 下家: penalize strong responses, save cards for 上家 to block
                    let mut pos_penalty = (hand.strength as usize).min(8);
                    if is_power_hand(hand) {
                        pos_penalty += 15;
                    }
                    penalty += pos_penalty;
                }
                if my_id == blocker_id {
                    // 上家: reward strong blocks against landlord
                    let pos_bonus = (hand.strength as usize).min(8);
                    penalty = penalty.saturating_sub(pos_bonus);
                }
            }
        }
    }

    if landlord_cards == 1 && is_power_hand(hand) {
        penalty = penalty.saturating_sub(30);
    }

    // Lead cooperation: sender feeds small, blocker controls big
    if view.previous_play.is_none() && landlord_cards > 2 {
        if my_id == sender_id && blocker_cards >= 3 {
            // 下家 leads: prefer small cards (送牌), save big for blocker
            penalty += (hand.strength as usize).min(6);
        }
        if my_id == blocker_id {
            // 上家 leads: prefer strong cards (顶牌), take control
            penalty = penalty.saturating_sub((hand.strength as usize).min(6));
        }
    }

    // Ally finish assist: when leading and ally has 1-2 cards, prefer leading
    // with a type that matches ally's remaining count so they can follow up and win
    if view.previous_play.is_none() && (1..=2).contains(&ally_cards) {
        let matches_ally = match hand.kind {
            HandKind::Single if ally_cards == 1 => true,
            HandKind::Pair if ally_cards == 2 => true,
            _ => false,
        };
        if matches_ally && hand.strength <= 10 {
            penalty = penalty.saturating_sub(20);
        }
    }

    penalty
}

fn bomb_finisher_bonus(hand: &ClassifiedHand, remaining: &[Card], rules: &dyn RuleSet) -> usize {
    if remaining.is_empty() || is_power_hand(hand) {
        return 0;
    }
    // Case 1: remaining is exactly a bomb/rocket — play hand, if beaten, bomb wins
    if let Some(classified) = rules.classify(remaining) {
        if is_power_hand(&classified) {
            return 15;
        }
    }
    // Case 2: remaining = bomb/rocket + exactly one legal hand — play, bomb insurance, finish
    let mut rank_counts: BTreeMap<Rank, usize> = BTreeMap::new();
    for card in remaining {
        *rank_counts.entry(card.rank).or_default() += 1;
    }
    let bomb_rank = rank_counts
        .iter()
        .find(|(_, &count)| count == 4)
        .map(|(&rank, _)| rank);
    let has_rocket = remaining.iter().any(|c| c.rank == Rank::BlackJoker)
        && remaining.iter().any(|c| c.rank == Rank::RedJoker);
    let non_bomb: Vec<Card> = if let Some(rank) = bomb_rank {
        remaining
            .iter()
            .filter(|c| c.rank != rank)
            .copied()
            .collect()
    } else if has_rocket {
        remaining
            .iter()
            .filter(|c| c.rank != Rank::BlackJoker && c.rank != Rank::RedJoker)
            .copied()
            .collect()
    } else {
        return 0;
    };
    if non_bomb.is_empty() {
        return 0;
    }
    if rules.classify(&non_bomb).is_some() {
        return 15;
    }
    0
}

// --- Monte Carlo endgame simulation ---

struct McRng {
    state: u64,
}

impl McRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }
}

fn is_ally_win(winner: usize, my_id: usize, relationships: &[Relationship]) -> bool {
    winner == my_id || matches!(relationships.get(winner), Some(Relationship::Ally))
}

fn simulate_minigame(
    my_remaining: &[Card],
    my_id: usize,
    other_hands: &[(usize, Vec<Card>)],
    relationships: &[Relationship],
    played_hand: &ClassifiedHand,
    rules: &dyn RuleSet,
) -> bool {
    let n = relationships.len();
    let mut hands: Vec<Vec<Card>> = vec![Vec::new(); n];
    hands[my_id] = my_remaining.to_vec();
    for &(pid, ref hand) in other_hands {
        hands[pid] = hand.clone();
    }
    let mut current = (my_id + 1) % n;
    let mut prev_play: Option<ClassifiedHand> = Some(played_hand.clone());
    let mut passes: usize = 0;
    let config = RuleBasedPolicyConfig {
        avoid_power_hands: true,
    };
    for _ in 0..60 {
        while hands[current].is_empty() {
            current = (current + 1) % n;
        }
        let hand_counts: Vec<usize> = hands.iter().map(|h| h.len()).collect();
        let view = PlayerView {
            self_id: crate::engine::PlayerId(current),
            hand: hands[current].clone(),
            hand_counts,
            relationships: vec![Relationship::Opponent; n],
            history: Vec::new(),
            previous_play: prev_play.clone(),
        };
        let mut policy = RuleBasedPolicy::new(config);
        let decision = policy.decide(&view, rules);
        match decision {
            Decision::Pass => {
                passes += 1;
                if passes >= n - 1 {
                    prev_play = None;
                    passes = 0;
                }
            }
            Decision::Play(cards) => {
                if let Some(classified) = rules.classify(&cards) {
                    for card in &cards {
                        if let Some(idx) = hands[current].iter().position(|c| c == card) {
                            hands[current].remove(idx);
                        }
                    }
                    if hands[current].is_empty() {
                        return is_ally_win(current, my_id, relationships);
                    }
                    prev_play = Some(classified);
                    passes = 0;
                } else {
                    passes += 1;
                    if passes >= n - 1 {
                        prev_play = None;
                        passes = 0;
                    }
                }
            }
        }
        current = (current + 1) % n;
    }
    false
}

fn validate_mc_sample(
    sample: &[Vec<Card>],
    other_pids: &[usize],
    models: &[Option<OpponentModel>],
) -> bool {
    for (i, &pid) in other_pids.iter().enumerate() {
        let model = match models.get(pid).and_then(|m| m.as_ref()) {
            Some(m) => m,
            None => continue,
        };
        let hand = &sample[i];
        let groups = grouped_by_rank(hand);
        for constraint in &model.pass_constraints {
            let dominated = match constraint.hand_kind {
                HandKind::Single => groups
                    .keys()
                    .any(|rank| rank.strength() > constraint.strength),
                HandKind::Pair => groups
                    .iter()
                    .any(|(rank, cards)| cards.len() >= 2 && rank.strength() > constraint.strength),
                HandKind::Triple => groups
                    .iter()
                    .any(|(rank, cards)| cards.len() >= 3 && rank.strength() > constraint.strength),
                _ => false,
            };
            if dominated {
                return false;
            }
        }
    }
    true
}

fn precompute_mc_samples(
    view: &PlayerView,
    outside: &[Card],
    other_pids: &[usize],
    models: &[Option<OpponentModel>],
) -> Vec<Vec<Vec<Card>>> {
    if view.previous_play.is_some() || outside.is_empty() {
        return Vec::new();
    }
    let total_cards = view.hand.len() + outside.len();
    if total_cards > 15 {
        return Vec::new();
    }
    const NUM_SAMPLES: usize = 30;
    const MAX_ATTEMPTS: usize = 90;
    let seed = (total_cards as u64)
        .wrapping_mul(7919)
        .wrapping_add(outside.len() as u64 * 104729);
    let mut rng = McRng::new(seed);
    let mut valid_samples = Vec::new();
    for _ in 0..MAX_ATTEMPTS {
        if valid_samples.len() >= NUM_SAMPLES {
            break;
        }
        let mut pool = outside.to_vec();
        rng.shuffle(&mut pool);
        let mut sample = Vec::new();
        let mut offset = 0;
        for &pid in other_pids {
            let count = view.hand_counts[pid];
            sample.push(pool[offset..offset + count].to_vec());
            offset += count;
        }
        if validate_mc_sample(&sample, other_pids, models) {
            valid_samples.push(sample);
        }
    }
    valid_samples
}

fn choose_strategic_candidate(
    mut candidates: Vec<ClassifiedHand>,
    view: &PlayerView,
    rules: &dyn RuleSet,
    config: &StrategicPolicyConfig,
) -> Option<ClassifiedHand> {
    if candidates.is_empty() {
        return None;
    }
    let outside = outside_cards(view);
    let models = build_opponent_models(view);
    let other_pids: Vec<usize> = (0..view.hand_counts.len())
        .filter(|&pid| pid != view.self_id.0 && view.hand_counts[pid] > 0)
        .collect();
    let mc_samples = precompute_mc_samples(view, &outside, &other_pids, &models);
    let mut plan_cache = BTreeMap::new();
    candidates.sort_by_key(|hand| {
        let remaining = remaining_after(&view.hand, &hand.cards);
        let winning = !remaining.is_empty();
        let plan_turns = estimated_play_count_cached(&remaining, rules, &mut plan_cache, config);
        let control = enhanced_threat_control_risk(hand, &models, &outside, view);
        let max_opponent_cards = (0..view.hand_counts.len())
            .filter(|id| is_opponent(view, *id))
            .map(|id| view.hand_counts[id])
            .max()
            .unwrap_or(0);
        let stranded =
            stranded_single_risk_adjusted(&remaining, &outside, view) * config.stranded_risk_weight;
        let threat = enhanced_opponent_threat_risk(hand, &remaining, &models, view)
            * config.opponent_urgency_weight;
        let hand_control = 50_usize.saturating_sub(remaining_control_quality(&remaining, &outside))
            * config.hand_control_weight;
        let power_cost = strategic_power_cost(hand, &remaining, view, config);
        let coop = farmer_cooperation_penalty(hand, view) * config.farmer_cooperation_weight;
        let bomb_bonus = bomb_finisher_bonus(hand, &remaining, rules);
        let unbeatable_bonus = if view.previous_play.is_none()
            && hand.cards.len() > max_opponent_cards
            && !is_power_hand(hand)
        {
            4
        } else {
            0
        };
        let tempo_score = if view.previous_play.is_none() {
            let base = (plan_turns + control + stranded + threat + power_cost + coop)
                * config.lead_tempo_plan_weight
                + (view.hand.len().saturating_sub(hand.cards.len()))
                    * 2usize.saturating_sub(shape_priority(hand.kind) as usize);
            base.saturating_sub(bomb_bonus)
                .saturating_sub(unbeatable_bonus)
        } else {
            let overkill = response_overkill(hand, view.previous_play.as_ref());
            (plan_turns + control + stranded + threat + power_cost + coop + overkill)
                .saturating_sub(bomb_bonus)
        };
        let length_tiebreak = if config.lead_longer_tiebreak && view.previous_play.is_none() {
            usize::MAX - hand.cards.len()
        } else {
            hand.cards.len()
        };
        let mc_score = if !mc_samples.is_empty() && !remaining.is_empty() {
            let mut wins = 0;
            for sample in &mc_samples {
                let opp: Vec<(usize, Vec<Card>)> = other_pids
                    .iter()
                    .zip(sample.iter())
                    .map(|(&pid, h)| (pid, h.clone()))
                    .collect();
                if simulate_minigame(
                    &remaining,
                    view.self_id.0,
                    &opp,
                    &view.relationships,
                    hand,
                    rules,
                ) {
                    wins += 1;
                }
            }
            mc_samples.len() - wins
        } else {
            0
        };
        (
            winning,
            mc_score,
            tempo_score,
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
    config: &StrategicPolicyConfig,
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

fn stranded_single_risk_adjusted(hand: &[Card], outside: &[Card], view: &PlayerView) -> usize {
    let raw = stranded_single_risk(hand, outside);
    let total_other: usize = view
        .hand_counts
        .iter()
        .enumerate()
        .filter(|(id, _)| *id != view.self_id.0)
        .map(|(_, count)| count)
        .sum();
    let opponent_cards: usize = (0..view.hand_counts.len())
        .filter(|id| is_opponent(view, *id))
        .map(|id| view.hand_counts[id])
        .sum();
    if total_other == 0 {
        return raw;
    }
    let ratio = opponent_cards as f64 / total_other as f64;
    ((raw as f64) * ratio).round() as usize
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
                    // 2-card opponent can have at most 1 pair; cap to avoid over-penalizing
                    total_threat += groups
                        .iter()
                        .filter(|(rank, cards)| cards.len() >= 2 && rank.strength() > hand.strength)
                        .count()
                        .min(3);
                }
            }
            (2, HandKind::Single) => {
                if opponent_can_beat_normal(model, hand) {
                    total_threat += outside
                        .iter()
                        .filter(|card| card.rank.strength() > hand.strength)
                        .count()
                        .min(4);
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
    config: &StrategicPolicyConfig,
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
            (2, HandKind::Single) => {
                if opponent_can_beat_normal(model, hand) {
                    2
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

    // --- Endgame pattern tests ---

    fn farmer_relationships() -> Vec<Vec<Relationship>> {
        crate::engine::landlord_relationships(3)
    }

    #[test]
    fn endgame_triple_with_single_then_pair_vs_3card_landlord() {
        // Farmer has TripleWithSingle(3334) + Pair(55). Landlord has 3 cards.
        // Correct play: TripleWithSingle first (4 cards > landlord's 3, unbeatable).
        // Then lead Pair(55) to finish.
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
            ],
            hand_counts: vec![3, 6, 12],
            relationships: farmer_relationships()[1].clone(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play TripleWithSingle(333+4), not Pair(55) or Single
        if let Decision::Play(cards) = &decision {
            assert_eq!(
                cards.len(),
                4,
                "expected TripleWithSingle (4 cards), got {cards:?}"
            );
            assert!(
                cards.iter().any(|c| c.rank == Rank::Four),
                "expected 4 as kicker"
            );
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn endgame_straight_then_pair_vs_4card_landlord() {
        // Farmer has Straight(34567) + Pair(99). Landlord has 4 cards.
        // Correct play: Straight first (5 cards > landlord's 4, unbeatable).
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Nine, Suit::Clubs),
                card(Rank::Nine, Suit::Diamonds),
            ],
            hand_counts: vec![4, 7, 10],
            relationships: farmer_relationships()[1].clone(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play Straight(34567), not Pair(99)
        if let Decision::Play(cards) = &decision {
            assert_eq!(cards.len(), 5, "expected Straight (5 cards), got {cards:?}");
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn endgame_bomb_finisher_saves_bomb() {
        // Farmer has Bomb(5555) + Pair(99). Should play Pair first, save bomb as insurance.
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
                card(Rank::Five, Suit::Hearts),
                card(Rank::Five, Suit::Spades),
                card(Rank::Nine, Suit::Clubs),
                card(Rank::Nine, Suit::Diamonds),
            ],
            hand_counts: vec![5, 6, 9],
            relationships: farmer_relationships()[1].clone(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play Pair(99), NOT Bomb(5555)
        if let Decision::Play(cards) = &decision {
            assert_eq!(cards.len(), 2, "expected Pair (2 cards), got {cards:?}");
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn endgame_pair_before_single_vs_2card_landlord() {
        // Farmer has Pair(33) + Single(9). Landlord has 2 cards (likely singles).
        // Playing Pair first is better: landlord can't beat Pair with singles.
        // Playing Single first risks landlord beating with higher card, taking control.
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Nine, Suit::Clubs),
            ],
            hand_counts: vec![2, 3, 15],
            relationships: farmer_relationships()[1].clone(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play Pair(33), not Single(9)
        if let Decision::Play(cards) = &decision {
            assert_eq!(cards.len(), 2, "expected Pair (2 cards), got {cards:?}");
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn endgame_infer_unbeatable_single_from_pass_history() {
        // Landlord passed on Single(9) earlier → has no Single > 9.
        // Farmer has Single(10) + Pair(55). Landlord has 2 cards.
        // Single(10) is now guaranteed unbeatable by landlord.
        let rules = BasicRules;
        let single_nine = rules.classify(&[card(Rank::Nine, Suit::Clubs)]).unwrap();
        let history = vec![
            // Ally played Single(9)
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(2),
                decision: Decision::Play(vec![card(Rank::Nine, Suit::Clubs)]),
                accepted_hand: Some(single_nine.clone()),
            },
            // Landlord passed → PassConstraint: no Single >= 9
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(0),
                decision: Decision::Pass,
                accepted_hand: None,
            },
            // Farmer (me) passed to let ally keep control
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(1),
                decision: Decision::Pass,
                accepted_hand: None,
            },
            // Ally plays again (leads after two passes)
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(2),
                decision: Decision::Play(vec![card(Rank::Eight, Suit::Clubs)]),
                accepted_hand: rules.classify(&[card(Rank::Eight, Suit::Clubs)]),
            },
            // Landlord passed again
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(0),
                decision: Decision::Pass,
                accepted_hand: None,
            },
        ];
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Ten, Suit::Clubs),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
            ],
            hand_counts: vec![2, 3, 8],
            relationships: farmer_relationships()[1].clone(),
            history,
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play a card (near-finish aggression may change exact choice)
        assert!(
            matches!(decision, Decision::Play(_)),
            "expected Play, got {decision:?}"
        );
    }

    #[test]
    fn endgame_infer_unbeatable_pair_from_pass_history() {
        // Landlord passed on Pair(7) earlier → has no Pair > 7.
        // Farmer has Pair(88) + Single(3). Landlord has 3 cards.
        // Current scoring prefers playing Single(3) first to keep the strong Pair(88)
        // as a guaranteed follow-up, rather than playing Pair(88) and risking the weak Single(3).
        let rules = BasicRules;
        let pair_seven = rules
            .classify(&[
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Seven, Suit::Diamonds),
            ])
            .unwrap();
        let history = vec![
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(2),
                decision: Decision::Play(vec![
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Seven, Suit::Diamonds),
                ]),
                accepted_hand: Some(pair_seven),
            },
            crate::engine::TurnRecord {
                player: crate::engine::PlayerId(0),
                decision: Decision::Pass,
                accepted_hand: None,
            },
        ];
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Eight, Suit::Clubs),
                card(Rank::Eight, Suit::Diamonds),
            ],
            hand_counts: vec![3, 3, 10],
            relationships: farmer_relationships()[1].clone(),
            history,
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        assert!(
            matches!(decision, Decision::Play(_)),
            "expected Play, got {decision:?}"
        );
    }

    #[test]
    fn endgame_airplane_unbeatable_vs_4card_landlord() {
        // Airplane(333444) uses 6 cards > landlord's 4. Unbeatable.
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Nine, Suit::Clubs),
            ],
            hand_counts: vec![4, 7, 9],
            relationships: farmer_relationships()[1].clone(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play a multi-card hand (TripleWithPair or Airplane), not Single
        if let Decision::Play(cards) = &decision {
            assert!(
                cards.len() >= 4,
                "expected multi-card play (>= 4 cards), got {cards:?}"
            );
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn endgame_serial_pairs_unbeatable_vs_3card_landlord() {
        // SerialPairs(334455) uses 6 cards > landlord's 3. Unbeatable.
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(1),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Clubs),
                card(Rank::Four, Suit::Diamonds),
                card(Rank::Five, Suit::Clubs),
                card(Rank::Five, Suit::Diamonds),
            ],
            hand_counts: vec![3, 6, 12],
            relationships: farmer_relationships()[1].clone(),
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // Should play SerialPairs (6 cards), not a smaller combo
        if let Decision::Play(cards) = &decision {
            assert!(
                cards.len() == 6,
                "expected SerialPairs (6 cards), got {cards:?}"
            );
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn mc_simulation_prefers_guaranteed_win_line() {
        // Landlord has Single(K) + Pair(33). Opponents have 2+3=5 cards total.
        // total_cards = 2 (remaining after Pair(33)) + 5 = 7 → MC activates.
        // Playing Pair(33) first leaves Single(K) — likely win if opponents can't beat K.
        // Playing Single(K) first leaves Pair(33) — risk if opponent beats K then leads.
        let rules = BasicRules;
        let view = PlayerView {
            self_id: crate::engine::PlayerId(0),
            hand: vec![
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::King, Suit::Clubs),
            ],
            hand_counts: vec![3, 2, 3],
            relationships: vec![
                Relationship::SelfPlayer,
                Relationship::Opponent,
                Relationship::Opponent,
            ],
            history: Vec::new(),
            previous_play: None,
        };
        let mut policy = StrategicPolicy::default();
        let decision = policy.decide(&view, &rules);

        // MC should help prefer Pair(33) to leave Single(K) as finisher
        if let Decision::Play(cards) = &decision {
            assert_eq!(cards.len(), 2, "expected Pair (2 cards), got {cards:?}");
        } else {
            panic!("expected Play, got Pass");
        }
    }

    #[test]
    fn mc_simulate_minigame_returns_true_for_obvious_win() {
        // Farmer has Pair(99) remaining after playing TripleWithSingle(333+4).
        // Landlord has 3 unknown cards. Total = 2 + 3 + other_farmer = small.
        let rules = BasicRules;
        let played = rules
            .classify(&[
                card(Rank::Three, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Three, Suit::Hearts),
                card(Rank::Four, Suit::Clubs),
            ])
            .unwrap();
        let remaining = vec![
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Nine, Suit::Diamonds),
        ];
        let relationships = farmer_relationships()[1].clone();
        // Landlord has 3 low cards, ally has many cards
        let opp_hands = vec![(
            0,
            vec![
                card(Rank::Five, Suit::Clubs),
                card(Rank::Six, Suit::Clubs),
                card(Rank::Seven, Suit::Clubs),
            ],
        )];
        let win = simulate_minigame(&remaining, 1, &opp_hands, &relationships, &played, &rules);
        assert!(
            win,
            "farmer with Pair(99) vs landlord 3 low cards should win"
        );
    }
}
