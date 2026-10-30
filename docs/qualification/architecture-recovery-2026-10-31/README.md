# Architecture recovery, 2026-10-31

Status: architecture ratchet repaired; application qualification remains incomplete.
This is a preservation checkpoint, not a release qualification or permission to
publish the candidate kernel as an already qualified provider.

## Structural change

The 69 failures reported at `dac661a46` are removed without increasing source
size limits, crate layer exceptions, geometry tolerances or proof/work budgets.
The 52 older grandfathered size pins remain. Three fixed pins were removed.

Large native tests retain their original module paths in dedicated test files.
Production implementations and algorithms moved into their owning child modules;
TypeScript facades preserve the public exports. Vue macros stay in Vue and the
extracted controllers share mutable state through typed accessors.

A structural census matched all 1943 moved Rust function bodies, accounting for
relative module paths and hashes of included fixture bytes. This census covers
the structural refactor, not the explicitly documented functional repairs:
closed-graph contact serialization, worker result dispatch, source wire defaults,
and the STEP /9 single-product identity condition in the active `cad-step` owner.
All 807 immutable qualification archive identities remain unchanged.

## Completed checks

- Architecture audit: zero failures; changed and new sources measured as well.
- Native library suite, 18 selected packages: 5055 passed, 7 ignored, zero failed.
  This is not a claim that every workspace integration test passed.
- Minimal-feature `nurbs-core` and `brep-core` library check passed.
- Frontend and MCP type checks passed.
- Node `.mjs` suite: 45 passed, one skipped, zero failed after worker entry repair.
- Two focused frontend runs: 35 passed each. These cover worker dispatch,
  renderer pipeline ownership, MeshModeler lifecycle, whole-wall wire validation,
  and preservation of nonempty topology metadata. They are not full-suite results.

## Unresolved qualification

The full frontend run initially reported 24 failed files, 693 passed files and
5 skipped files: 57 failed, 6469 passed, 10 skipped tests. Subsequent focused fixes
are recorded separately; there has not yet been a successful complete rerun.
Library structure verification passes (52 packages, 18 native kernels). The
separate algorithm-owner census is not qualified: the moved MeshModeler
interaction retains its existing owner decision, but other pre-existing numerical
files and stale decisions still require review and the source census changed.

Remaining categories include historical fingerprint/freeze integrity, current
runtime artifact binding, changed native proof reports, sweep admission and STEP
identity. Do not replace these failures with blanket skips or relaxed proof gates.

The Vite bundle was generated. A dedicated viewport chunk reduced DirectModeler
from 456561 to 441003 bytes under the existing 454000-byte budget.
Distribution verification still fails the geometry payload budget. A controlled
build of the input commit, with only the missing ClosedGraph match arm repaired,
already produced 5354665 embedded source bytes versus 5356999 after refactoring
(+2334). The optimized raw WASM grew from 15885999 to 15891395 bytes (+5396).
Both use the same toolchain and Binaryen 116 flags. No delivery budget was raised.
The old 4680000-byte geometry chunk cap therefore predates the input as well as
this refactor; this finding does not grant current artifact qualification.

The standalone original-input probe reproduces the STEP identity loss before
this refactor. The current whole-wall validator now accepts the native serde
empty metadata defaults while retaining authored geometry and nonempty metadata.

Logs are stored as deterministic gzip files with uncompressed SHA256 hashes in
`checks.json.gz`. Historical qualification archives and the old runtime provider
qualification record were not rewritten. Other active worktrees and stashes were
not collected, cleaned or modified. Main must not be pushed while these checks
remain unresolved.
