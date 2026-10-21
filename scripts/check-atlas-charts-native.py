#!/usr/bin/env python3
"""Exercise source-chart production, independent-process replay and refusals."""
import copy, hashlib, json, subprocess, sys, time
from pathlib import Path

checker, fixtures, output = map(Path, sys.argv[1:])
report = {"checkerSha256": hashlib.sha256(checker.read_bytes()).hexdigest(),
          "cases": [], "complete": False}

def save():
    output.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")

def run(mode, request, accepted):
    start = time.monotonic()
    p = subprocess.run([str(checker), mode], input=json.dumps(request, allow_nan=False),
                       text=True, capture_output=True, timeout=600)
    assert (p.returncode == 0) == accepted, (mode, p.returncode, p.stderr)
    return (json.loads(p.stdout) if accepted else p.stderr.strip(), time.monotonic()-start)

def zero(v):
    if isinstance(v, list): return [zero(x) for x in v]
    if isinstance(v, dict): return {k: zero(x) for k, x in v.items()}
    return 0.0 if isinstance(v, float) and v == 0 else v

save()
for name, depth in [("collapsed-boundary-tetrahedron", 0),
                    ("nonlinear-projective-clock-box", 0),
                    ("curved-trim-regular-poles-sphere", 1),
                    ("rational-freeform-box", 0)]:
    model = zero(json.loads((fixtures / (name + ".json")).read_text()))
    produced, seconds = run("certify-nurbs-source-atlas", {"model": model, "depth": depth}, True)
    request = json.loads(json.dumps({"model": model, "certificate": produced["certificate"]}))
    verified, verify_seconds = run("verify-nurbs-source-atlas", request, True)
    assert verified == produced
    case = {"name": name, "depth": depth, "producerSeconds": seconds,
            "verifierSeconds": verify_seconds, "request": request, "forgeries": []}
    report["cases"].append(case); save()
    mutations = [
        ("source-binding", lambda q: q["certificate"].update(source="0"*64)),
        ("chart-count", lambda q: q["certificate"]["report"].update(chartTriangles=999)),
        ("invalid-depth", lambda q: q["certificate"].update(depth=4)),
        ("collapsed-triangle", lambda q: q["certificate"]["triangles"][0].__setitem__(1, q["certificate"]["triangles"][0][0])),
        ("unknown-schema-field", lambda q: q["certificate"].update(unchecked=True)),
        ("complement-claim", lambda q: q["certificate"]["report"].update(preservesComplement=False)),
        ("rebound-geometry", lambda q: q["model"]["faces"][0]["surface"]["controlPoints"][0][0].__setitem__(0, q["model"]["faces"][0]["surface"]["controlPoints"][0][0][0] + 0.125)),
    ]
    for mutation, apply in mutations:
        forged = copy.deepcopy(request); apply(forged)
        rejection, seconds = run("verify-nurbs-source-atlas", forged, False)
        case["forgeries"].append({"mutation": mutation, "seconds": seconds, "rejection": rejection})
        save()
report["complete"] = True; save()
print(json.dumps({"proofs": len(report["cases"]),
                  "forgeriesRejected": sum(len(c["forgeries"]) for c in report["cases"])}))
