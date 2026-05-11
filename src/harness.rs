use crate::cards::Card;
use crate::decision::{legal_candidates, DecisionPolicy, LowestLegalPolicy};
use crate::engine::{Deal, Game, GameConfig, GameError, PlayerId, TurnRecord};
use crate::rules::{BasicRules, RuleSet};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug)]
pub enum HarnessError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidCard(String),
    InvalidScenario(String),
    Game(GameError),
}

impl From<std::io::Error> for HarnessError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for HarnessError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<GameError> for HarnessError {
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
    pub wins: Vec<usize>,
    pub avg_turns: f64,
    pub reports: Vec<SeededGameReport>,
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

pub fn run_scenario_file(path: &Path) -> Result<ScenarioReport, HarnessError> {
    let contents = std::fs::read_to_string(path)?;
    run_scenario_str(&contents)
}

pub fn run_scenario_str(contents: &str) -> Result<ScenarioReport, HarnessError> {
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
) -> Result<SeededRunReport, HarnessError> {
    if games == 0 {
        return Err(HarnessError::InvalidScenario(
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
        let mut policies = lowest_legal_policies(3);

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
        wins,
        avg_turns: total_turns as f64 / games as f64,
        reports,
    })
}

fn run_legal_candidates_scenario(
    name: String,
    hand: Vec<String>,
    previous_play: Option<Vec<String>>,
    expect: LegalCandidatesExpect,
) -> Result<ScenarioReport, HarnessError> {
    let hand = parse_cards(&hand)?;
    let rules = BasicRules;
    let previous = previous_play
        .as_ref()
        .map(|cards| {
            let cards = parse_cards(cards)?;
            rules.classify(&cards).ok_or_else(|| {
                HarnessError::InvalidScenario("previous_play is not a legal hand".to_string())
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
) -> Result<ScenarioReport, HarnessError> {
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
) -> Result<ScenarioReport, HarnessError> {
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

fn lowest_legal_policies(players: usize) -> Vec<Box<dyn DecisionPolicy>> {
    (0..players)
        .map(|_| Box::<LowestLegalPolicy>::default() as Box<dyn DecisionPolicy>)
        .collect()
}

fn parse_cards(values: &[String]) -> Result<Vec<Card>, HarnessError> {
    values
        .iter()
        .map(|value| Card::from_str(value).map_err(HarnessError::InvalidCard))
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
    use crate::cards::{Card, Rank, Suit};
    use crate::harness::{run_scenario_str, HarnessError};
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

        assert!(matches!(error, HarnessError::InvalidCard(_)));
    }
}
