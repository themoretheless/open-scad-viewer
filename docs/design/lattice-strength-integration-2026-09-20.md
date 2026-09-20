# Lattice Strength Integration Gate

Follow-up: [native nominal opening fit](lattice-opening-fit-2026-09-20.md)
selectively replaces the legacy FDM opening optimizer. Other remaining
strength/scenario features are still subject to the gates below.

Follow-up: [explicit truss scenarios](truss-scenarios-2026-09-20.md) adds bounded
case selection and signed combinations through the shared worker, refusing the
legacy support-union behavior. Workbench controls and finished-solid model
derivation remain outstanding.

Follow-up: the [nominal axial graph workbench](nominal-truss-workbench-2026-09-20.md)
now exposes editable node restraints, wrenches, cases, combinations and reports.
It explicitly distinguishes the bounding-box graph from finished-solid strength;
automatic derivation of the latter remains outstanding.

## Scope

Reviewed PR #7, `cursor/lattice-strength-analysis-274c`, at
`7b3afccf379aead6151c7ceaf9b633e40eb62924`, against main `418aea9e`.
The branch is not merged. Its scenario definitions, FDM settings and new
lattice patterns remain candidates for selective integration. Current main
already delegates lightening and spatial graph generation to Rust; replacing
those wrappers with the branch's older TypeScript geometry would undo that work.

## Reproduced Singular-System Failure

Extract the two branch files `src/services/latticeTrussFea.ts` and
`src/services/latticeStrengthScenario.ts` into an isolated directory. Run this
probe through the repository's `node --import tsx` runtime:

```ts
import {solveLatticeTruss} from './src/services/latticeTrussFea'

const result = solveLatticeTruss({
 graph: {nodes: [[0, 0, 0], [0, 0, 10]], edges: [[0, 1]]},
 E: 2000, sigmaAllow: 30, safety: 2, relativeDensity: 0.1,
 section: {kind: 'circle', a: 2},
 supports: [{id: 'base', kind: 'fixed', normal: [0, 0, -1], ru: '', en: ''}],
 actions: [{id: 'lateral', kind: 'force', direction: [1, 0, 0], magnitude: 100,
  at: [0, 0, 10], ru: '', en: ''}],
})
console.log(result)
```

Observed and asserted on 2026-09-20:

- Both nodal displacement vectors are `[0, 0, 0]`.
- `maxDeflectionMm`, `maxUtilization`, `maxBucklingRatio`, `reactionN` are zero.
- `homogenized.EStar` is 2000, equal to the input material modulus.
- `homogenized.sigmaStar` is 30, equal to the input allowable stress.
- `warnings` is empty.

This is an algebraic inconsistency in the implemented pin-jointed model:
the vertical member contributes zero X stiffness, while the free upper X DOF
has a nonzero applied load. No displacement vector can satisfy that equation.
`solveDense` skips a pivot smaller than `1e-9`, then sets the associated
displacement to zero during back substitution. Neither rank deficiency nor
the resulting force residual is checked before returning ordinary results.
This is a numerical regression probe, not a validated structural design result.

## Required Before UI Integration

- Represent invalid, singular and unsupported cases explicitly, without
  emitting successful stress, stiffness or margin recommendations.
- Check finite inputs, graph indices, zero-length members and resource bounds
  before allocating dense matrices or generating a graph.
- Verify equilibrium residuals and known stable fixtures in addition to the
  singular case above; use a scale-aware singularity criterion.
- Validate moment assembly with resultant force and resultant moment tests.
  The current code chooses extrema along the force direction and applies
  opposing forces in that same direction. This requires separate coverage;
  the two-node probe above does not exercise moment loading.
- Compute support reactions from the assembled equations rather than labeling
  half the sum of absolute member forces as a reaction.
- Keep heuristic material/environment factors and lattice rankings clearly
  separate from computed responses and experimentally validated properties.
- Port additional geometry patterns into the current bounded Rust path;
  retain main's wrapper contracts and benchmark representative workloads.

The branch's unique work is preserved. This gate does not establish that every
feature is defective, nor justify dropping the branch or claiming it merged.

## Centered Geometry Port

