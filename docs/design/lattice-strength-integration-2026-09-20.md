# Lattice Strength Integration Gate

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
Isogrid and the remaining print/strength changes are not integrated yet.

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

### Full Local Regression

`vitest run --maxWorkers 2` with local HTTP socket access completed in
108.49 seconds: 3327 passed and 9 failed across 347 files (343 passed,
4 failed). All failures are the existing qualification binding set:
`engineManifest` (1), `g0ToolchainFingerprints` (6),
`g1GithubActionsQualification` harness binding (1), and
`qualificationPlanArtifact` (1). No test timed out in this run, and the HTTP
tests ran without sandbox socket failures. The suite is not green; immutable
qualification archives were not rewritten to hide these failures.
Log: `/private/tmp/osv-centered-lattice-full-tests.log`.
