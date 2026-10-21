"""Run the standalone Rust checker against WASM witnesses and their forgeries."""
import argparse
import json
import pathlib
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("report", type=pathlib.Path)
parser.add_argument("output", type=pathlib.Path)
parser.add_argument("--checker", default="crates/target/debug/examples/verify_profile_certificate")
args = parser.parse_args()
results = []
for proof in json.loads(args.report.read_text())["proofs"]:
    started = time.perf_counter()
    request = {k: proof[k] for k in ("a", "b", "certificate")}
    good = subprocess.run([args.checker, "verify-minkowski-3d"], input=json.dumps(request),
                          text=True, capture_output=True, timeout=120)
    assert good.returncode == 0, (proof["name"], good.stderr)
    verified = json.loads(good.stdout)
    assert verified["status"] == "verified"
    assert verified["exactVolumeMm3"] == proof["verified"]["exactVolumeMm3"]
    refusals = []
    for forged in proof["forgeries"]:
        bad = subprocess.run([args.checker, "verify-minkowski-3d"], input=json.dumps(forged),
                             text=True, capture_output=True, timeout=120)
        assert bad.returncode != 0, (proof["name"], forged["mutation"])
        refusals.append({"mutation": forged["mutation"], "refusal": bad.stderr.strip()})
    results.append({"name": proof["name"], "verified": verified,
                    "forgedRejected": len(refusals), "refusals": refusals,
                    "seconds": time.perf_counter() - started})
    print(proof["name"], "verified;", len(refusals), "forgeries rejected", flush=True)
args.output.write_text(json.dumps(results, indent=2) + "\n")
