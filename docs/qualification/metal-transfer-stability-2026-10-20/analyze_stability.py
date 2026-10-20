#!/usr/bin/env python3
"""Summarize all blocks, without trimming samples or selecting fast runs."""
import csv
import argparse
import json
import math
from pathlib import Path
import random
import statistics

parser = argparse.ArgumentParser()
parser.add_argument("--input", type=Path, default=Path(__file__).resolve().parent)
ROOT = parser.parse_args().input
summaries = []
for phase in ("cpu", "full", "production"):
    for run in range(1, 4):
        with (ROOT / f"{phase}-{run}.csv").open() as source:
            rows = list(csv.DictReader(line for line in source if not line.startswith("#")))
        keys = dict.fromkeys((r["phase"], r["case"], int(r["bytes"])) for r in rows)
        for measured_phase, case, size in keys:
            block = [r for r in rows if r["phase"] == measured_phase and r["case"] == case and int(r["bytes"]) == size]
            assert len(block) == 24
            assert sum(r["order"] == "ABBA" for r in block) == 12
            for metric in ("cpu", "total"):
                a = [float(r[f"a_{metric}_ns"]) for r in block]
                b = [float(r[f"b_{metric}_ns"]) for r in block]
                if not all(x > 0 for x in a + b):
                    continue  # production API deliberately does not expose internal CPU timing
                ratios = [math.log(x / y) for x, y in zip(a, b)]
                rng = random.Random(712)
                bootstrap = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(2000))
                summaries.append({
                    "run": run, "phase": measured_phase, "case": case, "bytes": size,
                    "metric": metric, "blocks": len(block), "iterations_per_sample": int(block[0]["iterations"]),
                    "median_a_ns": statistics.median(a), "median_b_ns": statistics.median(b),
                    "median_paired_speedup": math.exp(statistics.median(ratios)),
                    "bootstrap95": [math.exp(bootstrap[49]), math.exp(bootstrap[1949])],
                    "candidate_wins": sum(x > y for x, y in zip(a, b)),
                })
(ROOT / "summary.json").write_text(json.dumps({
    "bootstrap_seed": 712, "bootstrap_resamples": 2000, "samples_discarded": 0,
    "units": "nanoseconds per operation", "speedup": "A/B; above one favors B",
    "results": summaries,
}, indent=2) + "\n")
for size in (4096, 1048576, 16777216):
    print(f"bytes={size}")
    keys = dict.fromkeys((r["phase"], r["case"], r["metric"]) for r in summaries if r["bytes"] == size)
    for measured_phase, case, metric in keys:
        values = [r for r in summaries if (r["phase"], r["case"], r["metric"], r["bytes"]) == (measured_phase, case, metric, size)]
        print(measured_phase, case, metric, " ".join(
            f"{r['median_paired_speedup']:.3f}[{r['bootstrap95'][0]:.3f},{r['bootstrap95'][1]:.3f}]" for r in values))
