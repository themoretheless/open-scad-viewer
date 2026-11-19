"""Exact checks on the periodic source emitted by the native material test."""
from itertools import combinations
from pathlib import Path
import json
import runpy
import sys

prior = Path(__file__).resolve().parents[1] / "periodic-basis-frenet-2026-11-18"
basis = runpy.run_path(str(prior / "exact_oracle.py"))


def det(a, b, c):
    return sum(a[i] * (b[(i + 1) % 3] * c[(i + 2) % 3]
                       - b[(i + 2) % 3] * c[(i + 1) % 3]) for i in range(3))


prefix = "periodic-bishop-material-source="
records = [json.loads(x[len(prefix):]) for x in Path(sys.argv[1]).read_text().splitlines()
           if x.startswith(prefix)]
assert len(records) == 1
path = records[0]["path"]
report = basis["verify"](path)
origin, *nodes = [basis["endpoint"](s, 0, 0) for s in basis["span_polynomials"](path)]
deltas = [[x - y for x, y in zip(node, origin)] for node in nodes]
nonplanarity = next(d for a, b, c in combinations(deltas, 3) if (d := det(a, b, c)))
report["exactNonplanarNodeDeterminant"] = str(nonplanarity)
print(json.dumps(report, indent=2))
