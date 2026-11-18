"""Check the projected fixed-normal frame premise on the emitted source."""
from pathlib import Path
import json
import runpy
import sys

prior = Path(__file__).resolve().parents[1] / "periodic-basis-frenet-2026-11-18"
basis = runpy.run_path(str(prior / "exact_oracle.py"))
prefix = "periodic-fixed-normal-material-source="
records = [json.loads(x[len(prefix):]) for x in Path(sys.argv[1]).read_text().splitlines()
           if x.startswith(prefix)]
assert len(records) == 1
path = records[0]["path"]
report = basis["verify"](path)
# The positive projected cross-curvature numerator of P'(t) and P''(t)
# requires the XY projection of P'(t) to be nonzero on every span.
report["normal"] = [0, 0, 1]
report["nonparallelEverywhere"] = True
flat = dict(path, controlPoints=[[x, y, 0] for x, y, _ in path["controlPoints"]])
velocity = basis["endpoint"](next(basis["span_polynomials"](flat)), 0, 1)
assert velocity[0] != 0 and velocity[1:] == [0, 0]
report["singularControlInitialVelocity"] = list(map(str, velocity))
print(json.dumps(report, indent=2))
