"""Check the transverse-plane rank obstruction on the emitted BRep inputs."""
from fractions import Fraction as Q
from pathlib import Path
import json
import runpy
import sys

prior = Path(__file__).resolve().parents[1] / "periodic-basis-frenet-2026-11-18"
basis = runpy.run_path(str(prior / "exact_oracle.py"))
prefix = "closed-fixed-transverse-material-source="
records = [json.loads(line[len(prefix):]) for line in Path(sys.argv[1]).read_text().splitlines()
           if line.startswith(prefix)]
assert len(records) == 1
data = records[0]
path = list(basis["span_polynomials"](data["path"]))
initial = basis["endpoint"](path[0], 0, 1)
assert initial[0] != 0 and initial[1:] == [0, 0]
# Projected longitudinal numerator along this fixed axis. Its zero is
# unaffected by normalization of the nonzero axis or longitudinal scaling.
axial_values = {
    sum(axis * Q(value) for axis, value in zip(initial, point))
    for row in data["cap"]["controlPoints"] for point in row
}
assert len(axial_values) == 1
assert basis["endpoint"](path[0], 0, 0) == basis["endpoint"](path[-1], 1, 0)
assert all(basis["endpoint"](a, 1, 1) == basis["endpoint"](b, 0, 1)
           for a, b in zip(path, path[1:]))
# The cap projection is zero in this fixture. Its longitudinal derivative
# therefore reduces to path X', independent of the varying scale or twist.
assert axial_values == {0}
stationary_nodes = [i for i, span in enumerate(path)
                    if 0 < i and basis["endpoint"](span, 0, 1)[0] == 0]
assert stationary_nodes
print(json.dumps({"initialDerivative": list(map(str, initial)),
                  "capProjection": "0", "stationaryInteriorKnots": stationary_nodes}, indent=2))
