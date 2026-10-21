# Qualified end boundaries and an owned open fillet face

## Implemented

`curve_surface_agreement::lift` proposes a 3D trace by tensor composition or
an original isoparametric restriction. It never modifies the authored pcurve.
Optional shared endpoint coordinates are inserted BEFORE the full original
surface/pcurve agreement check. Only a passing actual returned definition is
exposed. A failed proposal or unsupported trace is not a proof that no lift
exists. Cells remain shared and bounded at 100,000.

`contact_end::connect` joins two corresponding restricted-contact endpoints on
the actual original skin. It retains their clamped 3D endpoint coordinates
exactly, lifts the proposed skin pcurve with those endpoints, and projects the
actual curve into the original end-face chart with independent whole-interval
admission. Both surface lifts are then checked together. Original end-face trim
validity and boundary lifts are audited. Full interval trace boxes must remain
inside end-face material: valid endpoints do not excuse a curve through a hole.
Stored root reports identify the end/cutter selection but do not authorize any
of these fresh checks. Every numeric proposal remains separate from evidence.

`contact_end::patch` constructs one open B-rep face on the UNCHANGED original
skin, with four edges, four shared corner vertices and complete persistent IDs.
Both contact curves, both end curves, actual saved reversed pcurves and a
positive simple trim loop are checked. Both end constructions and final loop/
lift checks share 100,000 cells and 1,000,000 exact work. The model deliberately
has no Body and an open shell. It is not relabeled a closed fillet body.

## Current evidence

Three end-boundary integration controls passed:

- Plane/plane transition: both actual end curves preserve contact endpoints,
  continuously agree with skin and end faces, and produce the four-vertex
  open patch. Collapsed ends, wrong end-face identity and insufficient work
  refuse without changing source models.
- Plane/rational-cylinder transition: both end curves and the same open
  four-edge topology pass on the actual prepared G2 skin.
- Hole-crossing end curve: both original contact endpoint roots are valid
  material points, but the connecting curve crosses an interior hole. Fresh
  whole-curve material classification refuses it without modifying the face.

The control end planes extend beyond the contact endpoints. They do not prove
boundary-stratum ownership on actual body end-cap edges.

All 45 related B-rep integration tests and three lift/composed-degree tests
passed (48 total). The native geometry-bridge check passed. This is neither a
WASM rebuild nor a worker/UI/runtime acceptance result.

## Remaining full objective

Integrate the open skin face into retained original support/end-face sectors.
Interior body-edge root binding and shared split lineage are now implemented
in `contact_bind`; see `../contact-source-bind-2026-10-15/README.md`. Existing-
vertex/periodic ownership, material-side removal and consistently oriented
closed-body graft assembly remain required. General curved end intersections can remain unproven rather than be
replaced with an unchecked approximation. Unequal-radius complex corners,
whole-result radius/thickness/G2/embedding/self-intersections, three-part STEP/
history acceptance, WASM/worker/UI, full P0 and remaining P2/P3 are still open.
