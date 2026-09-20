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
