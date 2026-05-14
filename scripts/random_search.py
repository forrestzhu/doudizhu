#!/usr/bin/env python3
"""Random parameter search for Dou Di Zhu strategy.
Tests random perturbations of multiple parameters simultaneously.
"""
import json
import subprocess
import sys
import os
import copy
import random

ARENA_BIN = "target/release/arena"
GAMES = 300
RANDOM_SOURCE = 1778726250286786000
BASELINE_FILE = "strategies/roles_v1.json"
STRATEGY_DIR = "strategies"
NUM_TRIALS = 30

INT_PARAMS = {
    "lead_tempo_plan_weight": (0, 5),
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


def run_tournament(strategy_file):
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
        results = {}
        for run in data["runs"]:
            results[run["placement"]] = run["landlord_win_rate"]
        return results
    except (json.JSONDecodeError, KeyError):
        return None


def write_config(config, path):
    with open(path, "w") as f:
        json.dump(config, f, indent=2)
        f.write("\n")


def main():
    with open(BASELINE_FILE) as f:
        baseline = json.load(f)

    print("=== Random Parameter Search ===")
    print(f"Games: {GAMES}, Seed: {RANDOM_SOURCE}, Trials: {NUM_TRIALS}")
    print()

    print("Running baseline...")
    baseline_results = run_tournament(BASELINE_FILE)
    if baseline_results is None:
        print("FAILED baseline!")
        sys.exit(1)

    bl_ll = baseline_results.get("landlord_strategic", 0)
    bl_fs = baseline_results.get("farmers_strategic", 0)
    bl_all = baseline_results.get("all_strategic", 0)
    print(f"Baseline: ll={bl_ll:.3f} fs_farmer={1-bl_fs:.3f} all_ll={bl_all:.3f}")
    print()

    improvements = []
    random.seed(42)

    for trial in range(NUM_TRIALS):
        variant = copy.deepcopy(baseline)

        # Randomly perturb 2-4 parameters per role
        num_changes = random.randint(2, 4)
        roles_to_change = random.sample(ROLES, k=random.randint(1, 3))

        changes = []
        for role in roles_to_change:
            params = random.sample(list(INT_PARAMS.keys()), k=min(num_changes, len(INT_PARAMS)))
            for param in params:
                min_val, max_val = INT_PARAMS[param]
                current = variant[role][param]
                # Perturb by ±1-2
                delta = random.choice([-2, -1, 1, 2])
                new_val = max(min_val, min(max_val, current + delta))
                if new_val != current:
                    variant[role][param] = new_val
                    changes.append(f"{role}.{param}={new_val}(was {current})")

        if not changes:
            continue

        exp_file = os.path.join(STRATEGY_DIR, f"_rs_trial_{trial}.json")
        write_config(variant, exp_file)

        desc = ", ".join(changes[:3])
        if len(changes) > 3:
            desc += f" +{len(changes)-3} more"
        print(f"T{trial:2d}: {desc:60s}", end=" ", flush=True)

        results = run_tournament(exp_file)
        os.remove(exp_file)

        if results is None:
            print("FAILED")
            continue

        ll = results.get("landlord_strategic", 0)
        fs = results.get("farmers_strategic", 0)
        all_ll = results.get("all_strategic", 0)

        ll_delta = ll - bl_ll
        fs_delta = (1 - fs) - (1 - bl_fs)
        all_delta = all_ll - bl_all

        # Combined score: sum of improvements across placements
        combined = ll_delta + fs_delta + all_delta

        marker = ""
        if combined > 0.04:
            marker = " *** PROMISING"
            improvements.append((trial, changes, ll_delta, fs_delta, all_delta, combined))
        elif combined > 0.02:
            marker = " * interesting"
            improvements.append((trial, changes, ll_delta, fs_delta, all_delta, combined))

        print(f"ll={ll_delta:+.3f} fs={fs_delta:+.3f} all={all_delta:+.3f} Σ={combined:+.3f}{marker}")

    print()
    print("=== Summary ===")
    if improvements:
        improvements.sort(key=lambda x: -x[5])
        for trial, changes, ll_d, fs_d, all_d, comb in improvements:
            print(f"  T{trial}: Σ={comb:+.3f} ({', '.join(changes[:3])})")
    else:
        print("No improvements found in random search.")


if __name__ == "__main__":
    main()
