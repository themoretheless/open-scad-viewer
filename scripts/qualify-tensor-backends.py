#!/usr/bin/env python3
"""Run required native tensor backends serially and preserve their evidence."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
BACKENDS = {
    "wgsl": (
        ["-p", "compute-core", "--test", "tensor_convolution", "--test", "evaluation", "--test", "tensor", "--test", "tensor_backend", "--test", "tensor_index", "--test", "tensor_reduce", "--test", "tensor_scatter", "--test", "tensor_low", "--test", "tensor_low_ops", "--test", "tensor_low_index", "--test", "tensor_low_scatter", "--test", "tensor_low_normalization", "--test", "tensor_low_attention", "--test", "tensor_normalization", "--test", "tensor_attention"],
        {"COMPUTE_REQUIRE_GPU": "1"},
    ),
    "cuda": (["-p", "compute-cuda"], {"CUDA_REQUIRED": "1", "COMPUTE_REQUIRE_CUDA": "1"}),
    "mlx": (["-p", "compute-mlx"], {"COMPUTE_REQUIRE_MLX": "1"}),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", choices=BACKENDS, action="append", required=True,
                        help="Required backend; repeat for each installed GPU runtime")
    parser.add_argument("--output", type=Path, required=True,
                        help="Directory for report.json and raw test output")
    parser.add_argument("--offline", action="store_true", help="Use cached Cargo dependencies")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report = {
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(),
        "required_backends": list(dict.fromkeys(args.backend)),
        "runs": [],
        "passed": False,
    }
    base = ["cargo", "test", "--locked", "--manifest-path", str(ROOT / "crates/Cargo.toml")]
    if args.offline:
        base.append("--offline")
    runs = [("contracts", ["-p", "tensor-core", "--features", "conformance"], {})]
    for name in report["required_backends"]:
        packages, required_env = BACKENDS[name]
        runs.append((name, packages, required_env))
        runs.append((f"math-{name}", ["-p", "osv-math", "--features", f"tensor-{name}",
                                     "--test", "tensor_math"], required_env))
        if name == "cuda":
            runs.append(("math-cuda-f64", ["-p", "osv-math", "--features", "tensor-cuda",
                                           "--test", "tensor_f64"], required_env))
    for name, packages, required_env in runs:
        command = [*base, *packages, "--", "--test-threads=1", "--nocapture"]
        path = output / f"{name}.txt"
        print(f"Running {name}; log: {path}", flush=True)
        started = time.monotonic()
        with path.open("w", encoding="utf-8") as log:
            log.write(json.dumps({"command": command, "required_env": required_env}) + "\n")
            log.flush()
            try:
                code = subprocess.run(command, cwd=ROOT, env={**os.environ, **required_env},
                                      stdout=log, stderr=subprocess.STDOUT, check=False).returncode
            except OSError as error:
                log.write(f"Unable to start Cargo: {error}\n")
                code = 127
        report["runs"].append({
            "name": name, "command": command, "required_env": required_env,
            "exit_code": code, "elapsed_seconds": round(time.monotonic() - started, 3),
            "log": path.name,
        })
        print(f"{name}: {'PASS' if code == 0 else 'FAIL'}", flush=True)
        # Persist even if a later backend is interrupted or fails to initialize.
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    report["passed"] = all(run["exit_code"] == 0 for run in report["runs"])
    report["finished_utc"] = datetime.now(timezone.utc).isoformat()
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