BCC and octet graph generation are selectively integrated in
`crates/geometry-bridge/src/cad_centered_lattice.rs`, invoked by the existing
Rust CAD lightening path. No strength estimates or rankings are imported.
`bone` and `spatial` retain their existing generation logic and fixtures.
Isogrid is covered by the follow-up below. The remaining print/strength changes
are not integrated yet.

The new module counts both nodes and edges before allocating or generating
them, including body/face centers. The current limits remain 125 nodes and
400 edges. For example, an octet grid of 4x2x2 cells has 113 nodes but 464
edges and must be rejected even though its corner count and total nodes fit.
The main graph entry point still validates mesh, finite cell/jitter/seed and
positive cell size before invoking either implementation.

The workbench offers both patterns, uses spatial shell/wall/print fitting
controls, and hides the ineffective random seed/jitter controls for these
regular graphs. Their output intentionally ignores jitter, seed and the
axis-grid diagonal toggle, matching the branch. UI coverage checks that
switching back to spatial/grid restores the appropriate controls.

Validation on the rebuilt WASM:

- 32 tests across spatial lattice, planar lightening, print fitting and
  workbench UI passed, including closed lighter solids, positive connected
  volumes, shell/open-top and wall/core combinations for both new patterns.
- The Rust topology unit test checks counts, uniqueness and index validity.
- Vue and MCP typechecks passed; Vite and `verify-dist` passed with 91
  artifacts, 5,790,927 asset bytes plus 9,713,167 raw WASM bytes.
- `node --import tsx benchmarks/centered-lattice.mts` matches all node
  coordinates and ordered edges exactly against the pinned PR implementation
  for 196 admitted fixtures; both implementations reject 60 over-budget
  fixtures. Shapes cover 1-4 cells per axis and two offsets.

WASM SHA-256:
`b684472f46f3a72bba6f81725d8462bda31f06e80c402ced04930eebaab3a4f2`.

### Performance Boundary

The benchmark evaluates the original pure TS graph in the same JS realm as
the host, with isolated module exports and unused geometry imports stubbed.
The Rust side times the full existing ABI call including source mesh
transfer and validation. Kernel startup, body construction, serialization of
comparison results and assertions are outside timing. Each case has 20
warmups and 31 alternating samples, repeated in two fresh processes.

| Graph | TS p50 A/B, ms | Rust ABI p50 A/B, ms |
| --- | ---: | ---: |
| BCC, 9 nodes / 8 edges | 0.00633 / 0.00596 | 0.03054 / 0.03058 |
| BCC, 48 nodes / 96 edges | 0.03262 / 0.03338 | 0.04833 / 0.04817 |
| Octet, 14 nodes / 36 edges | 0.00571 / 0.00638 | 0.02375 / 0.02450 |
| Octet, 88 nodes / 352 edges | 0.06208 / 0.06287 | 0.07775 / 0.07733 |

This is not a standalone graph speedup: ABI costs outweigh native generation
on these bounded inputs. Retain the implementation as a feature extension of
the existing native lightening pipeline, not as a performance claim. Within
`cad_lightening`, generation is called inside Rust rather than crossing back
into TS to request the graph. Complete lightening/UI speedup was not measured.
Earlier exploratory measurements with the reference in a separate VM realm
were discarded because that setup biased the comparison.

Reports: `/private/tmp/osv-centered-lattice-same-realm-{a,b}.json`.

### Full Local Regression Before Isogrid

`vitest run --maxWorkers 2` with local HTTP socket access completed in
108.49 seconds: 3327 passed and 9 failed across 347 files (343 passed,
4 failed). All failures are the existing qualification binding set:
`engineManifest` (1), `g0ToolchainFingerprints` (6),
`g1GithubActionsQualification` harness binding (1), and
`qualificationPlanArtifact` (1). No test timed out in this run, and the HTTP
tests ran without sandbox socket failures. The suite is not green; immutable
qualification archives were not rewritten to hide these failures.
Log: `/private/tmp/osv-centered-lattice-full-tests.log`.

## Isogrid Follow-Up

The equilateral triangular isogrid pattern now uses the existing Rust planar
cell generation, clipping, inset and channel boolean path. It is offered in
the workbench with planar channel and print-fitting controls. No qualitative
strength rankings from the old branch are introduced. The nominal padded
grid admission rule (`columns * rows * 2 <= 144`) matches the TS implementation;
finite increasing bounds are checked before the bounded construction loops.

