#!/usr/bin/env python3
"""Hill-climbing optimizer for Dou Di Zhu strategy weights.

Systematically tests ±1 changes to each parameter for each role,
running arena tournaments to find improvements.
"""
import json
import subprocess
import sys
import os
import copy
from pathlib import Path

ARENA_BIN = "target/release/arena"
GAMES = 200
RANDOM_SOURCE = 1778665785420242000
BASELINE_FILE = "strategies/roles_v1.json"
STRATEGY_DIR = "strategies"

# Integer parameters to tune (name, min, max)
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

ROLES = ["landlord", "sender", "blocker"]


def run_tournament(strategy_file, games=GAMES):
    """Run arena tournament and return (landlord_wr, farmer_wr, avg_turns)."""
    cmd = [
        ARENA_BIN,
        "--random-tournament",
        "--games", str(games),
        "--random-source", str(RANDOM_SOURCE),
        "--strategy-file", strategy_file,
        "--format", "json",
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=os.getcwd())
    if result.returncode != 0:
        print(f"ERROR: {result.stderr}", file=sys.stderr)
        return None
    try:
        data = json.loads(result.stdout)
        for run in data["runs"]:
            if run["placement"] == "all_strategic":
                return (run["landlord_win_rate"], run["farmer_win_rate"], run["avg_turns"])
    except (json.JSONDecodeError, KeyError) as e:
        print(f"Parse error: {e}", file=sys.stderr)
        return None
    return None


def write_config(config, path):
    with open(path, "w") as f:
        json.dump(config, f, indent=2)
        f.write("\n")


def main():
    # Load baseline
    with open(BASELINE_FILE) as f:
        baseline = json.load(f)

    print(f"=== Hill Climbing Optimizer ===")
    print(f"Games per experiment: {GAMES}")
    print(f"Random source: {RANDOM_SOURCE}")
    print()

    # Run baseline
    print("Running baseline...")
    baseline_result = run_tournament(BASELINE_FILE)
    if baseline_result is None:
        print("Failed to run baseline!")
        sys.exit(1)
    baseline_ll, baseline_farmer, baseline_turns = baseline_result
    print(f"Baseline: landlord={baseline_ll:.2f} farmer={baseline_farmer:.2f} turns={baseline_turns:.2f}")
    print()

    # Track best improvements
    improvements = []
    regressions = []

    for role in ROLES:
        for param, (min_val, max_val) in INT_PARAMS.items():
            current_val = baseline[role][param]
            for delta in [-1, 1]:
                new_val = current_val + delta
                if new_val < min_val or new_val > max_val:
                    continue

                # Create variant config
                variant = copy.deepcopy(baseline)
                variant[role][param] = new_val

                exp_name = f"{role}.{param}={new_val}_(was_{current_val})"
                exp_file = os.path.join(STRATEGY_DIR, f"_opt_{role}_{param}_{new_val}.json")
                write_config(variant, exp_file)

                print(f"Testing {exp_name}...", end=" ", flush=True)
                result = run_tournament(exp_file)
                if result is None:
                    print("FAILED")
                    continue

                ll, farmer, turns = result
                ll_delta = ll - baseline_ll
                farmer_delta = farmer - baseline_farmer

                marker = ""
                if ll_delta > 0.02:
                    marker = " *** IMPROVED"
                    improvements.append((exp_name, ll_delta, farmer_delta))
                elif ll_delta < -0.02:
                    marker = " (regression)"
                    regressions.append((exp_name, ll_delta, farmer_delta))

                print(f"ll={ll:.2f} farmer={farmer:.2f} ll_delta={ll_delta:+.2f}{marker}")

                # Clean up experiment file
                os.remove(exp_file)

    print()
    print("=== Summary ===")
    if improvements:
        print(f"\nImprovements ({len(improvements)}):")
        for name, ll_d, f_d in sorted(improvements, key=lambda x: -x[1]):
            print(f"  {name}: ll_delta={ll_d:+.2f} farmer_delta={f_d:+.2f}")
    else:
        print("\nNo improvements found.")

    if regressions:
        print(f"\nRegressions ({len(regressions)}):")
        for name, ll_d, f_d in sorted(regressions, key=lambda x: x[1]):
            print(f"  {name}: ll_delta={ll_d:+.2f} farmer_delta={f_d:+.2f}")


if __name__ == "__main__":
    main()
