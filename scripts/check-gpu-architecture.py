#!/usr/bin/env python3
"""Check production GPU-layer dependencies and the CPU-only math contract."""
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "crates" / "Cargo.toml"
ALLOWED = {
    "tensor-core": set(),
    "gpu-compute": set(),
    "compute-core": {"gpu-compute", "tensor-core"},
    "compute-cuda": {"gpu-compute", "tensor-core"},
    "compute-mlx": {"tensor-core"},
    "raster-core": {"gpu-compute"},
    "osv-math": {"compute-core", "gpu-compute", "tensor-core", "compute-cuda", "compute-mlx"},
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
    def normal_dependencies(name, *features):
        tree = cargo(
            "tree", "--offline", "-p", name, "--no-default-features",
            *features,
            "--edges", "normal", "--prefix", "none", "--format", "{p}",
        )
        return {line.split()[0] for line in tree.splitlines() if line.strip()} - {name}

    gpu_dependencies = {
        "wgpu", "gpu-compute", "compute-core", "compute-cuda", "compute-mlx",
        "raster-core", "cudarc",
    }
    for name in ("osv-math", "tensor-core"):
        forbidden = normal_dependencies(name) & gpu_dependencies
        for dependency in sorted(forbidden):
            problems.append(f"CPU-only {name} unexpectedly includes {dependency}")
    # MLX owns its native runtime; pulling wgpu/CUDA through another layer would
    # defeat independent backend loading and the small tensor contract crate.
    for dependency in sorted(normal_dependencies("compute-mlx") & (gpu_dependencies - {"compute-mlx"})):
        problems.append(f"MLX adapter unexpectedly includes {dependency}")
    for dependency in sorted(normal_dependencies("osv-math", "--features", "tensor") & gpu_dependencies):
        problems.append(f"Tensor geometry contracts unexpectedly include {dependency}")
    for dependency in sorted(normal_dependencies("osv-math", "--features", "tensor-mlx") & (gpu_dependencies - {"compute-mlx"})):
        problems.append(f"MLX-only tensor geometry unexpectedly includes {dependency}")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print("GPU dependency directions, backend isolation and CPU-only contracts: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
