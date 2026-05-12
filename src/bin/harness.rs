use doudizhu::harness::run_session_after_steps_with_config;
use doudizhu::harness::{
    run_deal, run_random_tournament, run_scenario_file, run_seeded_games_with_landlord_policy,
    run_trace_with_landlord_policy, LandlordPolicy,
};
use doudizhu::{RuleBasedPolicyConfig, StrategicPolicyConfig};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games = read_usize_arg(&args, "--games").unwrap_or(1);
    let seed = read_u64_arg(&args, "--seed").unwrap_or(1);
    let max_turns = read_usize_arg(&args, "--max-turns").unwrap_or(1_000);
    let format = read_arg(&args, "--format").unwrap_or("text");
    let significance_threshold = read_f64_arg(&args, "--significance").unwrap_or(0.10);
    let strategy_config = read_arg(&args, "--strategy-file")
        .map(read_strategy_config)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        })
        .unwrap_or_default();
    let policy_config = RuleBasedPolicyConfig {
        avoid_power_hands: !has_flag(&args, "--allow-power"),
    };
    let landlord_policy = read_arg(&args, "--landlord-policy")
        .map(parse_landlord_policy)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        })
        .unwrap_or(LandlordPolicy::RuleBased);

    if has_flag(&args, "--random-tournament") {
        let report =
            run_random_tournament(games, max_turns, strategy_config, significance_threshold)
                .unwrap_or_else(|error| {
                    eprintln!("random tournament error={error:?}");
                    std::process::exit(1);
                });

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!(
                "random_tournament games={} random_source={} significance={:.2}",
                report.games, report.random_source, report.conclusion.significance_threshold
            );
            for run in &report.runs {
                println!(
                    "placement={} wins={:?} landlord_win_rate={:.2} farmer_win_rate={:.2} avg_turns={:.2}",
                    run.placement,
                    run.wins,
                    run.landlord_win_rate,
                    run.farmer_win_rate,
                    run.avg_turns
                );
            }
            println!(
                "conclusion landlord_delta={:.2} landlord_significant={} farmers_delta={:.2} farmers_significant={}",
                report.conclusion.landlord_strategic_delta,
                report.conclusion.landlord_strategic_significant,
                report.conclusion.farmers_strategic_delta,
                report.conclusion.farmers_strategic_significant
            );
        }

        return;
    }

    if has_flag(&args, "--deal") {
        let viewer = read_usize_arg(&args, "--viewer").unwrap_or(0);
        let report = run_deal(seed, viewer).unwrap_or_else(|error| {
            eprintln!("deal seed={seed} viewer={viewer} error={error:?}");
            std::process::exit(1);
        });

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            print_deal_report(&report);
        }

        return;
    }

    if has_flag(&args, "--session") {
        let viewer = read_usize_arg(&args, "--viewer").unwrap_or(0);
        let steps = read_usize_arg(&args, "--steps").unwrap_or(0);
        let report =
            run_session_after_steps_with_config(seed, viewer, steps, max_turns, policy_config)
                .unwrap_or_else(|error| {
                    eprintln!("session seed={seed} viewer={viewer} error={error:?}");
                    std::process::exit(1);
                });

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!(
                "session seed={} viewer={} current_player={} visible_hand={:?}",
                report.seed,
                report.view.viewer,
                report.view.current_player,
                report.view.visible_hand
            );
        }

        return;
    }

    if has_flag(&args, "--trace") {
        let report =
            run_trace_with_landlord_policy(seed, max_turns, policy_config, landlord_policy)
                .unwrap_or_else(|error| {
                    eprintln!("trace seed={seed} error={error:?}");
                    std::process::exit(1);
                });

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            std::process::exit(if report.error.is_none() { 0 } else { 1 });
        } else {
            println!(
                "trace seed={} winner={:?} turns={} error={:?}",
                report.seed, report.winner, report.turns, report.error
            );
            if report.error.is_some() {
                std::process::exit(1);
            }
        }

        return;
    }

    if let Some(scenario_path) = read_arg(&args, "--scenario") {
        let report = run_scenario_file(Path::new(scenario_path)).unwrap_or_else(|error| {
            eprintln!("scenario={scenario_path} error={error:?}");
            std::process::exit(1);
        });

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            print_scenario_report(&report);
        }

        std::process::exit(if report.pass { 0 } else { 1 });
    }

    let report = run_seeded_games_with_landlord_policy(seed, games, max_turns, landlord_policy)
        .unwrap_or_else(|error| {
            eprintln!("error={error:?}");
            std::process::exit(1);
        });

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        std::process::exit(if report.reports.iter().all(|game| game.error.is_none()) {
            0
        } else {
            1
        });
    }

    for game in &report.reports {
        if let Some(error) = &game.error {
            eprintln!("game={} seed={} error={error}", game.game, game.seed);
            std::process::exit(1);
        }
        println!(
            "game={} seed={} winner={} turns={} history_hash={}",
            game.game,
            game.seed,
            game.winner.expect("completed game has winner"),
            game.turns,
            game.history_hash
        );
    }

    println!(
        "summary games={} landlord_policy={} wins={:?} avg_turns={:.2}",
        report.games, report.landlord_policy, report.wins, report.avg_turns
    );
}

fn parse_landlord_policy(value: &str) -> Result<LandlordPolicy, String> {
    match value {
        "rule-based" | "rule_based" => Ok(LandlordPolicy::RuleBased),
        "strategic" => Ok(LandlordPolicy::Strategic),
        _ => Err(format!(
            "unsupported landlord policy: {value}; expected rule-based or strategic"
        )),
    }
}

fn print_deal_report(report: &doudizhu::harness::DealReport) {
    println!("deal seed={} viewer={}", report.seed, report.viewer);
    println!("bottom_cards={:?}", report.bottom_cards);
    for player in &report.players {
        println!(
            "player={} role={} relationship={} hand_count={} visible_hand={:?}",
            player.id, player.role, player.relationship, player.hand_count, player.visible_hand
        );
    }
}

fn print_scenario_report(report: &doudizhu::harness::ScenarioReport) {
    println!(
        "scenario={} kind={} pass={}",
        report.name, report.kind, report.pass
    );
    for check in &report.checks {
        println!("check={} pass={} {}", check.id, check.pass, check.message);
    }
}

fn read_usize_arg(args: &[String], flag: &str) -> Option<usize> {
    read_arg(args, flag).and_then(|value| value.parse().ok())
}

fn read_u64_arg(args: &[String], flag: &str) -> Option<u64> {
    read_arg(args, flag).and_then(|value| value.parse().ok())
}

fn read_f64_arg(args: &[String], flag: &str) -> Option<f64> {
    read_arg(args, flag).and_then(|value| value.parse().ok())
}

fn read_arg<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn read_strategy_config(path: &str) -> Result<StrategicPolicyConfig, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read strategy file {path}: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse strategy file {path}: {error}"))
}
