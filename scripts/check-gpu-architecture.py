#!/usr/bin/env python3
"""Check production GPU-layer dependencies and the CPU-only math contract."""
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "crates" / "Cargo.toml"
ALLOWED = {
    "gpu-compute": set(),
    "compute-core": {"gpu-compute"},
    "raster-core": {"gpu-compute"},
    "osv-math": {"compute-core", "gpu-compute"},
}


def cargo(*args):
    return subprocess.check_output(
        ["cargo", *args, "--manifest-path", str(MANIFEST)],
        cwd=ROOT, text=True,
    )


def main():
    metadata = json.loads(cargo("metadata", "--offline", "--no-deps", "--format-version", "1"))
    packages = {p["name"]: p for p in metadata["packages"]}
    problems = []
    for name, allowed in ALLOWED.items():
        # Build dependencies also affect the production build; dev-only
        # integration tests may compose multiple otherwise independent layers.
        dependencies = [d for d in packages[name]["dependencies"] if d["kind"] != "dev"]
        for dependency in dependencies:
            target = dependency["name"]
            if target in ALLOWED and target not in allowed:
                problems.append(f"{name} must not depend on {target}")
            if name == "osv-math" and target in allowed and not dependency["optional"]:
                problems.append(f"{name} -> {target} must remain optional")
    tree = cargo(
        "tree", "--offline", "-p", "osv-math", "--no-default-features",
        "--edges", "normal", "--prefix", "none", "--format", "{p}",
    )
    dependency_names = {line.split()[0] for line in tree.splitlines() if line.strip()}
    for forbidden in ["wgpu", "gpu-compute", "compute-core", "raster-core", "cudarc"]:
        if forbidden in dependency_names:
            problems.append(f"CPU-only osv-math unexpectedly includes {forbidden}")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print("GPU dependency directions and CPU-only math contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
