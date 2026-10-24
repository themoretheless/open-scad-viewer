#!/usr/bin/env bash
# Append-only kernel re-freeze for a kernel-changing PR (`npm run refreeze:kernel -- "<purpose>"`).
# Detects the next ownRust / G0 / G1 versions from the newest existing records, then:
# own-rust vN evidence -> frozen copy of the live refresh executor -> version bumps ->
# FROZEN_ARCHIVES pins -> review note -> `refresh --check` and `refresh --write`.
# Run `npm run build:geometry` first; pass --build to have this script run it.
# Extra crate dirs to bind in the own-rust evidence may follow the purpose.
set -euo pipefail
# Frozen kernel bytes are recorded from the linux-x86_64 builder: rustc embeds host
# path separators, so Windows/macOS builds differ. GNU sed/sha256sum are also required.
[ "$(uname -s)" = Linux ] || { echo "refreeze-kernel.sh must run on Linux (the recorded linux-x86_64 builder); use WSL, a container or CI" >&2; exit 2; }
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
BUILD=0; if [ "${1:-}" = "--build" ]; then BUILD=1; shift; fi
[ $# -ge 1 ] || { echo 'usage: refreeze-kernel.sh [--build] "<purpose>" [extra crate dirs...]' >&2; exit 2; }
PURPOSE=$1; shift; EXTRA_DIRS=("$@")
DATE=$(date -u +%F)
latest() { ls $1 2>/dev/null | sed -E "s#$2#\\1#" | sort -n | tail -1; }
pN=$(latest 'scripts/release-own-rust-v*.mjs' '.*release-own-rust-v([0-9]+)\.mjs')
pF=$(latest 'docs/qualification/g0-toolchain-fingerprints-v*.json' '.*fingerprints-v([0-9]+)\.json')
pP=$(latest 'docs/qualification/semantic-manifold-g1-plan-v*.json' '.*g1-plan-v([0-9]+)\.json')
N=$((pN+1)); F=$((pF+1)); P=$((pP+1))
[ -z "$(git status --porcelain -- docs/qualification scripts tests .github)" ] || { echo "qualification paths have uncommitted changes" >&2; exit 1; }
[ $BUILD = 0 ] || npm run build:geometry
ppF=$((F-2)); ppP=$((P-2))
sha() { sha256sum "$1" | cut -c1-64; }
req() { [ -e "$1" ] || { echo "missing $1" >&2; exit 1; }; }
req scripts/release-own-rust-v$pN.mjs; req docs/qualification/own-rust-cad-v$pN.json
req docs/qualification/g0-toolchain-fingerprints-v$pF.json; req docs/qualification/semantic-manifold-g1-plan-v$pP.json
req docs/qualification/g0-v$pF-g1-v$pP-refreeze-status-v1.json; req docs/qualification/g0-v$pF-g1-v$pP-refreeze-review.md
[ ! -e scripts/release-own-rust-v$N.mjs ] || { echo "v$N release script exists" >&2; exit 1; }
[ ! -e scripts/refresh-qualification-fingerprints-v$pF-v$pP.mjs ] || { echo "frozen copy v$pF-v$pP exists" >&2; exit 1; }
grep -q "own-rust-cad-v$pN.json" scripts/refresh-qualification-fingerprints.mjs || { echo "live refresh script is not at own-rust v$pN" >&2; exit 1; }

# 1. own-rust evidence vN (new release script = previous one with versions, date, purpose, builder and crate list bumped)
sed -e "s/own-rust-cad-v$pN/own-rust-cad-v$N/g; s/own-rust-cad-v$((pN-1))/own-rust-cad-v$pN/g" \
    -e "s/recordedAt:'[0-9-]*'/recordedAt:'$DATE'/" \
    -e "s/builder:'[a-z0-9-]*'/builder:'linux-x86_64'/" \
    scripts/release-own-rust-v$pN.mjs > scripts/release-own-rust-v$N.mjs
node - "$N" "$PURPOSE" "${EXTRA_DIRS[@]}" <<'JS'
const fs=require('fs');const [N,purpose,...dirs]=process.argv.slice(2)
const path=`scripts/release-own-rust-v${N}.mjs`;let s=fs.readFileSync(path,'utf8')
s=s.replace(/purpose:'(?:[^'\\]|\\.)*'/,`purpose:${JSON.stringify(purpose).replace(/^"|"$/g,"'")}`)
for(const d of dirs){ if(!s.includes(`'${d}'`)) s=s.replace(/(\['ls-files',(?:'crates\/[^']*',)*'crates\/[^']*')\]/,`$1,'${d}']`) }
fs.writeFileSync(path,s)
JS
node scripts/release-own-rust-v$N.mjs

# 2. frozen copy of the live refresh executor (bytes of the previous version)
cp scripts/refresh-qualification-fingerprints.mjs scripts/refresh-qualification-fingerprints-v$pF-v$pP.mjs

# 3. version bumps: highest version first so nothing is bumped twice
bump() { sed -i -e "s/\bv$pF\b/v$F/g; s/\bv$pP\b/v$P/g; s/\bv$ppF\b/v$pF/g; s/\bv$ppP\b/v$pP/g" "$@"; }
bump scripts/refresh-qualification-fingerprints.mjs tests/g0ToolchainFingerprints.test.ts tests/g1GithubActionsQualification.test.ts \
     scripts/g1-github-actions.mjs scripts/run-g1-candidate-clean-fragment.mjs scripts/run-g1-ubuntu-docker-fragment.mjs \
     .github/workflows/g1-qualification-clean.yml
sed -i -e "s/own-rust-cad-v$pN/own-rust-cad-v$N/g" scripts/refresh-qualification-fingerprints.mjs
sed -i -e "s/previousFingerprintSha256: '[0-9a-f]*'/previousFingerprintSha256: '$(sha docs/qualification/g0-toolchain-fingerprints-v$pF.json)'/" tests/g0ToolchainFingerprints.test.ts

# 4. the previous version's files become immutable archives
tmp=$(mktemp)
{
  printf "  'scripts/refresh-qualification-fingerprints-v%s-v%s.mjs': '%s',\n" $pF $pP "$(sha scripts/refresh-qualification-fingerprints-v$pF-v$pP.mjs)"
  printf "  'docs/qualification/own-rust-cad-v%s.json': '%s',\n" $pN "$(sha docs/qualification/own-rust-cad-v$pN.json)"
  printf "  'docs/qualification/g0-toolchain-fingerprints-v%s.json': '%s',\n" $pF "$(sha docs/qualification/g0-toolchain-fingerprints-v$pF.json)"
  printf "  'docs/qualification/semantic-manifold-g1-plan-v%s.json': '%s',\n" $pP "$(sha docs/qualification/semantic-manifold-g1-plan-v$pP.json)"
  printf "  'docs/qualification/g0-v%s-g1-v%s-refreeze-status-v1.json': '%s',\n" $pF $pP "$(sha docs/qualification/g0-v$pF-g1-v$pP-refreeze-status-v1.json)"
  printf "  'docs/qualification/g0-v%s-g1-v%s-refreeze-review.md': '%s',\n" $pF $pP "$(sha docs/qualification/g0-v$pF-g1-v$pP-refreeze-review.md)"
} > "$tmp"
sed -i -e "/^export const FROZEN_ARCHIVES = Object.freeze({$/r $tmp" scripts/qualificationFrozenArchives.mjs
rm -f "$tmp"

# 5. review note
cat > docs/qualification/g0-v$F-g1-v$P-refreeze-review.md <<MD
# G0 v$F / G1 v$P re-freeze review

$PURPOSE

Preserve ownRust$pN, G0v$pF, G1v$pP and all earlier records byte for byte. Bind current source, observed ownRust$N WASM, fixtures, transport tests and executors in new append-only records. No previous G1 result is imported: all 4740 clean semantic work units remain incomplete for this version. No G0/G1 approval or production cutover is claimed.
MD

node scripts/refresh-qualification-fingerprints.mjs --check >/dev/null
node scripts/refresh-qualification-fingerprints.mjs --write --recorded-at "$DATE" --reason "$PURPOSE" >/dev/null
echo "Re-frozen: ownRust v$N, G0 v$F, G1 v$P (previous v$pN / v$pF / v$pP pinned in FROZEN_ARCHIVES)."
