#!/usr/bin/env python3
"""Read-only lexical inventory of Curvex's direct 2D geometry dependency sites.

Print JSON to stdout; never writes to Curvex, runs Cargo, or interprets source
text as instructions. Includes comment-only references separately so a lexical
match is not misrepresented as a dependency call. This is an inventory, not a
Rust parser or proof that a migration compiles.
"""

import argparse
import collections
import hashlib
import json
import pathlib
import re
import subprocess


DEPENDENCIES = ("kurbo", "linesweeper", "lyon_path", "lyon_tessellation")
REFERENCE = re.compile(r"\b(" + "|".join(DEPENDENCIES) + r")\b")
NAMESPACE = re.compile(r"\b(" + "|".join(DEPENDENCIES) + r")\s*::")


def git_value(root, *args):
    result = subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=False
    )
    return result.stdout.strip() if result.returncode == 0 else None


def inventory(root):
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        raise ValueError("Curvex source root must contain Cargo.toml")
    paths = [manifest]
    for category in ("src", "tests", "benches", "examples"):
        paths.extend(sorted((root / category).rglob("*.rs")))
    entries = []
    files = []
    counts = collections.Counter()
    for path in paths:
        source = path.read_text(encoding="utf-8")
        relative = path.relative_to(root).as_posix()
        category = "manifest" if relative == "Cargo.toml" else relative.split("/")[0]
        files.append({"path": relative, "sha256": hashlib.sha256(source.encode()).hexdigest()})
        for line_number, line in enumerate(source.splitlines(), 1):
            matched = sorted(set(REFERENCE.findall(line)))
            if not matched:
                continue
            # Deliberately conservative: retains raw text and labels candidates.
            # Multiline block comments, strings and aliases need source review.
            code = line.split("//", 1)[0]
            is_code = bool(NAMESPACE.search(code)) or (
                category == "manifest"
                and re.match(r"\s*(" + "|".join(DEPENDENCIES) + r")\s*=", code) is not None
            )
            kind = "code_candidate" if is_code else "other_reference"
            for dependency in matched:
                counts[(category, dependency, kind)] += 1
            entries.append({
                "path": relative,
                "line": line_number,
                "dependencies": matched,
                "kind": kind,
                "text": line.strip(),
            })
    lock = root / "Cargo.lock"
    lock_reverse = {dependency: [] for dependency in DEPENDENCIES}
    if lock.is_file():
        for package in lock.read_text(encoding="utf-8").split("[[package]]")[1:]:
            name = re.search(r'^name = "([^"]+)"', package, re.M)
            dependencies = re.search(r"dependencies = \[(.*?)\]", package, re.S)
            if name and dependencies:
                names = set(re.findall(r'"([^" ]+)(?: [^"]*)?"', dependencies.group(1)))
                for dependency in DEPENDENCIES:
                    if dependency in names:
                        lock_reverse[dependency].append(name.group(1))
    return {
        "schema_version": 1,
        "source_root": str(root),
        "source_commit": git_value(root, "rev-parse", "HEAD"),
        "source_status": git_value(root, "status", "--porcelain"),
        "scope": "Every Rust file under src/tests/benches/examples and root Cargo.toml",
        "limitations": "Lexical inventory; aliases/method calls require manual contract review and compilation.",
        "dependencies": list(DEPENDENCIES),
        "files": files,
        "counts": [
            {"category": c, "dependency": d, "kind": k, "lines": n}
            for (c, d, k), n in sorted(counts.items())
        ],
        "lockfile_direct_consumers": lock_reverse,
        "references": entries,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=pathlib.Path, help="Read-only Curvex source directory")
    args = parser.parse_args()
    print(json.dumps(inventory(args.source.resolve()), indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
