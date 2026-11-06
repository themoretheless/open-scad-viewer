# Algebraic roots, polynomial contacts and standard material Boolean

Native Rust qualification, 2026-11-03. This extends the previous closed-contact
milestone. It does not qualify universal NURBS/NURBS Boolean or arbitrary
singularity stratification.

## Positively qualified changes

- The formerly unproved flat quartic loop is now certified as one complete
  embedded closed branch with two original procedural trims:
  `(u-.5)^4 + (v-.5)^4 = 1/256`, between the independent carriers
  `z_first=u^4/8` and `z_second=z_first+gap`. The original bound of 64 charts,
  at most eight repair proposals, and coverage depth 32 are retained.
- Complete original-product root coverage may use an exact 2D reduction when
  both original supports have the identical injective affine XY frame. Exact
  rational coefficients prove the parameter correspondence. Closed cells are
  either inside an actual original root fiber, strictly sign-excluded, or have
  their entire zero image enclosed inside an actual fiber by a rational
  mean-value/derivative bound. This is complete zero-set coverage, not sampling.
- Native Krawczyk root operators may use the exact global coordinate map
  `T(x,y,z)=(x,y,z-H_first(x,y))`. Its polynomial inverse is explicit and its
  determinant is one. The original source surfaces remain owned and visible.
  All transformed poles must be exactly representable: rounding refuses this
  conditioning attempt. Each replay checks the full coefficient construction.
  Original-domain endpoint checks, original support immersion and whole-support
  injection are retained. Rank transfers at roots through the invertible DT.
- Numerical tangent boxes, centered station ordering and proposal deduplication
  only produce candidates. No Newton point, repeated-seed decision or fitted
  curve authorizes a missed-root, topology or material claim.
- Exact polynomial graph-order certificates support nonseparable algebraic
  contact sets. The complete original gap is checked as a positive scalar
  times a reconstructed rational polynomial square, or by a complete closed
  Bernstein sign partition. A degree-six/four cusp contact
  `720*((u-.5)^3-(v-.5)^2)^2` is positively checked, including operand sign
  reversal refusal. No claim that all nonnegative polynomials are decided.

## Standard Boolean integration

`operations::boolean` now invokes the source-bound graph-order material proof
before the general retained-region author. It verifies EVERY original boundary
face is below/above the selected barriers, original exact saved edge/pcurve
agreement, bounded shell forests and the projection domain of the sign proof.
A bounded material cannot cross its graph barrier: z-H has nonzero z derivative,
so its forbidden extremum cannot lie in the material interior. Ordered graph
barriers therefore prove disjoint open material interiors even at a singular
algebraic contact. A nonzero polynomial gap has no open XY zero patch, excluding
shared face area. All these obligations are checked again by receipt replay.

The crossing contact `gap=(x-.5)^2*(y-.5)^2` between two curved graph solids is
qualified through the STANDARD Boolean entry point for all four operations:
intersection is empty, difference preserves the minuend, union/XOR concatenate
the two original bodies. Reversed operand difference/intersection are checked.
Union/XOR at lower-dimensional contacts are a checked compound of separately
embedded bodies; no single global embedded boundary atlas is asserted. No new
poles, trims, facets or approximate intersection edges are authored by this path.

## Validation

144 distinct frozen native tests passed, zero failed:

| Group | Passed |
| --- | ---: |
| NURBS integration: roots, closed branches, contacts, coverage, transitions, trace | 91 |
| Internal surface/surface proof and corrupt-receipt checks | 35 |
| B-rep standard Boolean, retained regions, shell forests and original graphs | 18 |

Fifteen selected tests also passed on the current shared checkout in a separate
fresh target directory: nine closed/root tests, three polynomial-order tests and
three standard Boolean tests. These replays are not extra distinct tests.
The source snapshot excludes unrelated dirty work; live replay tests integration
with that work separately. Source hashes, counts and full logs are adjacent.

The selected old retained-region suite includes original hole rims, original
seams, independent carriers, irrational endpoint refusals and XOR decomposition.
Negative checks cover bad projection domains, true positive-volume overlap,
altered source/result bindings, false square claims, omitted coverage cells,
forged exclusions, corrupt root operators and exhausted work.

The frozen source is `/private/tmp/osv-algebraic-boolean-source-20261103`, built
from the prior immutable qualification snapshot plus the task sources and the
committed identity/codec module split needed by the current curved-module parent.
The dedicated frozen target is `/private/tmp/osv-algebraic-boolean-target-20261103`;
the independent live target is `/private/tmp/osv-algebraic-live-target-20261103`.
Tests retain debug assertions: nurbs-core/brep-core opt-level 0, dependencies
opt-level 1. This is functional proof qualification, not performance evidence.

API-map, scoped formatting and diff whitespace checks pass. WASM/dist, browser,
STEP and a full workspace run are not part of this qualification.

## Remaining scope

This is still not an arbitrary NURBS/NURBS volumetric Boolean author. New
transverse algebraic intersections do not yet become ordinary Model algebraic
edges and split retained regions. The native closed-root owner publishes
procedural original root selectors, not fitted NURBS intersection curves.
Arbitrary singular root decomposition, non-graph rational/multispan carriers
outside the admitted sufficient proofs, and general coincident-face cancellation
remain unproved. Polynomial order is a sufficient proof, not a universal sign
or algebraic-set decision procedure.

World-error fitting through a nonlinear conditioning map explicitly refuses
until its error transport is independently bounded. Original world Jacobians
are retained and this refusal has a regression. It does not affect original
unconditioned trace certificates or the exact original root selectors.
