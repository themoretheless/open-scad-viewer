#!/usr/bin/env python3
"""Run replicas serially; never stop or reconfigure other workloads."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("--binary", type=Path, default=ROOT.parents[2] / "crates/target/release/examples/bench_transfer_stability")
parser.add_argument("--output", type=Path, default=ROOT)
parser.add_argument("--runs", type=int, default=3)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
if any(args.output.glob("cpu-*.csv")) or any(args.output.glob("full-*.csv")) or any(args.output.glob("production-*.csv")):
    raise SystemExit("Output already contains measurements; choose a fresh --output directory")
binary_hash = hashlib.sha256(args.binary.read_bytes()).hexdigest()

for run in range(1, args.runs + 1):
    for phase in ("cpu", "full", "production"):
        meta = {"run": run, "phase": phase, "start_unix": time.time(), "benchmark_sha256": binary_hash}
        for key, cmd in (("thermal", ["pmset", "-g", "therm"]), ("power", ["pmset", "-g", "batt"])):
            meta[key] = subprocess.run(cmd, capture_output=True, text=True).stdout
        processes = subprocess.check_output(["ps", "-axo", "pid,ppid,%cpu,etime,comm"], text=True)
        top = []
        for line in processes.splitlines()[1:]:
            parts = line.split(None, 4)
            if len(parts) == 5:
                top.append({"pid": int(parts[0]), "cpu_percent": float(parts[2]),
                            "elapsed": parts[3], "program": Path(parts[4]).name})
        meta["background_top_cpu"] = sorted(top, key=lambda p: p["cpu_percent"], reverse=True)[:12]
        with (args.output / f"{phase}-{run}.csv").open("w") as output:
            result = subprocess.run([str(args.binary), phase], stdout=output, stderr=subprocess.PIPE, text=True)
        meta.update(end_unix=time.time(), returncode=result.returncode, stderr=result.stderr)
        (args.output / f"{phase}-{run}-environment.json").write_text(json.dumps(meta, indent=2) + "\n")
        print(f"{phase} {run}: exit {result.returncode}, {meta['end_unix'] - meta['start_unix']:.1f}s", flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)
