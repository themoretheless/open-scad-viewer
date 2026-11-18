# Closed central network candidate — 2026-11-03

**Status: closed incidence and intended network G2 proven on the fixture;
curvature cap rejected. Not an admitted fillet/Solid.**

The fixture has stock 10 mm, radii 0.1/0.2/0.35 mm, quartic profiles,
runout 0.1 mm, span 2 mm, cyclic rim origin 1, handle 0.03125 mm.
The central proposal consists of five degree-10/9 rational patches.

## What was constructed and checked

- Ten actual outer 3D edges have complete agreement with the unchanged
  original supports. This closes the four previously unresolved bindings.
- Four central internal edges are stored once with common root vertices
  and opposite face-local traversal. No proximity welding is used.
- Original strip faces retain their surfaces and receive segmented near
  boundaries. Six original stock surfaces are retained byte-for-byte;
  their loops are trimmed through the shared edges. Three far planar
  closure faces complete the bounded construction.
- One closed candidate: **17 faces, 43 edges, 26 vertices**. Whole-model
  edge/lift closure and every face UV trim pass. Construction uses 6980
  cells under the 100000-cell allowance.
- **All ten exterior network G2 seams are freshly checked on the FINAL
  sewn coedges**, including material-normal orientation, with no pending
  cells. Four internal G2 seams belong to the unchanged central patches.
  No claim is made that every original sharp or far closure edge is G2.

## Root and source-binding fixes

Exact rational comparison now composes an original single tensor-Bezier
support with its affine pcurve, retaining both coordinates and weights.
For actual affine planes, rational pcurves are lifted from the exact original
plane coefficients; a non-affine fourth corner cannot use this shortcut.
Planar pcurves are authored by projecting the actual shared rational edge,
retaining its weights/clock, then freshly verifying the entire lift.

A source strip failed trim closure because two uses of the same 3D root
had UV values 0.14743885437825113 and 0.1474388543782511. Face-local UV
endpoints are now owned once by the actual vertex identity. Every edit
is bounded by the existing UV tolerance (1e-8), followed by fresh complete
3D lift and final G2 checks. Singular central pole charts are excluded
from this regular-chart root canonicalization.

Vertical boundary verification uses exact U/V index exchange, with no
coordinate recomputation or rounded clock reflection. All optional exact
arithmetic remains operation/bit bounded; inconclusive paths do not admit
geometry. Neither positional nor G2 tolerances were increased.

## Validation

- **64 CAD region-replacement tests passed**, including closed incidence,
  preserved original stock supports, ten final sewn G2 contacts, unchanged
  input source, root-ownership/budget refusals and curvature-cap rejection.
- **54 curve/surface agreement tests passed**.
- **54 G2 tests passed**, including negative geometry, zero speed,
  clock orientation and numerical-overflow fallback cases.
- Native diagnostic completed with exit 0. `native-stages.log` records the
  successful NEW network and its failed curvature gate. The example also
  runs the old single-surface constructor afterwards; its internal G2
  refusal does not concern this five-patch body.

## Mandatory remaining qualification

The actual closed candidate still has absolute principal curvature
**at least 15.595709250593478 /mm**, exceeding the required **10 /mm**.
The CAD regression explicitly retains this rejection: closed incidence
and seam G2 cannot promote the shape to an admitted feature.

The central/neighboring shape must be rebuilt under the curvature cap.
Whole-surface injectivity, global self-intersections, material volume,
wall thickness, general NURBS/5+ junction support, independent STEP
measurements and WASM/UI/history acceptance remain separate work.
The present construction is a bounded stock-aligned trihedral fixture,
not arbitrary B-rep trimming or a universal fillet implementation.

Final source snapshot confirmation: the two scope guards also require actual
affine retained stock edges and an interior stock-axis far cutter plane.
Unsupported curved retained edges are refused rather than straightened.
The confirmed CAD filter passes **64/64** on this snapshot. A new unrelated
shared-worktree unit-test import briefly prevented compilation; its
intra-crate path was corrected concurrently, and the complete filter was
rerun successfully. No unrelated source work was discarded.

Final native output explicitly records `final_g2_seams=10`,
`trims_valid=true`, `solid_admitted=false` and the failed curvature witness
on the actual sewn candidate. The largest final exterior normal bound is
1.4762307932023531e-6 and tensor bound is 7.629861530811035e-5 /mm.
