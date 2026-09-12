#!/usr/bin/env python3
"""Build and compare the same actual Curvex workloads in two isolated copies.

The supplied checkout directories are modified only by adding the example.
Never point this command at the user's original Curvex checkout. Build logs,
copied executables, all raw measurements and comparison evidence are retained.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("migrated", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--toolchain", default="stable")
    parser.add_argument("--target-dir", type=Path, help="Optional shared Cargo target directory")
    parser.add_argument("--replay-from", type=Path, help="Reuse verified executables and original build evidence from an earlier output directory; do not build")
    args = parser.parse_args()
    source = Path(__file__).with_name("boolean_benchmark.rs")
    repo = source.parents[2]
    args.output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str((args.target_dir or args.migrated / "target").resolve())
    evidence = {
        "schema_version": 1,
        "created_unix": time.time(),
        "platform": platform.platform(),
        "compiler": subprocess.check_output(["rustc", "+" + args.toolchain, "--version"], text=True).strip(),
        "benchmark_source_sha256": sha(source),
        "measurement": "Actual Curvex release application functions; 7 calibrated samples per workload, A/B/B/A order; operation and result drop timed; fixture and metadata work excluded.",
        "cargo_target_dir": env["CARGO_TARGET_DIR"],
        "builds": {}, "runs": [],
    }
    checkouts = [("baseline", args.baseline), ("migrated", args.migrated)]
    if args.replay_from:
        previous_path = args.replay_from / "comparison.json"
        if args.replay_from.resolve() == args.output.resolve():
            parser.error("Replay output must differ from its input to preserve previous samples")
        previous = json.loads(previous_path.read_text())
        if previous["benchmark_source_sha256"] != sha(source):
            parser.error("Benchmark source has changed since the recorded build")
        evidence["replay_of"] = {"path": str(previous_path.resolve()), "sha256": sha(previous_path)}
        evidence["compiler"] = previous["compiler"]
        evidence["cargo_target_dir"] = previous["cargo_target_dir"]
        evidence["builds"] = previous["builds"]
        for label, checkout in checkouts:
            build = previous["builds"][label]
            if str(checkout.resolve()) != build["checkout"]:
                parser.error("Replay checkout does not match the recorded " + label + " build")
            binary = args.replay_from / (label + "-boolean-benchmark")
            if sha(binary) != build["executable_sha256"]:
                parser.error("Replay executable hash differs from the recorded " + label + " build")
            shutil.copy2(binary, args.output / binary.name)
        checkouts = []
    for label, checkout in checkouts:
        checkout = checkout.resolve()
        example = checkout / "examples/osv_boolean_benchmark.rs"
        example.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, example)
        snapshot = {str(p.relative_to(repo)): sha(p) for package in ["planar-geometry", "math-core"] for p in sorted((repo / "crates" / package / "src").rglob("*.rs"))}
        command = ["cargo", "+" + args.toolchain, "build", "--offline", "--release", "--example", "osv_boolean_benchmark", "--manifest-path", str(checkout / "Cargo.toml")]
        with (args.output / (label + "-build.log")).open("w") as log:
            subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        executable = args.output / (label + "-boolean-benchmark")
        shutil.copy2(Path(env["CARGO_TARGET_DIR"]) / "release/examples/osv_boolean_benchmark", executable)
        evidence["builds"][label] = {
            "checkout": str(checkout), "command": command,
            "manifest_sha256": sha(checkout / "Cargo.toml"),
            "lockfile_sha256": sha(checkout / "Cargo.lock"),
            "example_sha256": sha(example), "executable_sha256": sha(executable),
            "source_sha256": {str(p.relative_to(checkout)): sha(p) for p in sorted((checkout / "src").rglob("*.rs"))},
            "kernel_source_sha256_before_build": snapshot if label == "migrated" else None,
            "kernel_source_unchanged_during_build": all(sha(repo / p) == digest for p, digest in snapshot.items()) if label == "migrated" else None,
        }
    for i, label in enumerate(["baseline", "migrated", "migrated", "baseline"]):
        output = subprocess.check_output([str((args.output / (label + "-boolean-benchmark")).resolve())], text=True)
        (args.output / ("run-%d-%s.jsonl" % (i + 1, label))).write_text(output)
        evidence["runs"].append({"order": i + 1, "engine": label, "measurements": [json.loads(line) for line in output.splitlines() if line.strip()]})
    names = [item["workload"] for item in evidence["runs"][0]["measurements"]]
    comparison = []
    for name in names:
        values = {}
        summaries = {}
        for label in ["baseline", "migrated"]:
            selected = [m for run in evidence["runs"] if run["engine"] == label for m in run["measurements"] if m["workload"] == name]
            values[label] = statistics.median([sample for item in selected for sample in item["samples_us"]])
            summaries[label] = [item["output"] for item in selected]
        comparison.append({"workload": name, "baseline_median_us": values["baseline"], "migrated_median_us": values["migrated"], "migrated_over_baseline": values["migrated"] / values["baseline"], "outputs": summaries})
    evidence["comparison"] = comparison
    (args.output / "comparison.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(comparison, indent=2))


if __name__ == "__main__":
    main()
