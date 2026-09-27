"""Recompute the retained benchmark summaries without running a GPU workload."""
from pathlib import Path
import argparse
import json
import math
import re

ROOT = Path(__file__).resolve().parents[3]
DEST = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument(
    "--stage",
    choices=["final", "initial", "specialized", "query-cache", "guarded-f16"],
    default="final",
)
stage = parser.parse_args().stage
prefix = "tensor-low-attention" + ("" if stage == "final" else f"-{stage}")
PATTERN = re.compile(
    r"^(\S+) (cast_f32|direct_low) (host|gpu) "
    r"median_ms=([\d.]+) p90_ms=([\d.]+) samples_ms=(\[.*\])$"
)
runs = []
pairs = 0
for suffix in ("", "-repeat"):
    path = Path(f"crates/compute-core/benchmarks/{prefix}-metal{suffix}.txt")
    cases = {}
    for line in (ROOT / path).read_text().splitlines():
        match = PATTERN.match(line)
        if not match:
            continue
        case, route, mode, median, p90, raw = match.groups()
        samples = json.loads(raw)
        assert len(samples) == 31 and all(math.isfinite(x) and x > 0 for x in samples)
        ordered = sorted(samples)
        assert abs(float(median) - ordered[15]) <= 0.500001e-6
        assert abs(float(p90) - ordered[27]) <= 0.500001e-6
        key = f"{route}_{mode}"
        assert key not in cases.setdefault(case, {})
        cases[case][key] = {
            "median_ms": ordered[15], "p90_ms": ordered[27], "samples_ms": samples
        }
        pairs += 1
    assert len(cases) == 10 and all(len(measures) == 4 for measures in cases.values())
    runs.append({"log": str(path), "cases": cases})

assert runs[0]["cases"].keys() == runs[1]["cases"].keys()
assert pairs == 80
report = {"stage": stage, "runs": runs, "median_p90_pairs": pairs, "samples": pairs * 31}
for name, predicate in [
    ("faster_in_both", lambda values: all(x > 1 for x in values)),
    ("slower_in_both", lambda values: all(x < 1 for x in values)),
    ("changed_sides", lambda values: min(values) < 1 < max(values)),
    ("tied_in_either", lambda values: any(x == 1 for x in values)),
]:
    report[name] = []
    for case in runs[0]["cases"]:
        ratios = [
            run["cases"][case]["cast_f32_gpu"]["median_ms"]
            / run["cases"][case]["direct_low_gpu"]["median_ms"]
            for run in runs
        ]
        if predicate(ratios):
            report[name].append(case)

categories = ("faster_in_both", "slower_in_both", "changed_sides", "tied_in_either")
assert sum(len(report[name]) for name in categories) == len(runs[0]["cases"])

output = "benchmark-summary.json" if stage == "final" else f"{stage}-benchmark-summary.json"
(DEST / output).write_text(json.dumps(report, indent=2) + "\n")
print(f"PASS: {pairs} median/p90 pairs from {pairs * 31} samples")
for name in categories:
    print(f"{name}: {len(report[name])}; {', '.join(report[name])}")
for case in runs[0]["cases"]:
    values = []
    for run in runs:
        measures = run["cases"][case]
        old = measures["cast_f32_gpu"]["median_ms"]
        new = measures["direct_low_gpu"]["median_ms"]
        values.append(f"{old:.6f}/{new:.6f} ms ({old/new:.3f}x)")
    print(case + ": " + "; ".join(values))
