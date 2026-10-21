# Root-bound contact restriction

`contact_trim::between_faces` recomputes both original contact-curve/trimmed-face
intersection reports, requires complete coverage and strictly ordered disjoint
retained root bands, and authors one endpoint representative inside each band.
Each endpoint's distance to every point in its root-position enclosure is
bounded by the supplied positional tolerance. The actual restricted curve
uses exactly these two endpoint coordinates.

The restricted curve is checked directly against a proof carrier holding the
complete ORIGINAL curve coefficients and knots, not an extracted proxy. Both
restricted original-support and fillet pcurves must then pass full-interval lift
agreement with the returned curve. Root/trim classification, original-curve
agreement and returned contact agreement share at most 100,000 cells; exact
contact work is bounded separately by 1,000,000. No caller-supplied proof grants
mutation, and any failure exposes no partial result or changed input.

`face_crossings::inspect_curve` now supports unowned curve-definition queries
without manufacturing a model or body. The definition key is marked explicitly
with `source_is_owned:false`; it grants no existing edge or vertex ownership.
The existing indexed-edge query keeps its original identity and is marked true.
Both paths use the same root, original boundary, trim and membership checker.

## Current validation

Three contact_trim integration tests passed:

- Both contacts of a G2 plane/plane proposal are restricted between actual end
  faces at y=2 and y=8, ordered by the ORIGINAL curve traversal (which can run in
  decreasing world y). Endpoint bounds, direct original-curve agreement, both
  original-support/skin lifts and fresh whole-interval G2 inspection pass.
- An original rational-cylinder contact is restricted between y=2 and y=6 with
  the same endpoint/lift checks and unchanged G2 thresholds: position 1e-6,
  normal 1e-5 and world curvature tensor 1e-4.
- A root inside an actual end-face hole is proved Outside its material region
  and does not authorize an endpoint. Missing roots, reversed root order and
  insufficient work also refuse without changing either input model or curve.

The focused contact_trim, fillet_contact, face_replacement, face_cut,
face_window, face_crossings, edge_partition, pcurve_geometric_g2 and watertight
suites passed 42 tests. The native geometry-bridge check passed; WASM/runtime/UI
qualification has not been performed for this operation.

## Remaining objective

Returned endpoint coordinates have bounded root correspondence, not shared
vertex identities in an assembled body or exact mathematical root equality.
This restriction operation alone does not create end curves or a skin face.
The subsequent `contact_end` stage now qualifies both end boundaries and an
open four-edge skin face; see `../contact-end-boundaries-2026-10-15/README.md`.
Material removal and assembly with retained original face sectors remain open.
General periodic/boundary/tangent root ownership and incomplete root coverage
remain unresolved. Fresh result-wide radius, G2, thickness, embedding and
self-intersection admission is still required. Unequal-radius complex corners,
WASM/worker/UI, three-part/history acceptance, P0 and P2/P3 remain open.
