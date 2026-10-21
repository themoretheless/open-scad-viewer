"""Orchestrate independent Rust checking of the cross-runtime proof corpus."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import time

p = argparse.ArgumentParser()
p.add_argument("report", type=pathlib.Path)
p.add_argument("output", type=pathlib.Path)
p.add_argument("--checker", default="crates/target/debug/examples/verify_profile_certificate")
p.add_argument("--skip-refusals", action="store_true", help="Replay historical proof compatibility without historical admission refusals")
a = p.parse_args()
corpus = json.loads(a.report.read_text())
results = []
for item in corpus["proofs"]:
    start = time.perf_counter()
    good = subprocess.run([a.checker, item["mode"]], input=json.dumps(item["request"]),
                          capture_output=True, text=True, timeout=120)
    assert good.returncode == 0, (item["name"], good.stderr)
    value = json.loads(good.stdout)
    assert value["status"] == "verified" and value["scope"] == item["verified"]["scope"]
    for key in ("maximumSetDeviationExactMm", "exactVolumeMm3", "algebraicLoci", "stationarySpans", "branchDecompositions", "approximationTopology", "sourceApproximationTopology"):
        if key in item["verified"]:
            assert value[key] == item["verified"][key], (item["name"], key)
    refusals = []
    for forged in item["forgeries"]:
        bad = subprocess.run([a.checker, item["mode"]], input=json.dumps(forged["request"]),
                             capture_output=True, text=True, timeout=120)
        assert bad.returncode != 0, (item["name"], forged["mutation"])
        refusals.append({"mutation": forged["mutation"], "reason": bad.stderr.strip()})
    results.append({"name": item["name"], "verified": value, "forgedRejected": len(refusals),
                    "refusals": refusals, "seconds": time.perf_counter() - start})
    print(item["name"], "verified;", len(refusals), "forgeries rejected", flush=True)
expected_refusals = []
for item in ([] if a.skip_refusals else corpus["refusals"]):
    bad = subprocess.run([a.checker, item["mode"]], input=json.dumps(item["request"]),
                         capture_output=True, text=True, timeout=120)
    assert bad.returncode != 0, item["name"]
    expected_refusals.append({"name": item["name"], "reason": bad.stderr.strip()})
a.output.write_text(json.dumps({"wasmSha256": corpus["wasmSha256"],
                               "checkerSha256": hashlib.sha256(pathlib.Path(a.checker).read_bytes()).hexdigest(),
                               "historicalRefusalsSkipped": [item["name"] for item in corpus["refusals"]] if a.skip_refusals else [],
                               "proofs": results, "expectedRefusals": expected_refusals}, indent=2) + "\n")
