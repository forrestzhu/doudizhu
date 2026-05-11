use doudizhu::harness::{run_scenario_file, run_seeded_games};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games = read_usize_arg(&args, "--games").unwrap_or(1);
    let seed = read_u64_arg(&args, "--seed").unwrap_or(1);
    let max_turns = read_usize_arg(&args, "--max-turns").unwrap_or(1_000);
    let format = read_arg(&args, "--format").unwrap_or("text");

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

    let report = run_seeded_games(seed, games, max_turns).unwrap_or_else(|error| {
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
        "summary games={} wins={:?} avg_turns={:.2}",
        report.games, report.wins, report.avg_turns
    );
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

fn read_arg<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}
