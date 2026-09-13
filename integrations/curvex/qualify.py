#!/usr/bin/env python3
"""Stage and validate the actual Curvex migration, retaining reproducible evidence.

Uses local cached Cargo dependencies. Original checkout is read-only. A fresh
destination is required; target artifacts may be shared to avoid rebuilds.
"""
import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_manifest():
    paths = sorted((ROOT / "crates/planar-geometry").rglob("*.rs"))
    paths += sorted((ROOT / "crates/math-core").rglob("*.rs"))
    paths += sorted(HERE.glob("*.rs")) + sorted(HERE.glob("*.py"))
    paths += [ROOT / "crates/planar-geometry/Cargo.toml", ROOT / "crates/math-core/Cargo.toml"]
    return {str(p.relative_to(ROOT)): sha(p) for p in paths}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("source", type=Path)
    ap.add_argument("destination", type=Path)
    ap.add_argument("--target", required=True, type=Path)
    ap.add_argument("--baseline", type=Path)
    ap.add_argument("--resume", action="store_true", help="Refresh an existing isolated migration from its verified original source and rerun every gate")
    args = ap.parse_args()
    source, destination = args.source.resolve(), args.destination.resolve()
    for label, path in (("destination", destination), ("target", args.target), ("baseline", args.baseline)):
        if path is not None:
            resolved = path.resolve()
            if resolved == source or source in resolved.parents:
                ap.error(f"{label} must be outside the original source checkout")
    if args.baseline is not None:
        baseline_example = (args.baseline / "examples/osv_render_qualification.rs").resolve()
        if baseline_example == source or source in baseline_example.parents:
            ap.error("baseline example must be outside the original source checkout")
    if args.resume:
        import prepare
        provenance = json.loads((destination/"osv-migration-source.json").read_text())
        if Path(provenance["source"]) != source:
            ap.error("resume source differs from recorded original checkout")
        for relative, digest in provenance["sourceHashes"].items():
            original = source / relative
            if sha(original) != digest:
                ap.error(f"original source changed: {relative}; use a fresh migration")
            changed = original.read_text()
            for old, new in prepare.REPLACEMENTS.items():
                changed = changed.replace(old, new)
            if relative == "src/ui/shape_render.rs":
                changed = prepare.migrate_gradient_sampling(changed)
            target = destination / relative
            if target.read_text() != changed:
                target.write_text(changed)
        for helper in ("gradient_sampling", "render_cache"):
            shutil.copy2(HERE/f"{helper}.rs", destination/f"src/ui/osv_{helper}.rs")
    else:
        subprocess.run([sys.executable, str(HERE / "prepare.py"), str(source), str(destination)], check=True)
    evidence = destination / "qualification"
    if evidence.exists():
        evidence.rename(destination / f"qualification-prior-{time.time_ns()}")
    evidence.mkdir()
    manifest = source_manifest()
    report = {"source": str(source), "destination": str(destination), "kernel_before": manifest, "commands": []}
    report["toolchain"] = subprocess.check_output(["rustc", "+stable", "-vV"], text=True).strip()
    host = re.search(r"^host: (.+)$", report["toolchain"], re.MULTILINE).group(1)
    for name in ("render_qualification", "boolean_benchmark"):
        shutil.copy2(HERE / f"{name}.rs", destination / "examples" / f"osv_{name}.rs")
    env = dict(os.environ, CARGO_TARGET_DIR=str(args.target.resolve()))

    def run(label, command, cwd=destination, env=env, stdout=None):
        log = evidence / f"{label}.log"
        start = time.monotonic()
        print(f"START {label}", flush=True)
        with log.open("w") as output:
            if stdout is None:
                result = subprocess.run(command, cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT)
            else:
                with stdout.open("w") as capture:
                    result = subprocess.run(command, cwd=cwd, env=env, stdout=capture, stderr=output)
        entry = {"label": label, "command": command, "cwd": str(cwd), "exit_code": result.returncode,
                 "seconds": round(time.monotonic()-start, 3), "log": str(log), "log_sha256": sha(log)}
        if stdout: entry["stdout"] = {"path": str(stdout), "sha256": sha(stdout)}
        report["commands"].append(entry)
        (evidence / "report.json").write_text(json.dumps(report, indent=2)+"\n")
        print(f"END {label}: exit {result.returncode}, {entry['seconds']}s", flush=True)
        if result.returncode:
            print(log.read_text()[-6000:], flush=True)
            raise SystemExit(result.returncode)
        return log

    cargo = ["cargo", "+stable"]
    # Cargo's local package fingerprints can collide across isolated copies
    # sharing a target directory, especially with preserved include! mtimes.
    # Remove only Curvex artifacts, retaining its expensive GUI dependencies.
    run("curvex-package-clean", cargo+["clean", "--package", "curvex"])
    run("curvex-tests", cargo+["test", "--offline", "--lib", "--bins", "--tests"])
    test_log = (evidence/"curvex-tests.log").read_text()
    for name in ("repeated_gradient_at_large_document_coordinates_keeps_sharp_seams",
                 "radial_interior_and_recolor_cache_survive_diagonal_changes",
                 "three_hundred_gradient_shapes_keep_geometry_and_sample_cache_hits"):
        if not re.search(r"^test .*::"+name+r" \.\.\. ok$", test_log, re.MULTILINE):
            raise SystemExit(f"Missing executed migration regression: {name}; inspect staged source and Cargo artifacts")
    run("curvex-original-ignored-timing", cargo+["test", "--offline", "--lib", "sample_fast_path_timing", "--", "--ignored"])
    run("curvex-doctests", cargo+["test", "--offline", "--doc"])
    run("curvex-check-all-targets", cargo+["check", "--offline", "--all-targets"])
    run("curvex-release-build", cargo+["build", "--offline", "--release"])
    metadata_file = evidence / "cargo-metadata.json"
    run("metadata", cargo+["metadata", "--offline", "--format-version", "1", "--filter-platform", host], stdout=metadata_file)
    metadata = json.loads(metadata_file.read_text())
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    native = next(p["id"] for p in metadata["packages"] if p["name"] == "planar-geometry")
    closure, pending = set(), [native]
    while pending:
        package = pending.pop()
        if package in closure: continue
        closure.add(package)
        pending.extend(nodes[package]["dependencies"])
    old = {"kurbo", "linesweeper", "lyon_path", "lyon_tessellation"}
    report["kernel_dependency_closure"] = [{"name": packages[i]["name"], "version": packages[i]["version"]} for i in sorted(closure)]
    assert not any(packages[i]["name"] in old for i in closure)
    app = next(p for p in metadata["packages"] if p["name"] == "curvex")
    assert not any(d["name"] in old for d in app["dependencies"])
    for package in metadata["packages"]:
        if package["name"] in old:
            name = f"{package['name']}@{package['version']}"
            run(f"remaining-{name}", cargo+["tree", "--offline", "--invert", name])
    run("curvex-render-release", cargo+["run", "--offline", "--release", "--example", "osv_render_qualification"], stdout=evidence / "migrated-render.json")
    if args.baseline:
        baseline = args.baseline.resolve()
        shutil.copy2(HERE / "render_qualification.rs", baseline / "examples/osv_render_qualification.rs")
        run("baseline-render-release", cargo+["run", "--offline", "--release", "--example", "osv_render_qualification"], cwd=baseline, stdout=evidence / "baseline-render.json")
    # Rebuild the patch after Cargo updates the lockfile. Qualification examples
    # and logs are deliberately outside the application migration patch.
    paths = [p.relative_to(source) for p in source.rglob("*.rs") if not any(x in p.parts for x in ("target", ".git"))]
    paths += [Path("Cargo.toml"), Path("Cargo.lock"), Path("src/ui/osv_gradient_sampling.rs"), Path("src/ui/osv_render_cache.rs")]
    patch = []
    for path in sorted(set(paths)):
        original = (source/path).read_text() if (source/path).exists() else ""
        changed = (destination/path).read_text()
        if changed != original:
            patch.extend(difflib.unified_diff(original.splitlines(True), changed.splitlines(True),
                fromfile="a/"+str(path) if (source/path).exists() else "/dev/null", tofile="b/"+str(path)))
    (destination/"osv-migration.patch").write_text("".join(patch))
    report["migration_patch_sha256"] = sha(destination/"osv-migration.patch")
    tests = (evidence/"curvex-tests.log").read_text()
    report["tests"] = {name: result for name,result in re.findall(r"^test (.*?) \.\.\. (ok|FAILED|ignored)", tests, re.MULTILINE)}
    report["kernel_after"] = source_manifest()
    report["kernel_unchanged"] = report["kernel_before"] == report["kernel_after"]
    (evidence/"report.json").write_text(json.dumps(report, indent=2)+"\n")
    if not report["kernel_unchanged"]: raise SystemExit("Kernel source changed during qualification; rerun on frozen sources")
    print(f"Qualification build/test evidence saved to {evidence}", flush=True)


if __name__ == "__main__":
    main()
