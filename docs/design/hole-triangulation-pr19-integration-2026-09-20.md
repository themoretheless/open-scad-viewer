# PR 19 integration reconciliation

Source branch head: `7cbd425b7b05daf1e6537540940e8767ebfc6efc` (fetched remote
and local agree). Its two commits are `5022e486` and `7cbd425b`. The original
PR targets the old optimization branch. Main before reconciliation: `624304bd`.

## Disposition

The branch addresses aligned hole grids and the old 2048-vertex admission
limit. Those capabilities are already implemented by the sector-aware,
boundary-preserving triangulator on main, with stronger topology tests and
measured native/WASM results documented in
[profile triangulation](profile-triangulation-2026-09-19.md).

All three changed files were reviewed:

| Branch change | Resolution |
| --- | --- |
| `planar-geometry/src/triangulation.rs`: ray/edge-interior bridge candidates and inserted positions | Replaced by selecting the correct local sector at existing bridge endpoint occurrences. Keep exact authored boundary coordinates, with no new boundary vertex that adjacent B-rep faces do not share. The branch implementation fails the existing boundary-conformance tests. |
| Same file: 4096-vertex cap | Already present, but main accounts for duplicated bridge endpoints before doing bridge work. It additionally charges visibility and ear scans against an 8,000,000-credit budget. |
| Same file: all-pairs intersection preflight | Not copied: it is an extra quadratic scan outside the current shared work budget, and does not fix the branch's boundary-conformance/degenerate-output failures. Main retains its bounded triangulation refusal behavior. The branch's bow-tie test is already ported; this reconciliation adds a nonzero-area crossing case and a deterministic corpus of 500 vertex permutations, with more than 300 proper-crossing inputs required and checked for refusal. This finite corpus is not a proof of rejection for every malformed profile or numerical scale. |
| Same file: aligned square grids, bow tie, oversized grid tests | Ported in `f3a24c60`, with exact boundary/paired-interior-edge validation in addition to area checks. |
| `polygon-core/src/solid/primitives.rs`: successful 16/64-hole prism subtraction | Superseded by current 4/16/36/64/100-hole checks of closed topology and analytic volume. Import and assertion formatting changes carry no separate behavior. |
| `docs/design/csg-scaling-2026-09-19.md`: branch capability and measurement claims | Historical document stays historical. The follow-up profile report documents the current algorithm, successful 100-hole WASM workload, limitations and measured regressions as well as wins. No unsupported claim that triangulation makes general BSP linear is introduced. |

## Reproduced Rejection Of The Original Implementation

The retained isolated comparison tree is
`/private/tmp/osv-triangulation-check.e41uqF`. Its triangulation source Git blob
is `696d668571d15a4a8e01186e024978461627fd78`, identical to the PR head.
Its boundary validation helper matches main's blob
`5e96ea54e2e15c07cef2eec3f9beb1d1589c19dc`.

Re-running its four integration tests reproduces four failures:

- Aligned hole grids: new boundary coordinate.
- Collinear boundary segments and orientations: new boundary coordinate.
- Reflected/scaled/reordered holes: new boundary coordinate.
- Degenerate input: a collinear three-point profile incorrectly succeeds.

This is an isolated test of the original algorithm against the stronger
integration corpus, not an assertion that the branch's own tests fail.
The faulty implementation must not overwrite the current triangulator.

## Current Verification

- `cargo test --locked --manifest-path crates/Cargo.toml -p planar-geometry
  -p polygon-core`: 282 tests passed, zero failed. Includes nine profile
  integration tests and both newly added crossing cases. Log:
  `/private/tmp/osv-pr19-native-tests.log`.
- 67 Vitest tests passed across `openscadParser`, `brepAnalytic` and
  `brepProfileExtrusionAudit`; this exercises the shipped WASM for the real
  16/36/64/100-cutter prism cases and holed B-rep extrusion.
- No production source or WASM bytes change in this reconciliation. It makes
  no new performance claim and does not resolve the nine known qualification
  binding failures from the latest full-suite run.

An explicit reconciliation merge preserves main's verified replacement and
retains both original branch commits in its ancestry. It records integration
of the useful work, not adoption of the rejected Steiner bridge algorithm.
