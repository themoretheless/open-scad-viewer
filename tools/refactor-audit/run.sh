#!/usr/bin/env bash
# Reproduce the refactoring audit: measures every Rust and TypeScript function, the module graphs,
# duplication and unreachable files, then regenerates docs/refactor/INVENTORY.md and docs/refactor/inventory/.
# Read-only for the source tree. AUDIT_LINTS=0 skips the clippy and dead-code passes (about 6 minutes).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"; HERE="$ROOT/tools/refactor-audit"
OUT="${AUDIT_OUT:-$HERE/.out}"; A="$OUT/audit"; DOCS="${AUDIT_DOCS:-$ROOT/docs/refactor}"
mkdir -p "$A" "$DOCS/inventory"
cd "$ROOT"
CARGO_TARGET_DIR="$OUT/target" cargo build --quiet --release --locked --manifest-path "$HERE/audit-rs/Cargo.toml"
cargo metadata --manifest-path crates/Cargo.toml --format-version 1 --locked > "$OUT/metadata.json"
node "$HERE/metadata.mjs" "$OUT/metadata.json" "$OUT"
"$OUT/target/release/audit-rs" "$ROOT" "$OUT/roots.tsv" "$OUT/deps.tsv" "$A" crates
git status --porcelain > "$A/gitstatus.txt"
git log --since='60 days ago' --format='@%h' --name-only -- crates src scripts tests | grep -v '^@' | grep -v '^$' | sort | uniq -c | sort -rn > "$A/churn60.txt"
if [ "${AUDIT_LINTS:-1}" = 1 ]; then
  cargo clippy --locked --manifest-path crates/Cargo.toml --workspace --all-targets --keep-going --message-format=json -- --cap-lints warn > "$OUT/clippy.json" 2> "$OUT/clippy.err" || true
  CARGO_TARGET_DIR="$OUT/target-fw" RUSTFLAGS="--force-warn dead_code --force-warn unused_imports" \
    cargo check --locked --manifest-path crates/Cargo.toml --workspace --lib --keep-going --message-format=json > "$OUT/forcewarn.json" 2> "$OUT/forcewarn.err" || true
fi
python3 "$HERE/lints.py" "$OUT" "$A"
python3 "$HERE/summarize.py" "$A" > "$A/summary.txt"
python3 "$HERE/dupes.py" "$ROOT" "$A/dupes-rs.json" 30 .rs crates
python3 "$HERE/dupes.py" "$ROOT" "$A/dupes-ts-src.json" 20 .ts,.svt,.mts src
python3 "$HERE/graph.py" "$A" > "$A/graph.txt"
python3 "$HERE/duptests.py" "$A" > "$A/duptests.txt"
python3 "$HERE/orphans.py" "$A" "$ROOT"
python3 "$HERE/nurbs_layers.py" "$A" > "$A/nurbs-layers.txt"
python3 "$HERE/brep_families.py" "$A" > "$A/brep-families.txt"
node "$HERE/audit-ts.mjs" "$ROOT" "$A/ts.json" src scripts tests benchmarks tools examples
python3 "$HERE/ts_reach.py" "$A"
python3 "$HERE/gen_inventory.py" "$A" "$DOCS/inventory"
python3 "$HERE/gen_inventory_ts.py" "$A" "$DOCS/inventory"
python3 "$HERE/gen_duplicates.py" "$A" "$DOCS/inventory"
python3 "$HERE/gen_inventory_index.py" "$A" "$DOCS/INVENTORY.md"
echo "audit data: $A"; echo "docs: $DOCS/INVENTORY.md, $DOCS/inventory/"
