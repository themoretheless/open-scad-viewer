#!/usr/bin/env python3
"""Produce new source witnesses and replay qualified source/enclosure archives."""
import copy
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

checker, fixtures, old_source, old_enclosure, output = map(Path, sys.argv[1:])
report = {"checkerSha256": hashlib.sha256(checker.read_bytes()).hexdigest(),
          "proofs": [], "forgeries": [], "complete": False}


def save():
    output.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")


def run(mode, request, accept):
    assert mode in {"certify-nurbs-source-atlas", "verify-nurbs-source-atlas",
                    "verify-minkowski-nurbs", "verify-minkowski-3d"}, mode
    started = time.monotonic()
    result = subprocess.run([str(checker), mode], input=json.dumps(request, allow_nan=False),
                            text=True, capture_output=True, timeout=600)
    assert (result.returncode == 0) == accept, (mode, result.returncode, result.stderr)
    return (json.loads(result.stdout) if accept else result.stderr.strip(),
            time.monotonic() - started)


def zero(value):
    if isinstance(value, list): return [zero(x) for x in value]
    if isinstance(value, dict): return {k: zero(v) for k, v in value.items()}
    return 0.0 if isinstance(value, float) and value == 0 else value


def verify(name, mode, request, origin):
    value, seconds = run(mode, request, True)
    assert value["status"] == "verified"
    assert value["certificate"] == request["certificate"]
    report["proofs"].append({"name": name, "mode": mode, "origin": origin,
                             "seconds": seconds, "request": request})
    save()


def refuse(name, mutation, mode, request):
    rejection, seconds = run(mode, request, False)
    report["forgeries"].append({"name": name, "mutation": mutation,
                                 "mode": mode, "seconds": seconds, "rejection": rejection})
    save()


save()
for name in ["generic-two-holed-region", "generic-curved-through-hole",
             "stationary-angular-radial-pole", "nonprojective-polynomial-clock-box", "rational-nonmonomial-poles-tetrahedron",
             "nonplanar-rational-pole-tetrahedron"]:
    model = zero(json.loads((fixtures / (name + ".json")).read_text()))
    value, seconds = run("certify-nurbs-source-atlas", {"model": model, "depth": 0}, True)
    request = json.loads(json.dumps({"model": model, "certificate": value["certificate"]}))
    verify(name, "verify-nurbs-source-atlas", request, "new-producer")
    report["proofs"][-1]["producerSeconds"] = seconds
    mutations = [
        ("source-binding", lambda q: q["certificate"].update(source="0"*64)),
        ("chart-count", lambda q: q["certificate"]["report"].update(chartTriangles=999)),
        ("invalid-depth", lambda q: q["certificate"].update(depth=4)),
        ("collapsed-simplex", lambda q: q["certificate"]["triangles"][0].__setitem__(1, q["certificate"]["triangles"][0][0])),
        ("unknown-field", lambda q: q["certificate"].update(unchecked=True)),
        ("complement-claim", lambda q: q["certificate"]["report"].update(preservesComplement=False)),
        ("source-geometry", lambda q: q["model"]["faces"][0]["surface"]["controlPoints"][0][0].__setitem__(0, q["model"]["faces"][0]["surface"]["controlPoints"][0][0][0]+0.125)),
    ]
    for mutation, apply in mutations:
        forged = copy.deepcopy(request)
        apply(forged)
        refuse(name, mutation, "verify-nurbs-source-atlas", forged)
    forged = copy.deepcopy(request)
    forged["certificate"]["triangles"].pop()
    forged["certificate"]["report"]["chartTriangles"] -= 1
    refuse(name, "omit-triangle-with-consistent-count", "verify-nurbs-source-atlas", forged)

for case in json.loads(old_source.read_text())["cases"]:
    verify(case["name"], "verify-nurbs-source-atlas", case["request"], "qualified-2026-10-20")

archive = json.loads(old_enclosure.read_text())
for case in archive["sourceProofs"] + archive["sumProofs"]:
    verify(case["name"], case["mode"], case["request"], "qualified-2026-10-19")
    for forgery in case.get("forgeries", []):
        refuse(case["name"], forgery["mutation"], case["mode"], forgery["request"])

# Recompute whole-source admission for invalid interiors, without a stale
# source hash or any certificate to short-circuit the geometric predicates.
for name in ["rational-nonmonomial-poles-tetrahedron", "nonplanar-rational-pole-tetrahedron"]:
    model = zero(json.loads((fixtures / (name + ".json")).read_text()))
    model["faces"][0]["surface"]["controlPoints"][1][1][0] += 32
    rejection, seconds = run("certify-nurbs-source-atlas", {"model": model, "depth": 0}, False)
    report["forgeries"].append({"name": name, "mutation": "folded-source-interior",
                               "mode": "certify-nurbs-source-atlas", "seconds": seconds,
                               "rejection": rejection})
    save()

report["complete"] = True
save()
print(json.dumps({"proofs": len(report["proofs"]),
                  "forgeriesRejected": len(report["forgeries"])}))
