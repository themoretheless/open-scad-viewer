# Contact-end roots bound to original body-edge vertices

`contact_bind::between_faces` recomputes both contact end-root reports on the
immutable original contact and actual cutter faces, then treats selected body
edge indices/natural parameters as proposals. A proposed original-edge point
must be bounded to the complete contact-root position enclosure within the
source model tolerance. No proximity merge or supplied proof grants admission.

Both actual incident face-local uses of each selected source edge are split by
`edge_partition::partition`. The actual authored vertex is checked again against
its original contact-root enclosure. Both contacts receive exactly those source
vertex coordinates. Each final body-edge child is checked directly against the
immutable original body-edge definition; the returned contact is checked against
its immutable original curve and both actual original-support/skin lifts after
endpoint changes. No pre-edit certificate survives an endpoint edit unchecked.

Same-edge ends are processed in descending ORIGINAL natural parameter order.
All three final children are checked against the original curve, rather than
allowing successive rounded split errors to accumulate. The entire operation
shares 100,000 cells and 1,000,000 exact work. Both ends are one transaction:
failed root correspondence, split, parent agreement or returned lift exposes
no changed source model. Returned metadata includes original source edge IDs,
original natural parameters and actual shared source vertex indices.

## Current evidence

Two integration tests pass:

- Both contacts of a prepared G2 plane/plane transition bind to four vertices on
  four edges of a real cuboid body (bounds [0,2,0] to [10,8,10]). Each bound vertex
  is used by both incident source faces. Source topology remains closed, exact-
  aware continuous closure passes, and whole-interval G2 is freshly rechecked
  on each changed contact. An open transition patch built afterward uses
  byte-identical corner coordinates to these four source vertices.
- Two original contact end roots on one original body edge produce two shared
  vertices and three continuously qualified children with complete naming.
  Duplicate end proposals refuse. Wrong spatial proposals and insufficient
  work also refuse without changing the input body.

The focused contact_bind, contact_end, contact_trim, fillet_contact,
face_replacement, face_cut, face_window, face_crossings, edge_partition,
pcurve_geometric_g2 and watertight suites passed 47 tests. The binding tests were
rerun after adding returned original-parameter metadata and both passed. The
native geometry-bridge check passed; WASM/runtime/UI qualification is separate.

## Remaining requested work

This subdivides the original body's edges and binds contact coordinates. It
DOES NOT insert the transition patch, remove the original sharp-edge sectors,
or produce a new closed fillet body. The subsequent `contact_graft` stage now reuses these bound source vertex
indices and transition edges to build a closed control candidate; see
`../contact-graft-2026-10-16/README.md`. General planning and global admission
remain outstanding.
Source boundary changes are continuously bounded within model tolerance, not
claimed as exact mathematical root/curve identities. Periodic source edges and
endpoints at existing source vertices remain refused by this operation.

General retained-region/material selection, unequal-radius corner/end blends,
fresh whole-result radius/thickness/embedding/self-intersection admission,
three-part STEP/history acceptance, WASM/worker/UI, P0 and remaining P2/P3 stay
open. The source body's closure check is not a global fillet-volume certificate.
