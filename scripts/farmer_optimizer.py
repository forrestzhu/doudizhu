#!/usr/bin/env python3
"""Farmer-specific optimizer using farmers_strategic placement.

Evaluates sender/blocker parameter changes by measuring farmer_win_rate
in farmers_strategic mode (strategic farmers vs rule-based landlord).
"""
import json
import subprocess
import sys
import os
import copy

ARENA_BIN = "target/release/arena"
GAMES = 200
RANDOM_SOURCE = 1778665785420242000
BASELINE_FILE = "strategies/roles_v1.json"
STRATEGY_DIR = "strategies"

INT_PARAMS = {
    "endgame_search_limit": (0, 20),
    "power_cost_normal": (1, 8),
    "power_cost_threat": (0, 4),
    "lead_tempo_plan_weight": (0, 5),
    "stranded_risk_weight": (0, 5),
    "opponent_urgency_weight": (0, 5),
    "hand_control_weight": (0, 5),
    "farmer_cooperation_weight": (0, 5),
    "decomposition_weight": (0, 5),
    "pass_value_weight": (0, 5),
    "bomb_control_bonus": (0, 10),
    "trump_conservation_weight": (0, 5),
    "opening_resilience_weight": (0, 5),
    "endgame_pair_preserve": (0, 5),
}

ROLES = ["sender", "blocker"]


def run_tournament(strategy_file):
    """Run arena tournament and return farmer_win_rate from farmers_strategic."""
    cmd = [
        ARENA_BIN, "--random-tournament",
        "--games", str(GAMES),
        "--random-source", str(RANDOM_SOURCE),
        "--strategy-file", strategy_file,
        "--format", "json",
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=os.getcwd())
    if result.returncode != 0:
        return None
    try:
        data = json.loads(result.stdout)
        for run in data["runs"]:
            if run["placement"] == "farmers_strategic":
                return run["farmer_win_rate"]
    except (json.JSONDecodeError, KeyError):
        return None
    return None


def write_config(config, path):
    with open(path, "w") as f:
        json.dump(config, f, indent=2)
        f.write("\n")


def main():
    with open(BASELINE_FILE) as f:
        baseline = json.load(f)

    print("=== Farmer-Specific Optimizer ===")
    print(f"Games: {GAMES}, Seed: {RANDOM_SOURCE}")
    print(f"Metric: farmers_strategic farmer_win_rate")
    print()

    print("Running baseline...")
    baseline_fwr = run_tournament(BASELINE_FILE)
    if baseline_fwr is None:
        print("FAILED baseline!")
        sys.exit(1)
    print(f"Baseline farmer_win_rate: {baseline_fwr:.2f}")
    print()

    improvements = []
    regressions = []

    for role in ROLES:
        for param, (min_val, max_val) in INT_PARAMS.items():
            current_val = baseline[role][param]
            for delta in [-1, 1]:
                new_val = current_val + delta
                if new_val < min_val or new_val > max_val:
                    continue

                variant = copy.deepcopy(baseline)
                variant[role][param] = new_val

                exp_name = f"{role}.{param}={new_val}_(was_{current_val})"
                exp_file = os.path.join(STRATEGY_DIR, f"_fopt_{role}_{param}_{new_val}.json")
                write_config(variant, exp_file)

                print(f"Testing {exp_name}...", end=" ", flush=True)
                fwr = run_tournament(exp_file)
                if fwr is None:
                    print("FAILED")
                    continue

                delta_val = fwr - baseline_fwr
                marker = ""
                if delta_val > 0.02:
                    marker = " *** IMPROVED"
                    improvements.append((exp_name, delta_val))
                elif delta_val < -0.02:
                    marker = " (regression)"
                    regressions.append((exp_name, delta_val))

                print(f"fwr={fwr:.2f} delta={delta_val:+.2f}{marker}")
                os.remove(exp_file)

    print()
    print("=== Summary ===")
    if improvements:
        print(f"\nImprovements ({len(improvements)}):")
        for name, d in sorted(improvements, key=lambda x: -x[1]):
            print(f"  {name}: +{d:.2f}")
    else:
        print("\nNo farmer improvements found.")

    if regressions:
        print(f"\nRegressions ({len(regressions)}):")
        for name, d in sorted(regressions, key=lambda x: x[1]):
            print(f"  {name}: {d:+.2f}")


if __name__ == "__main__":
    main()
