# Retained face and sewn replacement surface

## Implemented operation

`face_window::replace` performs an atomic geometry change on one interior closed
window of a face. The original surface is retained unchanged outside a new
clockwise hole. A supplied replacement surface joins the same oriented shell,
using the same authored 3D curves and endpoint vertices on both sides.

Each boundary supplies a shared curve and independent source/replacement
pcurves. The actual saved reversed source definitions are checked; a rounded
reverse is not silently treated as exact. Full coefficient identities are tried
first under one fixed 1000000-work ledger. Other boundaries require continuous
interval agreement under the separate shared cell budget. Both contour regions
must pass native trim validity with a shared 10000-pair ceiling. No existing
proof limit was increased to admit the higher-degree fixture.

Untouched edge and face identities persist. The modified source face and new
replacement have explicit parent-to-children Split lineage. The operation returns
a candidate, not a solid certificate: embedding, orientation, intersections,
G2 and thickness remain separate requirements.

## Exact-aware seam closure

The old interval-only closure checker could not qualify the degree-six boundary
composition within its supported correlated-bound representation. The stored
curves instead have exact original coefficient identities. The new
`boundary_agreement::verify_with_exact` and `watertight::inspect_with_exact` APIs
make both ledgers explicit and use each exact Equal result only for its own use.
Unsupported, different, indeterminate and unprocessed uses must still receive
whole-interval evidence. A partial exact report or closed shell flag cannot
qualify remaining uses. Closure does not establish trim validity or embedding.

## Current native evidence

Fourteen focused integration tests passed: three window tests, five trimmed-face
crossing regressions and six previous watertight tests. Window cases cover exact
shared boundaries, preserved original geometry/identities, Split lineage,
malformed neighboring curves without panics, bad lifts, overlapping windows,
resource exhaustion, and a deliberately broken target with closed shell flags.

`face_window_acceptance` also runs a fresh full native volume audit. The positive
control has seven faces, sixteen edges and twelve vertices. Exact boundary
lifts/joins, valid trims, injective faces, all face pairs and global volume
admission passed. The negative control retains exact boundary/trim evidence but
its deeper surface crosses the opposite wall; global admission correctly fails.
False admission is not inferred from closed ownership or smooth-looking seams.

## Independent changed-geometry evidence

The fixture replaces a 4×4 mm window of an 8×8×8 mm cuboid with a nonplanar
Bernstein surface. A center control displacement of -1.024 mm yields a measured
center depth of 0.1 mm. Unlike edge subdivision, this changes the actual volume.

The independent design integral is
`512 - 16 × 1.024 / 49 = 511.6656326530612 mm³`.
OpenCascade STEP import independently returned a valid single solid, sixteen
unique edges, twelve unique vertices, unchanged outer 8×8×8 mm bounds and volume
511.6656326530611 mm³. Volume discrepancy was 1.1368683772161603e-13 mm³ and the
reported bounds discrepancy approximately 1e-7 mm. The expected volume comes
from the design polynomial, not a native measured mass or exported tessellation.

Generated local files are in `output/qualification/face-window-2026-10-15/`.
Reproduce with the native `face_window_acceptance` example, followed by
`scripts/verify-cad-step-occt.py <directory> --no-preview` in the qualified OCP
environment.

## Remaining objective

This authors a closed interior window and a supplied surface. It does not yet
classify general open retained face sectors, generate general fillet contact/end
surfaces or unequal-radius converging corners, handle periodic/pole chart
ownership, or numerically qualify general G2/radius/thickness along all fillet
seams. Whole-interval inspection now confirms G2 on all four original trimmed
pcurves of this fixture. Separate negative controls distinguish a normal jump
from a curvature jump despite exact shared positions, and reject a corrupted
owned 3D curve. The surface-region checker works on original knot spans;
periodic charts remain unresolved. This does not certify arbitrary fillet radii,
thickness, corner construction or global embedding from G2 alone. WASM/worker/UI, the named part and
history acceptance, P0 and the remaining P2–P3 requirements stay open.
