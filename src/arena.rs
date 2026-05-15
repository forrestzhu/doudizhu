use crate::cards::Card;
use crate::decision::{
    legal_candidates, Decision, DecisionPolicy, RoleStrategyConfig, RuleBasedPolicy,
    RuleBasedPolicyConfig, StrategicPolicy,
};
use crate::engine::{Deal, Game, GameConfig, GameError, GameStatus, PlayerId, TurnRecord};
use crate::rules::{BasicRules, ClassifiedHand, RuleSet};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum ArenaError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidCard(String),
    InvalidScenario(String),
    Game(GameError),
}

impl From<std::io::Error> for ArenaError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ArenaError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<GameError> for ArenaError {
    fn from(error: GameError) -> Self {
        Self::Game(error)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub id: String,
    pub pass: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioReport {
    pub schema_version: String,
    pub name: String,
    pub kind: String,
    pub deterministic: bool,
    pub pass: bool,
    pub checks: Vec<CheckReport>,
    pub metrics: BTreeMap<String, u64>,
    pub details: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeededGameReport {
    pub game: usize,
    pub seed: u64,
    pub winner: Option<usize>,
    pub turns: usize,
    pub history_hash: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeededRunReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub seed: u64,
    pub games: usize,
    pub landlord_policy: String,
    pub wins: Vec<usize>,
    pub avg_turns: f64,
    pub reports: Vec<SeededGameReport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandlordPolicy {
    RuleBased,
    Strategic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyPlacement {
    AllRuleBased,
    LandlordStrategic,
    FarmersStrategic,
    AllStrategic,
    LandlordNewFarmersOld,
    LandlordOldFarmersNew,
}

impl PolicyPlacement {
    pub fn name(self) -> &'static str {
        match self {
            Self::AllRuleBased => "all_rule_based",
            Self::LandlordStrategic => "landlord_strategic",
            Self::FarmersStrategic => "farmers_strategic",
            Self::AllStrategic => "all_strategic",
            Self::LandlordNewFarmersOld => "landlord_v6_farmers_v5",
            Self::LandlordOldFarmersNew => "landlord_v5_farmers_v6",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TournamentRunReport {
    pub placement: String,
    pub games: usize,
    pub wins: Vec<usize>,
    pub landlord_win_rate: f64,
    pub farmer_win_rate: f64,
    pub avg_turns: f64,
    pub reports: Vec<SeededGameReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TournamentConclusion {
    pub significance_threshold: f64,
    pub landlord_strategic_delta: f64,
    pub farmers_strategic_delta: f64,
    pub landlord_strategic_significant: bool,
    pub farmers_strategic_significant: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TournamentOptions {
    pub num_threads: usize,
    pub pilot_games: usize,
    pub early_stop_regression: f64,
}

impl Default for TournamentOptions {
    fn default() -> Self {
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            num_threads,
            pilot_games: 100,
            early_stop_regression: 0.15,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RandomTournamentReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub random_source: u64,
    pub games: usize,
    pub deal_seeds: Vec<u64>,
    pub strategy: RoleStrategyConfig,
    pub runs: Vec<TournamentRunReport>,
    pub conclusion: TournamentConclusion,
    pub early_stopped: bool,
}

impl LandlordPolicy {
    pub fn name(self) -> &'static str {
        match self {
            Self::RuleBased => "rule_based",
            Self::Strategic => "strategic",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DealPlayerReport {
    pub id: usize,
    pub role: String,
    pub relationship: String,
    pub hand_count: usize,
    pub visible_hand: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DealReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub seed: u64,
    pub viewer: usize,
    pub bottom_cards: Vec<String>,
    pub players: Vec<DealPlayerReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerPublicReport {
    pub id: usize,
    pub role: String,
    pub relationship: String,
    pub hand_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct HandReport {
    pub kind: String,
    pub cards: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TurnReport {
    pub turn: usize,
    pub player: usize,
    pub decision: String,
    pub cards: Vec<String>,
    pub accepted_hand: Option<HandReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GameViewReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub seed: u64,
    pub viewer: usize,
    pub current_player: usize,
    pub winner: Option<usize>,
    pub visible_hand: Vec<String>,
    pub hand_counts: Vec<usize>,
    pub players: Vec<PlayerPublicReport>,
    pub relationships: Vec<String>,
    pub history: Vec<TurnReport>,
    pub previous_player: Option<usize>,
    pub previous_play: Option<HandReport>,
    pub bottom_cards: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HintReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub seed: u64,
    pub viewer: usize,
    pub current_player: usize,
    pub legal_hints: Vec<Vec<String>>,
    pub recommended: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub seed: u64,
    pub view: GameViewReport,
    pub hint: HintReport,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionAction {
    Auto,
    Manual {
        #[serde(default)]
        cards: Vec<String>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct EpisodeReport {
    pub schema_version: String,
    pub deterministic: bool,
    pub seed: u64,
    pub policies: Vec<PolicyReport>,
    pub initial_hands: Vec<Vec<String>>,
    pub bottom_cards: Vec<String>,
    pub winner: Option<usize>,
    pub turns: usize,
    pub history: Vec<TurnReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyReport {
    pub player: usize,
    pub name: String,
    pub avoid_power_hands: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum Scenario {
    #[serde(rename = "legal_candidates")]
    LegalCandidates {
        name: String,
        hand: Vec<String>,
        previous_play: Option<Vec<String>>,
        #[serde(default)]
        expect: LegalCandidatesExpect,
    },
    #[serde(rename = "visibility")]
    Visibility {
        name: String,
        hands: Vec<Vec<String>>,
        viewer: usize,
        expect: VisibilityExpect,
    },
    #[serde(rename = "self_play")]
    SelfPlay {
        name: String,
        seed: u64,
        #[serde(default = "one_game")]
        games: usize,
        max_turns: Option<usize>,
        #[serde(default)]
        expect: SelfPlayExpect,
    },
}

#[derive(Debug, Default, Deserialize)]
struct LegalCandidatesExpect {
    #[serde(default)]
    contains: Vec<Vec<String>>,
    #[serde(default)]
    excludes: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct VisibilityExpect {
    hand: Vec<String>,
    hand_counts: Vec<usize>,
    #[serde(default)]
    forbidden_cards: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SelfPlayExpect {
    wins: Option<Vec<usize>>,
}

fn one_game() -> usize {
    1
}

pub fn run_scenario_file(path: &Path) -> Result<ScenarioReport, ArenaError> {
    let contents = std::fs::read_to_string(path)?;
    run_scenario_str(&contents)
}

pub fn run_scenario_str(contents: &str) -> Result<ScenarioReport, ArenaError> {
    let scenario: Scenario = serde_json::from_str(contents)?;
    match scenario {
        Scenario::LegalCandidates {
            name,
            hand,
            previous_play,
            expect,
        } => run_legal_candidates_scenario(name, hand, previous_play, expect),
        Scenario::Visibility {
            name,
            hands,
            viewer,
            expect,
        } => run_visibility_scenario(name, hands, viewer, expect),
        Scenario::SelfPlay {
            name,
            seed,
            games,
            max_turns,
            expect,
        } => run_self_play_scenario(name, seed, games, max_turns.unwrap_or(1_000), expect),
    }
}

pub fn run_seeded_games(
    seed: u64,
    games: usize,
    max_turns: usize,
) -> Result<SeededRunReport, ArenaError> {
    run_seeded_games_with_landlord_policy(seed, games, max_turns, LandlordPolicy::RuleBased)
}

pub fn run_seeded_games_with_landlord_policy(
    seed: u64,
    games: usize,
    max_turns: usize,
    landlord_policy: LandlordPolicy,
) -> Result<SeededRunReport, ArenaError> {
    if games == 0 {
        return Err(ArenaError::InvalidScenario(
            "games must be greater than zero".to_string(),
        ));
    }

    let mut wins = vec![0usize; 3];
    let mut reports = Vec::with_capacity(games);
    let mut total_turns = 0usize;

    for index in 0..games {
        let game_seed = seed + index as u64;
        let deal = Deal::from_seed(game_seed, 3);
        let config = GameConfig {
            max_turns,
            ..GameConfig::default()
        };
        let mut game = Game::new(deal, config)?;
        let mut policies = self_play_policies(landlord_policy, RuleBasedPolicyConfig::default());

        match game.run(&mut policies) {
            Ok(outcome) => {
                wins[outcome.winner.0] += 1;
                total_turns += outcome.turns;
                reports.push(SeededGameReport {
                    game: index + 1,
                    seed: game_seed,
                    winner: Some(outcome.winner.0),
                    turns: outcome.turns,
                    history_hash: history_hash(game.history()),
                    error: None,
                });
            }
            Err(error) => {
                reports.push(SeededGameReport {
                    game: index + 1,
                    seed: game_seed,
                    winner: None,
                    turns: game.history().len(),
                    history_hash: history_hash(game.history()),
                    error: Some(format!("{error:?}")),
                });
            }
        }
    }

    Ok(SeededRunReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        games,
        landlord_policy: landlord_policy.name().to_string(),
        wins,
        avg_turns: total_turns as f64 / games as f64,
        reports,
    })
}

pub fn run_random_tournament(
    games: usize,
    max_turns: usize,
    strategy: RoleStrategyConfig,
    significance_threshold: f64,
) -> Result<RandomTournamentReport, ArenaError> {
    run_random_tournament_from_source(
        random_source(),
        games,
        max_turns,
        strategy,
        significance_threshold,
    )
}

pub fn run_random_tournament_from_source(
    random_source: u64,
    games: usize,
    max_turns: usize,
    strategy: RoleStrategyConfig,
    significance_threshold: f64,
) -> Result<RandomTournamentReport, ArenaError> {
    if games == 0 {
        return Err(ArenaError::InvalidScenario(
            "games must be greater than zero".to_string(),
        ));
    }

    let deal_seeds = random_deal_seeds(random_source, games);
    let baseline = run_policy_placement_games(
        &deal_seeds,
        max_turns,
        PolicyPlacement::AllRuleBased,
        strategy,
    )?;
    let landlord_strategic = run_policy_placement_games(
        &deal_seeds,
        max_turns,
        PolicyPlacement::LandlordStrategic,
        strategy,
    )?;
    let farmers_strategic = run_policy_placement_games(
        &deal_seeds,
        max_turns,
        PolicyPlacement::FarmersStrategic,
        strategy,
    )?;
    let all_strategic = run_policy_placement_games(
        &deal_seeds,
        max_turns,
        PolicyPlacement::AllStrategic,
        strategy,
    )?;

    let landlord_strategic_delta =
        landlord_strategic.landlord_win_rate - baseline.landlord_win_rate;
    let farmers_strategic_delta = farmers_strategic.farmer_win_rate - baseline.farmer_win_rate;

    Ok(RandomTournamentReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: false,
        random_source,
        games,
        deal_seeds,
        strategy,
        runs: vec![
            baseline,
            landlord_strategic,
            farmers_strategic,
            all_strategic,
        ],
        conclusion: TournamentConclusion {
            significance_threshold,
            landlord_strategic_delta,
            farmers_strategic_delta,
            landlord_strategic_significant: landlord_strategic_delta >= significance_threshold,
            farmers_strategic_significant: farmers_strategic_delta >= significance_threshold,
        },
        early_stopped: false,
    })
}

fn run_policy_placement_games(
    deal_seeds: &[u64],
    max_turns: usize,
    placement: PolicyPlacement,
    strategy: RoleStrategyConfig,
) -> Result<TournamentRunReport, ArenaError> {
    let mut wins = vec![0usize; 3];
    let mut reports = Vec::with_capacity(deal_seeds.len());
    let mut total_turns = 0usize;

    for (index, game_seed) in deal_seeds.iter().copied().enumerate() {
        let deal = Deal::from_seed(game_seed, 3);
        let config = GameConfig {
            max_turns,
            ..GameConfig::default()
        };
        let mut game = Game::new(deal, config)?;
        let mut policies =
            placement_policies(placement, RuleBasedPolicyConfig::default(), strategy, None);

        match game.run(&mut policies) {
            Ok(outcome) => {
                wins[outcome.winner.0] += 1;
                total_turns += outcome.turns;
                reports.push(SeededGameReport {
                    game: index + 1,
                    seed: game_seed,
                    winner: Some(outcome.winner.0),
                    turns: outcome.turns,
                    history_hash: history_hash(game.history()),
                    error: None,
                });
            }
            Err(error) => {
                reports.push(SeededGameReport {
                    game: index + 1,
                    seed: game_seed,
                    winner: None,
                    turns: game.history().len(),
                    history_hash: history_hash(game.history()),
                    error: Some(format!("{error:?}")),
                });
            }
        }
    }

    let games = deal_seeds.len();
    let landlord_win_rate = wins[0] as f64 / games as f64;
    let farmer_win_rate = (wins[1] + wins[2]) as f64 / games as f64;

    Ok(TournamentRunReport {
        placement: placement.name().to_string(),
        games,
        wins,
        landlord_win_rate,
        farmer_win_rate,
        avg_turns: total_turns as f64 / games as f64,
        reports,
    })
}

pub fn run_cross_placement_games(
    deal_seeds: &[u64],
    max_turns: usize,
    placement: PolicyPlacement,
    strategy: RoleStrategyConfig,
    baseline: RoleStrategyConfig,
) -> Result<TournamentRunReport, ArenaError> {
    let mut wins = vec![0usize; 3];
    let mut reports = Vec::with_capacity(deal_seeds.len());
    let mut total_turns = 0usize;

    for (index, game_seed) in deal_seeds.iter().copied().enumerate() {
        let deal = Deal::from_seed(game_seed, 3);
        let config = GameConfig {
            max_turns,
            ..GameConfig::default()
        };
        let mut game = Game::new(deal, config)?;
        let mut policies = placement_policies(
            placement,
            RuleBasedPolicyConfig::default(),
            strategy,
            Some(baseline),
        );

        match game.run(&mut policies) {
            Ok(outcome) => {
                wins[outcome.winner.0] += 1;
                total_turns += outcome.turns;
                reports.push(SeededGameReport {
                    game: index + 1,
                    seed: game_seed,
                    winner: Some(outcome.winner.0),
                    turns: outcome.turns,
                    history_hash: history_hash(game.history()),
                    error: None,
                });
            }
            Err(error) => {
                reports.push(SeededGameReport {
                    game: index + 1,
                    seed: game_seed,
                    winner: None,
                    turns: 0,
                    history_hash: String::new(),
                    error: Some(format!("{error:?}")),
                });
            }
        }
    }

    let games = deal_seeds.len();
    let landlord_win_rate = wins[0] as f64 / games as f64;
    let farmer_win_rate = (wins[1] + wins[2]) as f64 / games as f64;

    Ok(TournamentRunReport {
        placement: placement.name().to_string(),
        games,
        wins,
        landlord_win_rate,
        farmer_win_rate,
        avg_turns: total_turns as f64 / games as f64,
        reports,
    })
}

fn run_policy_placement_games_parallel(
    deal_seeds: &[u64],
    max_turns: usize,
    placement: PolicyPlacement,
    strategy: RoleStrategyConfig,
    num_threads: usize,
) -> Result<TournamentRunReport, ArenaError> {
    if deal_seeds.is_empty() {
        return Ok(TournamentRunReport {
            placement: placement.name().to_string(),
            games: 0,
            wins: vec![0; 3],
            landlord_win_rate: 0.0,
            farmer_win_rate: 0.0,
            avg_turns: 0.0,
            reports: Vec::new(),
        });
    }

    let rule_config = RuleBasedPolicyConfig::default();
    let chunk_size = deal_seeds.len().div_ceil(num_threads);

    let thread_results = thread::scope(|s| {
        deal_seeds
            .chunks(chunk_size)
            .enumerate()
            .map(|(thread_idx, chunk)| {
                let start_index = thread_idx * chunk_size;
                s.spawn(move || {
                    let mut wins = vec![0usize; 3];
                    let mut total_turns = 0usize;
                    let mut reports = Vec::with_capacity(chunk.len());

                    for (i, game_seed) in chunk.iter().copied().enumerate() {
                        let game_index = start_index + i + 1;
                        let deal = Deal::from_seed(game_seed, 3);
                        let config = GameConfig {
                            max_turns,
                            ..GameConfig::default()
                        };
                        let mut game = Game::new(deal, config).expect("valid deal");
                        let mut policies =
                            placement_policies(placement, rule_config, strategy, None);

                        match game.run(&mut policies) {
                            Ok(outcome) => {
                                wins[outcome.winner.0] += 1;
                                total_turns += outcome.turns;
                                reports.push(SeededGameReport {
                                    game: game_index,
                                    seed: game_seed,
                                    winner: Some(outcome.winner.0),
                                    turns: outcome.turns,
                                    history_hash: history_hash(game.history()),
                                    error: None,
                                });
                            }
                            Err(error) => {
                                reports.push(SeededGameReport {
                                    game: game_index,
                                    seed: game_seed,
                                    winner: None,
                                    turns: game.history().len(),
                                    history_hash: history_hash(game.history()),
                                    error: Some(format!("{error:?}")),
                                });
                            }
                        }
                    }

                    (wins, total_turns, reports)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });

    let mut all_wins = vec![0usize; 3];
    let mut all_total_turns = 0usize;
    let mut all_reports = Vec::with_capacity(deal_seeds.len());

    for (wins, total_turns, reports) in thread_results {
        for i in 0..3 {
            all_wins[i] += wins[i];
        }
        all_total_turns += total_turns;
        all_reports.extend(reports);
    }

    all_reports.sort_by_key(|r| r.game);

    let games = deal_seeds.len();
    let landlord_win_rate = all_wins[0] as f64 / games as f64;
    let farmer_win_rate = (all_wins[1] + all_wins[2]) as f64 / games as f64;

    Ok(TournamentRunReport {
        placement: placement.name().to_string(),
        games,
        wins: all_wins,
        landlord_win_rate,
        farmer_win_rate,
        avg_turns: all_total_turns as f64 / games as f64,
        reports: all_reports,
    })
}

fn merge_run_reports(a: &TournamentRunReport, b: &TournamentRunReport) -> TournamentRunReport {
    let games = a.games + b.games;
    let wins: Vec<usize> = a.wins.iter().zip(&b.wins).map(|(x, y)| x + y).collect();
    let total_turns = (a.avg_turns * a.games as f64 + b.avg_turns * b.games as f64) as usize;
    let mut reports = a.reports.clone();
    for r in &b.reports {
        reports.push(SeededGameReport {
            game: r.game + a.games,
            ..r.clone()
        });
    }

    TournamentRunReport {
        placement: a.placement.clone(),
        games,
        wins: wins.clone(),
        landlord_win_rate: wins[0] as f64 / games as f64,
        farmer_win_rate: (wins[1] + wins[2]) as f64 / games as f64,
        avg_turns: total_turns as f64 / games as f64,
        reports,
    }
}

pub fn run_random_tournament_from_source_opt(
    random_source: u64,
    games: usize,
    max_turns: usize,
    strategy: RoleStrategyConfig,
    significance_threshold: f64,
    options: TournamentOptions,
) -> Result<RandomTournamentReport, ArenaError> {
    if games == 0 {
        return Err(ArenaError::InvalidScenario(
            "games must be greater than zero".to_string(),
        ));
    }

    let deal_seeds = random_deal_seeds(random_source, games);
    let num_threads = options.num_threads.max(1);
    let run_fn = |seeds: &[u64],
                  placement: PolicyPlacement|
     -> Result<TournamentRunReport, ArenaError> {
        if num_threads == 1 {
            run_policy_placement_games(seeds, max_turns, placement, strategy)
        } else {
            run_policy_placement_games_parallel(seeds, max_turns, placement, strategy, num_threads)
        }
    };

    let pilot_games = options.pilot_games.min(games);
    let early_stop = pilot_games > 0 && pilot_games < games && options.early_stop_regression > 0.0;

    if !early_stop {
        let baseline = run_fn(&deal_seeds, PolicyPlacement::AllRuleBased)?;
        let landlord_strategic = run_fn(&deal_seeds, PolicyPlacement::LandlordStrategic)?;
        let farmers_strategic = run_fn(&deal_seeds, PolicyPlacement::FarmersStrategic)?;
        let all_strategic = run_fn(&deal_seeds, PolicyPlacement::AllStrategic)?;

        let landlord_strategic_delta =
            landlord_strategic.landlord_win_rate - baseline.landlord_win_rate;
        let farmers_strategic_delta = farmers_strategic.farmer_win_rate - baseline.farmer_win_rate;

        return Ok(RandomTournamentReport {
            schema_version: "2026-05-11".to_string(),
            deterministic: false,
            random_source,
            games,
            deal_seeds,
            strategy,
            runs: vec![
                baseline,
                landlord_strategic,
                farmers_strategic,
                all_strategic,
            ],
            conclusion: TournamentConclusion {
                significance_threshold,
                landlord_strategic_delta,
                farmers_strategic_delta,
                landlord_strategic_significant: landlord_strategic_delta >= significance_threshold,
                farmers_strategic_significant: farmers_strategic_delta >= significance_threshold,
            },
            early_stopped: false,
        });
    }

    let (pilot_seeds, rest_seeds) = deal_seeds.split_at(pilot_games);

    let baseline_pilot = run_fn(pilot_seeds, PolicyPlacement::AllRuleBased)?;
    let landlord_pilot = run_fn(pilot_seeds, PolicyPlacement::LandlordStrategic)?;
    let farmers_pilot = run_fn(pilot_seeds, PolicyPlacement::FarmersStrategic)?;
    let all_strategic_pilot = run_fn(pilot_seeds, PolicyPlacement::AllStrategic)?;

    let landlord_regression = baseline_pilot.landlord_win_rate - landlord_pilot.landlord_win_rate;
    let farmers_regression = baseline_pilot.farmer_win_rate - farmers_pilot.farmer_win_rate;

    if landlord_regression > options.early_stop_regression
        && farmers_regression > options.early_stop_regression
    {
        let landlord_delta = landlord_pilot.landlord_win_rate - baseline_pilot.landlord_win_rate;
        let farmers_delta = farmers_pilot.farmer_win_rate - baseline_pilot.farmer_win_rate;
        return Ok(RandomTournamentReport {
            schema_version: "2026-05-11".to_string(),
            deterministic: false,
            random_source,
            games: pilot_games,
            deal_seeds: pilot_seeds.to_vec(),
            strategy,
            runs: vec![
                baseline_pilot,
                landlord_pilot,
                farmers_pilot,
                all_strategic_pilot,
            ],
            conclusion: TournamentConclusion {
                significance_threshold,
                landlord_strategic_delta: landlord_delta,
                farmers_strategic_delta: farmers_delta,
                landlord_strategic_significant: false,
                farmers_strategic_significant: false,
            },
            early_stopped: true,
        });
    }

    let baseline_rest = run_fn(rest_seeds, PolicyPlacement::AllRuleBased)?;
    let landlord_rest = run_fn(rest_seeds, PolicyPlacement::LandlordStrategic)?;
    let farmers_rest = run_fn(rest_seeds, PolicyPlacement::FarmersStrategic)?;
    let all_strategic_rest = run_fn(rest_seeds, PolicyPlacement::AllStrategic)?;

    let baseline = merge_run_reports(&baseline_pilot, &baseline_rest);
    let landlord_strategic = merge_run_reports(&landlord_pilot, &landlord_rest);
    let farmers_strategic = merge_run_reports(&farmers_pilot, &farmers_rest);
    let all_strategic = merge_run_reports(&all_strategic_pilot, &all_strategic_rest);

    let landlord_strategic_delta =
        landlord_strategic.landlord_win_rate - baseline.landlord_win_rate;
    let farmers_strategic_delta = farmers_strategic.farmer_win_rate - baseline.farmer_win_rate;

    Ok(RandomTournamentReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: false,
        random_source,
        games,
        deal_seeds,
        strategy,
        runs: vec![
            baseline,
            landlord_strategic,
            farmers_strategic,
            all_strategic,
        ],
        conclusion: TournamentConclusion {
            significance_threshold,
            landlord_strategic_delta,
            farmers_strategic_delta,
            landlord_strategic_significant: landlord_strategic_delta >= significance_threshold,
            farmers_strategic_significant: farmers_strategic_delta >= significance_threshold,
        },
        early_stopped: false,
    })
}

fn random_source() -> u64 {
    generate_random_source()
}

pub fn generate_random_source() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (now.as_nanos() as u64) ^ ((now.as_nanos() >> 64) as u64)
}

fn random_deal_seeds(mut state: u64, games: usize) -> Vec<u64> {
    (0..games)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        })
        .collect()
}

pub fn run_deal(seed: u64, viewer: usize) -> Result<DealReport, ArenaError> {
    let deal = Deal::from_seed(seed, 3);
    let bottom_cards = card_strings(&deal.bottom_cards);
    let game = Game::new(deal, GameConfig::default())?;
    if viewer >= 3 {
        return Err(ArenaError::InvalidScenario(format!(
            "viewer {viewer} is outside the 3-player game"
        )));
    }

    let view = game.player_view(PlayerId(viewer));
    let players = view
        .hand_counts
        .iter()
        .enumerate()
        .map(|(id, hand_count)| DealPlayerReport {
            id,
            role: if id == 0 {
                "Landlord".to_string()
            } else {
                "Peasant".to_string()
            },
            relationship: format!("{:?}", view.relationships[id]),
            hand_count: *hand_count,
            visible_hand: if id == viewer {
                card_strings(&view.hand)
            } else {
                Vec::new()
            },
        })
        .collect();

    Ok(DealReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        viewer,
        bottom_cards,
        players,
    })
}

pub fn run_session(seed: u64, viewer: usize) -> Result<SessionReport, ArenaError> {
    run_session_after_steps(seed, viewer, 0, 1_000)
}

pub fn run_session_after_steps(
    seed: u64,
    viewer: usize,
    steps: usize,
    max_turns: usize,
) -> Result<SessionReport, ArenaError> {
    run_session_after_steps_with_config(
        seed,
        viewer,
        steps,
        max_turns,
        RuleBasedPolicyConfig::default(),
    )
}

pub fn run_session_after_steps_with_config(
    seed: u64,
    viewer: usize,
    steps: usize,
    max_turns: usize,
    policy_config: RuleBasedPolicyConfig,
) -> Result<SessionReport, ArenaError> {
    run_session_after_steps_with_landlord_policy(
        seed,
        viewer,
        steps,
        max_turns,
        policy_config,
        LandlordPolicy::RuleBased,
        RoleStrategyConfig::default(),
    )
}

pub fn run_session_after_steps_with_landlord_policy(
    seed: u64,
    viewer: usize,
    steps: usize,
    max_turns: usize,
    rule_config: RuleBasedPolicyConfig,
    landlord_policy: LandlordPolicy,
    strategy_config: RoleStrategyConfig,
) -> Result<SessionReport, ArenaError> {
    let deal = Deal::from_seed(seed, 3);
    let config = GameConfig {
        max_turns,
        ..GameConfig::default()
    };
    let mut game = Game::new(deal, config)?;
    let mut policies = strategic_policies(rule_config, landlord_policy, strategy_config);
    for _ in 0..steps {
        if game.finished() {
            break;
        }
        let player = game.current_player();
        game.step_current(policies[player.0].as_mut())?;
    }

    Ok(SessionReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        view: game_view_report(&game, seed, viewer)?,
        hint: hint_report(&game, seed, viewer, landlord_policy, &strategy_config)?,
    })
}

pub fn run_session_manual_step(
    seed: u64,
    viewer: usize,
    steps: usize,
    manual_cards: Option<Vec<Card>>,
    max_turns: usize,
    rule_config: RuleBasedPolicyConfig,
    landlord_policy: LandlordPolicy,
    strategy_config: RoleStrategyConfig,
) -> Result<SessionReport, ArenaError> {
    let deal = Deal::from_seed(seed, 3);
    let config = GameConfig {
        max_turns,
        ..GameConfig::default()
    };
    let mut game = Game::new(deal, config)?;
    let mut policies = strategic_policies(rule_config, landlord_policy, strategy_config);

    // Replay AI steps
    for _ in 0..steps {
        if game.finished() {
            break;
        }
        let player = game.current_player();
        game.step_current(policies[player.0].as_mut())?;
    }

    // Apply manual decision
    if !game.finished() {
        let decision = match manual_cards {
            Some(cards) => Decision::Play(cards),
            None => Decision::Pass,
        };
        let current = game.current_player();
        game.submit_decision(current, decision)?;
    }

    Ok(SessionReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        view: game_view_report(&game, seed, viewer)?,
        hint: hint_report(&game, seed, viewer, landlord_policy, &strategy_config)?,
    })
}

pub fn run_session_actions(
    seed: u64,
    viewer: usize,
    actions: &[SessionAction],
    max_turns: usize,
    rule_config: RuleBasedPolicyConfig,
    landlord_policy: LandlordPolicy,
    strategy_config: RoleStrategyConfig,
) -> Result<SessionReport, ArenaError> {
    let deal = Deal::from_seed(seed, 3);
    let config = GameConfig {
        max_turns,
        ..GameConfig::default()
    };
    let mut game = Game::new(deal, config)?;
    let mut policies = strategic_policies(rule_config, landlord_policy, strategy_config);

    for action in actions {
        if game.finished() {
            break;
        }
        apply_session_action(&mut game, &mut policies, action)?;
    }

    Ok(SessionReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        view: game_view_report(&game, seed, viewer)?,
        hint: hint_report(&game, seed, viewer, landlord_policy, &strategy_config)?,
    })
}

fn apply_session_action(
    game: &mut Game,
    policies: &mut [Box<dyn DecisionPolicy>],
    action: &SessionAction,
) -> Result<(), ArenaError> {
    match action {
        SessionAction::Auto => {
            let player = game.current_player();
            game.step_current(policies[player.0].as_mut())?;
        }
        SessionAction::Manual { cards } => {
            let decision = if cards.is_empty() {
                Decision::Pass
            } else {
                Decision::Play(parse_cards(cards)?)
            };
            let current = game.current_player();
            game.submit_decision(current, decision)?;
        }
    }
    Ok(())
}

fn strategic_policies(
    rule_config: RuleBasedPolicyConfig,
    landlord_policy: LandlordPolicy,
    strategy_config: RoleStrategyConfig,
) -> Vec<Box<dyn DecisionPolicy>> {
    let make_policy = || match landlord_policy {
        LandlordPolicy::RuleBased => {
            Box::new(RuleBasedPolicy::new(rule_config)) as Box<dyn DecisionPolicy>
        }
        LandlordPolicy::Strategic => Box::new(StrategicPolicy::from_role_configs(strategy_config)),
    };
    vec![make_policy(), make_policy(), make_policy()]
}

pub fn run_trace(seed: u64, max_turns: usize) -> Result<EpisodeReport, ArenaError> {
    run_trace_with_config(seed, max_turns, RuleBasedPolicyConfig::default())
}

pub fn run_trace_with_config(
    seed: u64,
    max_turns: usize,
    policy_config: RuleBasedPolicyConfig,
) -> Result<EpisodeReport, ArenaError> {
    run_trace_with_landlord_policy(seed, max_turns, policy_config, LandlordPolicy::RuleBased)
}

pub fn run_trace_with_landlord_policy(
    seed: u64,
    max_turns: usize,
    policy_config: RuleBasedPolicyConfig,
    landlord_policy: LandlordPolicy,
) -> Result<EpisodeReport, ArenaError> {
    let deal = Deal::from_seed(seed, 3);
    let initial_hands = deal.hands.iter().map(|hand| card_strings(hand)).collect();
    let bottom_cards = card_strings(&deal.bottom_cards);
    let config = GameConfig {
        max_turns,
        ..GameConfig::default()
    };
    let mut game = Game::new(deal, config)?;
    let mut policies = self_play_policies(landlord_policy, policy_config);
    let error = match game.run(&mut policies) {
        Ok(_) => None,
        Err(error) => Some(format!("{error:?}")),
    };

    Ok(EpisodeReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        policies: policy_reports_with_landlord(3, policy_config, landlord_policy),
        initial_hands,
        bottom_cards,
        winner: game.winner().map(|winner| winner.0),
        turns: game.history().len(),
        history: public_history(game.history()),
        error,
    })
}

fn run_legal_candidates_scenario(
    name: String,
    hand: Vec<String>,
    previous_play: Option<Vec<String>>,
    expect: LegalCandidatesExpect,
) -> Result<ScenarioReport, ArenaError> {
    let hand = parse_cards(&hand)?;
    let rules = BasicRules;
    let previous = previous_play
        .as_ref()
        .map(|cards| {
            let cards = parse_cards(cards)?;
            rules.classify(&cards).ok_or_else(|| {
                ArenaError::InvalidScenario("previous_play is not a legal hand".to_string())
            })
        })
        .transpose()?;
    let candidates = legal_candidates(&hand, previous.as_ref(), &rules);
    let candidate_cards: Vec<Vec<Card>> = candidates.into_iter().map(|hand| hand.cards).collect();

    let mut checks = Vec::new();
    for expected in expect.contains {
        let expected = parse_cards(&expected)?;
        let pass = contains_hand(&candidate_cards, &expected);
        checks.push(CheckReport {
            id: format!("contains:{}", cards_key(&expected)),
            pass,
            message: if pass {
                "expected hand is legal".to_string()
            } else {
                "expected hand was not found in legal candidates".to_string()
            },
        });
    }
    for forbidden in expect.excludes {
        let forbidden = parse_cards(&forbidden)?;
        let pass = !contains_hand(&candidate_cards, &forbidden);
        checks.push(CheckReport {
            id: format!("excludes:{}", cards_key(&forbidden)),
            pass,
            message: if pass {
                "forbidden hand is absent".to_string()
            } else {
                "forbidden hand was present in legal candidates".to_string()
            },
        });
    }

    Ok(report(
        name,
        "legal_candidates",
        checks,
        BTreeMap::from([("candidate_count".to_string(), candidate_cards.len() as u64)]),
        json!({
            "candidates": candidate_cards.iter().map(|cards| card_strings(cards)).collect::<Vec<_>>()
        }),
    ))
}

fn run_visibility_scenario(
    name: String,
    hands: Vec<Vec<String>>,
    viewer: usize,
    expect: VisibilityExpect,
) -> Result<ScenarioReport, ArenaError> {
    let hands = hands
        .iter()
        .map(|hand| parse_cards(hand))
        .collect::<Result<Vec<_>, _>>()?;
    let deal = Deal {
        bottom_cards: Vec::new(),
        hands,
    };
    let game = Game::new(deal, GameConfig::default())?;
    let view = game.player_view(PlayerId(viewer));
    let expected_hand = parse_cards(&expect.hand)?;
    let forbidden_cards = parse_cards(&expect.forbidden_cards)?;

    let checks = vec![
        CheckReport {
            id: "own_hand".to_string(),
            pass: normalize_cards(view.hand.clone()) == normalize_cards(expected_hand),
            message: "view hand matches expected player hand".to_string(),
        },
        CheckReport {
            id: "hand_counts".to_string(),
            pass: view.hand_counts == expect.hand_counts,
            message: "visible hand counts match expected counts".to_string(),
        },
        CheckReport {
            id: "hidden_cards_absent".to_string(),
            pass: forbidden_cards
                .iter()
                .all(|forbidden| !view.hand.contains(forbidden)),
            message: "forbidden hidden cards are absent from own hand view".to_string(),
        },
    ];

    Ok(report(
        name,
        "visibility",
        checks,
        BTreeMap::from([("viewer".to_string(), viewer as u64)]),
        json!({
            "hand": card_strings(&view.hand),
            "hand_counts": view.hand_counts
        }),
    ))
}

fn run_self_play_scenario(
    name: String,
    seed: u64,
    games: usize,
    max_turns: usize,
    expect: SelfPlayExpect,
) -> Result<ScenarioReport, ArenaError> {
    let seeded = run_seeded_games(seed, games, max_turns)?;
    let mut checks = vec![CheckReport {
        id: "all_games_completed".to_string(),
        pass: seeded.reports.iter().all(|report| report.error.is_none()),
        message: "all seeded games completed without engine errors".to_string(),
    }];

    if let Some(expected_wins) = expect.wins {
        checks.push(CheckReport {
            id: "wins".to_string(),
            pass: seeded.wins == expected_wins,
            message: format!(
                "expected wins {:?}, actual wins {:?}",
                expected_wins, seeded.wins
            ),
        });
    }

    Ok(report(
        name,
        "self_play",
        checks,
        BTreeMap::from([
            ("games".to_string(), seeded.games as u64),
            ("seed".to_string(), seeded.seed),
        ]),
        serde_json::to_value(seeded)?,
    ))
}

fn report(
    name: String,
    kind: &str,
    checks: Vec<CheckReport>,
    metrics: BTreeMap<String, u64>,
    details: serde_json::Value,
) -> ScenarioReport {
    let pass = checks.iter().all(|check| check.pass);
    ScenarioReport {
        schema_version: "2026-05-11".to_string(),
        name,
        kind: kind.to_string(),
        deterministic: true,
        pass,
        checks,
        metrics,
        details,
    }
}

fn rule_based_policies(
    players: usize,
    config: RuleBasedPolicyConfig,
) -> Vec<Box<dyn DecisionPolicy>> {
    (0..players)
        .map(|_| Box::new(RuleBasedPolicy::new(config)) as Box<dyn DecisionPolicy>)
        .collect()
}

fn self_play_policies(
    landlord_policy: LandlordPolicy,
    config: RuleBasedPolicyConfig,
) -> Vec<Box<dyn DecisionPolicy>> {
    let landlord: Box<dyn DecisionPolicy> = match landlord_policy {
        LandlordPolicy::RuleBased => Box::new(RuleBasedPolicy::new(config)),
        LandlordPolicy::Strategic => Box::new(StrategicPolicy::new(config)),
    };

    vec![
        landlord,
        Box::new(RuleBasedPolicy::new(config)),
        Box::new(RuleBasedPolicy::new(config)),
    ]
}

fn placement_policies(
    placement: PolicyPlacement,
    rule_config: RuleBasedPolicyConfig,
    strategy_config: RoleStrategyConfig,
    baseline_config: Option<RoleStrategyConfig>,
) -> Vec<Box<dyn DecisionPolicy>> {
    let baseline = baseline_config.unwrap_or(strategy_config);
    match placement {
        PolicyPlacement::AllRuleBased => rule_based_policies(3, rule_config),
        PolicyPlacement::LandlordStrategic => vec![
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
            Box::new(RuleBasedPolicy::new(rule_config)),
            Box::new(RuleBasedPolicy::new(rule_config)),
        ],
        PolicyPlacement::FarmersStrategic => vec![
            Box::new(RuleBasedPolicy::new(rule_config)),
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
        ],
        PolicyPlacement::AllStrategic => vec![
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
        ],
        PolicyPlacement::LandlordNewFarmersOld => vec![
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
            Box::new(StrategicPolicy::from_role_configs(baseline)),
            Box::new(StrategicPolicy::from_role_configs(baseline)),
        ],
        PolicyPlacement::LandlordOldFarmersNew => vec![
            Box::new(StrategicPolicy::from_role_configs(baseline)),
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
            Box::new(StrategicPolicy::from_role_configs(strategy_config)),
        ],
    }
}

fn policy_reports_with_landlord(
    players: usize,
    config: RuleBasedPolicyConfig,
    landlord_policy: LandlordPolicy,
) -> Vec<PolicyReport> {
    (0..players)
        .map(|player| PolicyReport {
            player,
            name: if player == 0 {
                landlord_policy.name()
            } else {
                "rule_based"
            }
            .to_string(),
            avoid_power_hands: config.avoid_power_hands,
        })
        .collect()
}

fn game_view_report(game: &Game, seed: u64, viewer: usize) -> Result<GameViewReport, ArenaError> {
    let view = game.player_view_checked(PlayerId(viewer))?;
    let players = view
        .hand_counts
        .iter()
        .enumerate()
        .map(|(id, hand_count)| PlayerPublicReport {
            id,
            role: role_name(id),
            relationship: format!("{:?}", view.relationships[id]),
            hand_count: *hand_count,
        })
        .collect();

    Ok(GameViewReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        viewer,
        current_player: game.current_player().0,
        winner: match game.status() {
            GameStatus::Running => None,
            GameStatus::Finished(outcome) => Some(outcome.winner.0),
        },
        visible_hand: card_strings(&view.hand),
        hand_counts: view.hand_counts,
        players,
        relationships: view
            .relationships
            .iter()
            .map(|relationship| format!("{relationship:?}"))
            .collect(),
        history: public_history(&view.history),
        previous_player: game.previous_player().map(|player| player.0),
        previous_play: view.previous_play.as_ref().map(hand_report),
        bottom_cards: card_strings(game.bottom_cards()),
    })
}

fn hint_report(
    game: &Game,
    seed: u64,
    viewer: usize,
    landlord_policy: LandlordPolicy,
    strategy_config: &RoleStrategyConfig,
) -> Result<HintReport, ArenaError> {
    let view = game.player_view_checked(PlayerId(viewer))?;
    let hints = legal_candidates(&view.hand, view.previous_play.as_ref(), game.rules());
    let legal_hints: Vec<Vec<String>> =
        hints.iter().map(|hand| card_strings(&hand.cards)).collect();
    let recommended = if game.current_player() == PlayerId(viewer) {
        let mut policy: Box<dyn DecisionPolicy> = match landlord_policy {
            LandlordPolicy::RuleBased => Box::new(RuleBasedPolicy::default()),
            LandlordPolicy::Strategic => {
                Box::new(StrategicPolicy::from_role_configs(strategy_config.clone()))
            }
        };
        match policy.decide(&view, game.rules()) {
            Decision::Play(cards) => Some(card_strings(&cards)),
            Decision::Pass => Some(Vec::new()),
        }
    } else {
        None
    };

    Ok(HintReport {
        schema_version: "2026-05-11".to_string(),
        deterministic: true,
        seed,
        viewer,
        current_player: game.current_player().0,
        legal_hints,
        recommended,
    })
}

fn public_history(history: &[TurnRecord]) -> Vec<TurnReport> {
    history
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let cards = match &record.decision {
                crate::decision::Decision::Pass => Vec::new(),
                crate::decision::Decision::Play(cards) => card_strings(cards),
            };
            TurnReport {
                turn: index + 1,
                player: record.player.0,
                decision: match record.decision {
                    crate::decision::Decision::Pass => "Pass".to_string(),
                    crate::decision::Decision::Play(_) => "Play".to_string(),
                },
                cards,
                accepted_hand: record.accepted_hand.as_ref().map(hand_report),
            }
        })
        .collect()
}

fn hand_report(hand: &ClassifiedHand) -> HandReport {
    HandReport {
        kind: format!("{:?}", hand.kind),
        cards: card_strings(&hand.cards),
    }
}

fn role_name(id: usize) -> String {
    if id == 0 {
        "Landlord".to_string()
    } else {
        "Peasant".to_string()
    }
}

fn parse_cards(values: &[String]) -> Result<Vec<Card>, ArenaError> {
    values
        .iter()
        .map(|value| Card::from_str(value).map_err(ArenaError::InvalidCard))
        .collect::<Result<Vec<_>, _>>()
        .map(normalize_cards)
}

fn contains_hand(candidates: &[Vec<Card>], expected: &[Card]) -> bool {
    let expected = normalize_cards(expected.to_vec());
    candidates
        .iter()
        .any(|candidate| normalize_cards(candidate.clone()) == expected)
}

fn normalize_cards(mut cards: Vec<Card>) -> Vec<Card> {
    cards.sort();
    cards
}

fn cards_key(cards: &[Card]) -> String {
    card_strings(cards).join(",")
}

fn card_strings(cards: &[Card]) -> Vec<String> {
    normalize_cards(cards.to_vec())
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn history_hash(history: &[TurnRecord]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for record in history {
        hash_bytes(&mut hash, record.player.0.to_string().as_bytes());
        hash_bytes(&mut hash, format!("{:?}", record.decision).as_bytes());
        hash_bytes(&mut hash, format!("{:?}", record.accepted_hand).as_bytes());
    }
    format!("{hash:016x}")
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

#[cfg(test)]
mod tests {
    use crate::arena::{
        run_deal, run_scenario_str, run_seeded_games_with_landlord_policy, run_session,
        run_session_actions, run_session_after_steps, run_trace, run_trace_with_config, ArenaError,
        LandlordPolicy, SessionAction,
    };
    use crate::cards::{Card, Rank, Suit};
    use crate::decision::{RoleStrategyConfig, RuleBasedPolicyConfig};
    use crate::engine::Deal;
    use crate::rules::{BasicRules, RuleSet};
    use std::str::FromStr;

    #[test]
    fn parses_compact_card_codes() {
        assert_eq!(
            Card::from_str("10h").unwrap(),
            Card::suited(Rank::Ten, Suit::Hearts)
        );
        assert_eq!(Card::from_str("BJ").unwrap(), Card::joker(Rank::BlackJoker));
        assert!(Card::from_str("1Z").is_err());
    }

    #[test]
    fn legal_candidate_scenario_checks_required_and_forbidden_plays() {
        let report = run_scenario_str(
            r#"{
              "name": "bomb_beats_pair",
              "kind": "legal_candidates",
              "hand": ["4C", "4D", "4H", "4S", "3C"],
              "previous_play": ["7C", "7D"],
              "expect": {
                "contains": [["4C", "4D", "4H", "4S"]],
                "excludes": [["3C"]]
              }
            }"#,
        )
        .unwrap();

        assert!(report.pass, "{report:#?}");
        assert_eq!(report.name, "bomb_beats_pair");
    }

    #[test]
    fn visibility_scenario_rejects_hidden_card_leaks() {
        let report = run_scenario_str(
            r#"{
              "name": "visibility_no_leak",
              "kind": "visibility",
              "hands": [["3C", "6C"], ["4C", "7C"], ["5C", "8C"]],
              "viewer": 1,
              "expect": {
                "hand": ["4C", "7C"],
                "hand_counts": [2, 2, 2],
                "forbidden_cards": ["3C", "5C"]
              }
            }"#,
        )
        .unwrap();

        assert!(report.pass, "{report:#?}");
        assert_eq!(report.checks.len(), 3);
    }

    #[test]
    fn self_play_scenario_reports_deterministic_wins() {
        let report = run_scenario_str(
            r#"{
              "name": "seeded_self_play_smoke",
              "kind": "self_play",
              "seed": 42,
              "games": 2,
              "expect": {
                "wins": [1, 1, 0]
              }
            }"#,
        )
        .unwrap();

        assert!(report.pass, "{report:#?}");
        assert_eq!(report.metrics.get("games").copied(), Some(2));
    }

    #[test]
    fn seeded_games_can_install_strategic_landlord_policy() {
        let report =
            run_seeded_games_with_landlord_policy(42, 2, 1_000, LandlordPolicy::Strategic).unwrap();

        assert_eq!(report.landlord_policy, "strategic");
        assert_eq!(report.reports.len(), 2);
        assert!(report.reports.iter().all(|game| game.error.is_none()));
    }

    #[test]
    fn random_tournament_compares_all_policy_placements() {
        let report =
            super::run_random_tournament(2, 1_000, RoleStrategyConfig::default(), 0.10).unwrap();

        assert!(!report.deterministic);
        assert_eq!(report.deal_seeds.len(), 2);
        assert_eq!(
            report
                .runs
                .iter()
                .map(|run| run.placement.as_str())
                .collect::<Vec<_>>(),
            [
                "all_rule_based",
                "landlord_strategic",
                "farmers_strategic",
                "all_strategic"
            ]
        );
        assert!(report
            .runs
            .iter()
            .all(|run| run.reports.iter().all(|game| game.error.is_none())));
    }

    #[test]
    fn scenario_returns_structured_errors_for_invalid_cards() {
        let error = run_scenario_str(
            r#"{
              "name": "bad_card",
              "kind": "legal_candidates",
              "hand": ["ZZ"],
              "expect": {}
            }"#,
        )
        .unwrap_err();

        assert!(matches!(error, ArenaError::InvalidCard(_)));
    }

    #[test]
    fn deal_report_exposes_only_viewer_hand() {
        let report = run_deal(42, 0).unwrap();

        assert_eq!(report.seed, 42);
        assert_eq!(report.viewer, 0);
        assert_eq!(report.bottom_cards.len(), 3);
        assert_eq!(report.players[0].role, "Landlord");
        assert_eq!(report.players[1].role, "Peasant");
        assert_eq!(report.players[2].role, "Peasant");
        assert_eq!(
            report
                .players
                .iter()
                .map(|player| player.hand_count)
                .collect::<Vec<_>>(),
            [20, 17, 17]
        );
        assert_eq!(report.players[0].visible_hand.len(), 20);
        for bottom_card in &report.bottom_cards {
            assert!(report.players[0].visible_hand.contains(bottom_card));
        }
        assert!(report.players[1].visible_hand.is_empty());
        assert!(report.players[2].visible_hand.is_empty());
    }

    #[test]
    fn deal_report_rejects_invalid_viewer() {
        let error = run_deal(42, 3).unwrap_err();

        assert!(matches!(error, ArenaError::InvalidScenario(_)));
    }

    #[test]
    fn session_view_and_hint_do_not_leak_other_hands() {
        let seed = 42;
        let viewer = 1;
        let deal = Deal::from_seed(seed, 3);
        let report = run_session(seed, viewer).unwrap();
        let hidden_cards = deal
            .hands
            .iter()
            .enumerate()
            .filter(|(id, _)| *id != viewer)
            .flat_map(|(_, hand)| hand.iter().map(ToString::to_string))
            .collect::<Vec<_>>();

        assert_eq!(report.view.viewer, viewer);
        assert_eq!(report.view.visible_hand.len(), deal.hands[viewer].len());
        for hidden in hidden_cards {
            assert!(!report.view.visible_hand.contains(&hidden));
            assert!(report
                .hint
                .legal_hints
                .iter()
                .all(|hint| !hint.contains(&hidden)));
        }
        assert_eq!(report.view.hand_counts, [20, 17, 17]);
        assert_eq!(report.view.bottom_cards.len(), 3);
    }

    #[test]
    fn trace_report_contains_trusted_initial_hands() {
        let seed = 42;
        let deal = Deal::from_seed(seed, 3);
        let report = run_trace(seed, 1_000).unwrap();

        assert_eq!(report.seed, seed);
        assert_eq!(report.initial_hands.len(), 3);
        assert_eq!(
            report
                .initial_hands
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [20, 17, 17]
        );
        assert_eq!(report.initial_hands[0], super::card_strings(&deal.hands[0]));
        assert_eq!(report.bottom_cards, super::card_strings(&deal.bottom_cards));
        assert!(report.error.is_none(), "{report:#?}");
        assert!(!report.history.is_empty());
    }

    #[test]
    fn trace_report_records_policy_configuration() {
        let report = run_trace_with_config(
            42,
            1_000,
            RuleBasedPolicyConfig {
                avoid_power_hands: false,
            },
        )
        .unwrap();

        assert_eq!(report.policies.len(), 3);
        assert!(report
            .policies
            .iter()
            .all(|policy| policy.name == "rule_based" && !policy.avoid_power_hands));
    }

    #[test]
    fn trace_history_never_contains_smaller_response() {
        let rules = BasicRules;
        for seed in [42, 43, 44, 45, 46] {
            let report = run_trace(seed, 1_000).unwrap();
            let mut previous = None;
            let mut passes_since_play = 0usize;

            for turn in &report.history {
                match turn.decision.as_str() {
                    "Pass" => {
                        passes_since_play += 1;
                        if passes_since_play >= 2 {
                            previous = None;
                            passes_since_play = 0;
                        }
                    }
                    "Play" => {
                        let cards = super::parse_cards(&turn.cards).unwrap();
                        let classified = rules.classify(&cards).unwrap();
                        assert!(
                            rules.can_play_over(&classified, previous.as_ref()),
                            "seed {seed} turn {} played {:?} over {:?}",
                            turn.turn,
                            classified,
                            previous
                        );
                        previous = Some(classified);
                        passes_since_play = 0;
                    }
                    other => panic!("unexpected decision: {other}"),
                }
            }
        }
    }

    #[test]
    fn stepped_session_uses_engine_visibility_after_pass_reset() {
        let report = run_session_after_steps(42, 2, 14, 1_000).unwrap();

        assert_eq!(report.view.current_player, 2);
        assert_eq!(report.view.previous_player, Some(2));
        assert!(report.view.previous_play.is_none());
        assert_eq!(report.hint.recommended, Some(vec!["3C".to_string()]));
    }

    #[test]
    fn action_session_preserves_manual_play_before_later_auto_steps() {
        let report = run_session_actions(
            42,
            0,
            &[
                SessionAction::Manual {
                    cards: vec!["3D".to_string()],
                },
                SessionAction::Auto,
            ],
            1_000,
            RuleBasedPolicyConfig::default(),
            LandlordPolicy::Strategic,
            RoleStrategyConfig::default(),
        )
        .unwrap();

        assert!(report.view.history.len() >= 2);
        assert_eq!(report.view.history[0].player, 0);
        assert_eq!(report.view.history[0].decision, "Play");
        assert_eq!(report.view.history[0].cards, ["3D"]);
    }
}