Extending conformance to all planar patterns exposed two existing Rust issues:

- `inset_polygon` iterated indices from the initial polygon but read them from
  the changing clipped result. Insetting the triangle `[[0,0],[1,0],[0,1]]`
  by 2 panicked with `index out of bounds: len is 0 but index is 1`.
  It now offsets original edges and stops when the result is empty. The
  consumed-cell WASM test also calls the kernel again to check recovery.
- Honeycomb disabled X jitter but still applied Y jitter. In a 1x9 domain
  with cell 6, rib 0.2, seed 42 and jitter 0.5, the first clipped opening
  differed from the TS reference by about 0.7726 mm. Honeycomb now disables
  both components; only `web` applies randomization. The expanded corpus
  reproduced 40 honeycomb mismatches before this fix and no mismatches in
  the other planar patterns.

Both issues have regression tests. The consumed-inset native test and the
honeycomb public-WASM test were observed failing before their fixes.
After the source fixes, all 260 `geometry-bridge` library tests passed.

Final rebuilt-WASM verification:

- All 40 targeted planar/spatial lattice, print-fitting and UI tests pass.
- The same-realm reference harness matches 360 planar fixtures across grid,
  triangles, honeycomb, web and isogrid; ordered polygon sizes match and the
  maximum coordinate difference is `1.4210854715202004e-14` (tolerance `1e-9`).
  The 196 admitted and 60 rejected centered graph cases still pass.
- Full local Vitest with socket access: 3335 passed, 9 failed in 107.91 seconds,
  across 347 files. The nine failures are the same qualification binding set
  listed above. No timeout or new failure appeared in this run.
- Vue, MCP and standalone benchmark typechecks pass. `verify-dist` verifies
  91 artifacts: 5,789,948 asset bytes plus 9,713,121 raw WASM bytes.

Final WASM SHA-256:
`b4ad458cd84207f41e788ece11208b1018fa151552e8985512e844116e36a8bb`.

The extended harness also times isogrid through the full cell ABI, using the
same warmup/alternation protocol as the centered graph cases:

| Domain / returned cells | TS p50 A/B, ms | Rust ABI p50 A/B, ms |
| --- | ---: | ---: |
| 16x12 / 15 | 0.02329 / 0.02317 | 0.02933 / 0.02942 |
| 40x20 / 56 | 0.03942 / 0.04096 | 0.06271 / 0.06304 |

Again, there is no standalone ABI speedup. The changes extend the existing
native lightening path and repair geometry semantics; these timings do not
establish a complete lightening or UI performance improvement.
Reports: `/private/tmp/osv-isogrid-final-{a,b}.json`.
Full regression log: `/private/tmp/osv-isogrid-full-tests.log`.

## Truss Numerical Foundation

`mechanics-core::truss` now provides a bounded, typed native solver as the
replacement foundation for the branch's unchecked dense elimination. It is
not yet wired to the geometry ABI or workbench. The old TS strength module
and its heuristic material, moment, support and ranking logic remain unmerged.

The input separates explicit nodal forces, XYZ zero-displacement constraints,
and per-member modulus/area from geometric graph generation. There is no
implicit ground support, inferred face load, pseudoinverse fallback, epsilon
length substitution or automatic stiffness regularization. Inputs are limited
to 125 nodes and 400 unique members before dense matrices are allocated.

