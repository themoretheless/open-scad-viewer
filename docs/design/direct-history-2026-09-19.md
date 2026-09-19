# Direct history: retain snapshot sizes

`DirectHistory` previously kept documents and their sizes in parallel arrays.
Undo discarded the known size, moved the document to the redo stack, and then
serialized the restored document again solely to recover its size. Redo did
the same. Both also performed the required defensive JSON clone for callers.

Each private snapshot now owns `{ document, characters }`. Moving a snapshot
between current, undo and redo states preserves its known size. No retained
JSON strings, content hash cache or shared mutable document references were
added. Validation, JSON normalization, no-op commit detection and defensive
copies remain unchanged.

The previous bounds are preserved: 80 past snapshots, at most 16,000,000
serialized characters except that one previous snapshot is always retained.
This is not a precise heap-byte bound and does not include current/redo state.
The change does not claim to fix total memory budgeting.

## Reproduction

`node --import tsx benchmarks/direct-history.mts [output-json]`

Two herringbone gear documents, three warmups and nine measured undo/redo
cycles. Timings include the returned defensive copy. Fixture construction,
commit and complete output comparison are outside the timer. Local environment:
Apple M4 Max, macOS arm64, Node v22.23.2. No build/test process ran alongside
the measurements.

| Fixture | Serialized characters | Undo before | Undo after / repeat | Redo before | Redo after / repeat |
| --- | ---: | ---: | ---: | ---: | ---: |
| 12 teeth, 158 faces | 1,226,273 | 28.43 ms | 12.46 / 12.40 ms | 29.55 ms | 12.34 / 12.26 ms |
| 32 teeth, 418 faces | 3,282,120 | 54.00 ms | 33.96 / 34.20 ms | 53.86 ms | 33.79 / 34.04 ms |

The larger fixture improves about 37% in both directions. This is local API
latency, not a measured browser frame-time or whole-application speedup. The
smaller baseline has visibly more variance in its raw samples; do not generalize
its larger percentage to other documents. Input/output SHA-256 values match
across baseline and both after runs.

Raw evidence: `tmp/performance/direct-history-before.json`,
`direct-history-after.json`, `direct-history-after-repeat.json`. Reports include
all samples and source/kernel fingerprints.

## Verification

115 tests passed across 12 targeted suites after the history change, including
new count/size retention, undo/redo cycles, branching, no-op/invalid commit
atomicity, and caller isolation tests. Strict TypeScript and `git diff --check`
passed. Log: `tmp/performance/direct-history-tests.log`.

Before the history change, a full suite verified the B-rep contract updates:
3,203 passed and nine historical manifest/qualification fingerprint tests
failed. That full run is not evidence for the subsequent history refactor;
the 115 targeted tests above are. Log: `tmp/performance/brep-contract-full-tests.log`.

A subsequent full run including the history refactor and bounded inspection
cache passed 3,216 tests with the same nine historical qualification failures;
see [the cache report](brep-inspection-cache-2026-09-19.md) for current evidence.

## B-rep refusal expectations

Two old negative tests were corrected only after checking the current native
implementation and numerical results:

- A box `[-1,1]^3` sits inside the hole of a torus with major radius 4 and
  minor radius 1. Despite overlapping AABBs, their union has two bodies, preserves
  every authored surface, and has volume `8*pi^2 + 8`. Serialization and input
  immutability are checked. Coincident torus/box trim-boundary contact is still
  explicitly refused in a separate test.
- Two radius-2 spheres one unit apart produce a rational eight-face lens with
  volume `27*pi/4` and centroid `(0.5,0,0)`. Tests check rational carriers,
  closed consistently oriented display meshes at detail 1/2/4, increasing mesh
  volume below the analytic result, serialization and native execution disposal.

Neither result is promoted to certified solid geometry. Inspection still
reports sampled agreement and `solidGeometryStatus: not_certified`. Existing
unsupported hull/offset and invalid execution-entry tests remain.
