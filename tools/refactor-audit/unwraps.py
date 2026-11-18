#!/usr/bin/env python3
"""Classify `.unwrap()` / `.expect(` calls in production Rust code.

Usage: python3 tools/refactor-audit/unwraps.py [ROOT] > docs/refactor/unwraps.md

Test code is excluded: files under `tests/`, `benches/`, `examples/`, files
named `tests.rs` or `*_tests.rs`, modules under a `tests/` directory, and
the bodies of `#[cfg(test)]` and `#[test]` items (tracked by brace depth). Comments and
string literals are stripped first. Each remaining call is classified by
the expression it unwraps, a syntactic guess at why it cannot fail:

- lock: `lock()`, `read()`, `write()` (poisoned mutex only)
- conversion: `try_into()`, `try_from(`, `parse` of an already-checked value
- collection: `first()`, `last()`, `get(`, `next()`, `pop()`, `max_by`,
  `min_by`, `iter().max`, `iter().min`, `peek()`
- arithmetic: `checked_*`, `partial_cmp`
- io/format: `write!`, `writeln!`, `to_string`, `from_utf8`
- other: anything else (needs a human look)
"""
import os
import re
import sys
from collections import Counter, defaultdict

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "../.."))
CRATES = os.path.join(ROOT, "crates")

CATEGORIES = [
    ("lock", re.compile(r"\.(lock|read|write)\(\)\s*$")),
    ("conversion", re.compile(r"(try_into\(\)|try_from\(.*\)|parse(::<[^>]*>)?\(\))\s*$")),
    ("collection", re.compile(r"\.(first|last|next|pop|peek|next_back)\(\)\s*$|\.get(_mut)?\(.*\)\s*$|\.(max_by|min_by|max_by_key|min_by_key)\(.*\)\s*$|\.(max|min)\(\)\s*$")),
    ("arithmetic", re.compile(r"\.(checked_\w+|partial_cmp)\(.*\)\s*$")),
    ("io/format", re.compile(r"(write!|writeln!|to_string\(\)|from_utf8\(.*\))\s*$")),
]
CALL = re.compile(r"\.(unwrap|expect)\(")


def strip(src):
    """Blank out comments and string/char literals, keeping line structure."""
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif src.startswith("/*", i):
            j = src.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", src[i:j]))
            i = j
        elif c == "r" and re.match(r'r#*"', src[i:i + 10]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            hashes = re.match(r'r(#*)"', src[i:]).group(1)
            end = '"' + hashes
            j = src.find(end, i + 2 + len(hashes))
            j = n if j < 0 else j + len(end)
            out.append(re.sub(r"[^\n]", " ", src[i:j]))
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            j = min(j + 1, n)
            out.append('"' + re.sub(r"[^\n]", " ", src[i + 1:j - 1]) + '"')
            i = j
        elif c == "'" and re.match(r"'(\\.|[^\\'])'", src[i:i + 4]):
            m = re.match(r"'(\\.|[^\\'])'", src[i:i + 4])
            out.append(" " * len(m.group(0)))
            i += len(m.group(0))
        else:
            out.append(c)
            i += 1
    return "".join(out)


def is_test_path(path):
    parts = path.split(os.sep)
    name = parts[-1]
    return (
        any(p in ("tests", "benches", "examples", "target", "fuzz") or p.endswith("_tests") for p in parts[:-1])
        or name in ("tests.rs", "test.rs", "test_utils.rs")
        or name.endswith("_tests.rs")
        or name.startswith("tests_")
    )


def production_lines(src):
    """Yield (line_number, text) for lines outside #[cfg(test)] items."""
    depth = 0
    skip = None  # brace depth at which the skipped item started
    pending = False
    for number, line in enumerate(strip(src).split("\n"), 1):
        skipping = skip is not None
        if not skipping and re.search(r"#\[(cfg\((all\()?test\b|test\])", line):
            pending = True
        for ch in line:
            if ch == "{":
                if pending and skip is None:
                    skip, pending, skipping = depth, False, True
                depth += 1
            elif ch == "}":
                depth -= 1
                if skip is not None and depth == skip:
                    skip = None
            elif ch == ";" and pending and skip is None:
                pending = False  # `#[cfg(test)] mod tests;` or a one-line item
                skipping = True
        if not skipping and not pending:
            yield number, line


def classify(prefix):
    for name, pattern in CATEGORIES:
        if pattern.search(prefix):
            return name
    return "other"


def main():
    per_crate = defaultdict(Counter)
    per_file = Counter()
    examples = defaultdict(list)
    for crate in sorted(os.listdir(CRATES)):
        base = os.path.join(CRATES, crate)
        if not os.path.isfile(os.path.join(base, "Cargo.toml")):
            continue
        src_dir = os.path.join(base, "src")
        for dirpath, _, files in os.walk(src_dir):
            for f in files:
                if not f.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, f)
                rel = os.path.relpath(path, ROOT)
                if is_test_path(os.path.relpath(path, base)):
                    continue
                with open(path, encoding="utf-8", errors="replace") as fh:
                    src = fh.read()
                for number, line in production_lines(src):
                    for m in CALL.finditer(line):
                        kind = classify(line[: m.start()])
                        per_crate[crate][kind] += 1
                        per_file[rel] += 1
                        if len(examples[kind]) < 5:
                            examples[kind].append(f"{rel}:{number}")
    kinds = ["lock", "conversion", "collection", "arithmetic", "io/format", "other"]
    total = Counter()
    for c in per_crate.values():
        total.update(c)
    print("# Production `unwrap` / `expect` audit\n")
    print("Generated by `python3 tools/refactor-audit/unwraps.py`. Test code is excluded")
    print("(see the script docstring); categories are a syntactic guess at why a call")
    print("cannot fail, `other` needs a human look.\n")
    print(f"Total: {sum(total.values())} calls.\n")
    print("| crate | " + " | ".join(kinds) + " | total |")
    print("|---|" + "---:|" * (len(kinds) + 1))
    for crate, c in sorted(per_crate.items(), key=lambda kv: -sum(kv[1].values())):
        print(f"| {crate} | " + " | ".join(str(c[k]) for k in kinds) + f" | {sum(c.values())} |")
    print("| **all** | " + " | ".join(str(total[k]) for k in kinds) + f" | {sum(total.values())} |\n")
    print("## Files with the most calls\n")
    for rel, count in per_file.most_common(25):
        print(f"- `{rel}`: {count}")
    print("\n## Sample locations per category\n")
    for k in kinds:
        print(f"- {k}: " + ", ".join(f"`{e}`" for e in examples[k]))


if __name__ == "__main__":
    main()