The implementation uses pinned `nalgebra` 0.35.0 with default features off and
`std` on. Its [checked Cholesky constructor](https://docs.rs/nalgebra/0.35.0/nalgebra/linalg/struct.Cholesky.html#method.new)
returns no factor for a non-positive-definite matrix. The adapter additionally
equilibrates by the stiffness diagonal, rejects normalized pivots at or below
`1e-12`, and verifies free-DOF componentwise backward residuals at `1e-9`.
The pivot threshold is a refusal heuristic, not a condition-number estimate
or a bound on displacement forward error. Reactions come from restrained
components of `K*u-F`, not a proxy based on member-force magnitudes.

Failures return `TRUSS_INVALID_INPUT`, `TRUSS_NUMERIC_RANGE`, `TRUSS_SINGULAR`
or `TRUSS_RESIDUAL`, without successful-looking displacement/stress values.
Results are linear axial-bar responses only: no bending, moments, buckling,
geometric nonlinearity, FDM qualification or strength recommendations.

Verification:

- Nine new solver tests and five existing section tests pass in both debug and
  release profiles.
- The exact two-node lateral-load regression now returns `TRUSS_SINGULAR`,
  including when its load is zero; no unconstrained mode is silently dropped.
- Analytical axial extension, stress, series-member force and signed support
  reactions pass, as do fully restrained loads and rotated/translated tripods.
- Uniform modulus/load scaling from `1e-100` to `1e100` preserves displacement.
- Near-singular geometry, malformed inputs and numeric overflow fail closed.
- A 125-node, 366-member fixture solves with 366 free DOFs and checked reactions.
- `cargo check --locked --manifest-path crates/Cargo.toml -p mechanics-core
  --target wasm32-unknown-unknown` passes. This is compilation evidence, not
  browser execution, ABI integration or a shipped-WASM size measurement.
- Targeted Clippy (`--all-targets --no-deps -- -D warnings`) passes. Including
  dependency linting fails on 24 pre-existing `planar-geometry` lints; those
  unrelated files and lint policy were not changed.

`bench_truss` measures validation, assembly, Cholesky, residual checks and
reaction/member recovery on independent supported-tripod fixtures. Construction
is outside timing; each case has 20 warmups and 31 samples. Two separate native
release runs on macOS/aarch64 gave:

| Nodes / free DOFs | p50 run 1 / run 2, ms |
| --- | ---: |
| 4 / 3 | 0.00179 / 0.00083 |
| 43 / 120 | 0.14346 / 0.17058 |
| 125 / 366 | 2.78188 / 2.81983 |

The maximum relative residual was `2.56e-16` or less for these fixtures. These
are initial native costs, not a TS comparison, speedup claim, complete lattice
analysis or a certified structural result. Reproduce with:

```sh
cargo build --locked --release --manifest-path crates/Cargo.toml -p mechanics-core --example bench_truss
crates/target/release/examples/bench_truss
```

Reports: `/private/tmp/osv-truss-native-{a,b}.json`.
Before UI integration, resolve the branch's support/load semantics against this
explicit model, test moment resultants, preserve typed failures through the ABI,
and measure the size/transport cost of linking the new numerical dependency.

## Reproduced Scenario Defects

`node scripts/audit-lattice-branch.mjs` reads pinned PR7 commit
`7b3afccf379aead6151c7ceaf9b633e40eb62924` with `git show`, transpiles its two
scenario modules and exposes private load/support helpers for observation only.
Their bodies are unchanged; no legacy implementation is linked into production.
The report includes source hashes and independently sums nodal forces and
position-cross-force moments. Local report:
`/private/tmp/osv-lattice-scenario-audit.json`.

For a 10 mm cube with a requested 100 N mm couple on its +Z face:

| Requested moment, N mm | Assembled moment, N mm | Total force, N |
| --- | --- | --- |
| [100, 0, 0] | [0, 0, 0] | [0, 0, 0] |
| [0, 100, 0] | [0, 0, 100] | [0, 0, 0] |
| [0, 0, 100] | [0, 0, 0] | [0, 0, 0] |

The helper selects extremes along the same direction as the applied forces.
On the face-normal case both extremes are the same node, so opposite forces
cancel exactly; other axes can produce collinear pairs or a wrong moment axis.
A point moment [0, 0, 100] at [10, 0, 10] instead yields force [0, 0, 40] N
and moment [0, -400, 0] N mm about the origin, because a single selected node
falls through to the scalar moment-to-force approximation.

Adding an interior node at [5, 5, 2] makes the -Z pinned-support helper fully fix
that interior node, outside the selected face. Its centroid-nearest-node search
uses all graph nodes rather than the face subset. An empty support list also
silently fixes 12 base DOFs on the cube. Both behaviors differ from the explicit
zero-displacement masks required by the new native solver.

Before porting scenarios, selected node sets must be explicit and nonempty;
support selection must not escape the selected set. Any conversion of a force
and moment into nodal forces must preserve both resultants about the declared
origin. Collinear or single-node selections cannot represent arbitrary pure
couples and must refuse unrealizable loads, not silently introduce a force.
Pin-jointed axial bars still have no rotational DOFs: a nodal force couple is
an explicit loading model, not an implementation of member bending.
