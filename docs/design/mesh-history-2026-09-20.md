# Mesh history serialization cost

## Change

MeshHistory previously serialized its entire retained document array on every
commit, and again for each size-driven eviction. It now follows DirectHistory's
snapshot-size pattern: validated documents carry their serialized character count
through undo and redo. Commit sums at most 81 scalar counts, rather than rescanning
all historical geometry. No new generic history abstraction is introduced.

The original semantics are preserved: 80 past snapshots, a 24,000,000-character
serialized-array budget, one retained snapshot even when oversized, no-op commit
preserves redo, branch commit discards redo, and all exposed documents are clones.
The count includes JSON brackets and commas (`sum + snapshotCount + 1` for a
nonempty array). It is not an actual heap-byte limit. Constructor now additionally
serializes the validated initial snapshot once to obtain its count.

## Reproduction

`node --import tsx benchmarks/mesh-history.mts BASELINE_PATH`, where BASELINE_PATH
is meshEditing.ts from `84b74ec0` with its relative imports resolvable. The report
includes source hashes, Node/architecture, raw samples and undo-history hashes.
Each variant receives identical name-only edits on mesh documents. Nine measured
commits alternate baseline/candidate order after zero or 80 setup commits. Setup
and exact document/undo/redo parity checks are outside timing; validation, snapshot
allocation and GC are included. Initially empty histories grow during sampling.

Two isolated runs on Node v22.23.2, arm64. Median commit time, milliseconds:

| Triangles | Setup commits | Retained undo | Baseline | Candidate | Repeat baseline | Repeat candidate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 0 | 9 | 1.268 | 0.644 | 1.101 | 0.653 |
| 1,000 | 80 | 80 | 9.120 | 0.615 | 9.172 | 0.622 |
| 10,000 | 0 | 9 | 15.155 | 7.350 | 13.214 | 7.374 |
| 10,000 | 80 | 59 | 155.214 | 7.505 | 155.581 | 7.464 |

Full histories improve about 15x and 21x respectively. The larger fixture retains
59, not 80, undo states because the character limit is reached first. This is
history commit cost, not full editing latency, sculpt performance or application
FPS. Full undo/redo documents and retained counts matched the baseline in both
runs.

## Verification

13 focused mesh-history/editing tests passed. Regression coverage includes clone
ownership, no-op and branched commits, the 80-entry limit across undo/redo, and
four snapshots of 5,999,998 versus 5,999,999 characters. Including array punctuation,
those fall immediately below and above the 24-million limit.

UI type checking, Vite build and dist verification passed (89 artifacts;
5,782,641 asset bytes plus 9,697,456 raw WASM bytes). The integrated full run passed
3289 tests and failed the same nine evidence gates, with no new local failures
(345 files; 110.89 seconds). Log: `/private/tmp/osv-mesh-history-integrated-tests.log`.
