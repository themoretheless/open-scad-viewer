# Source-bound material covers and a mixed root cavity

`QualifiedRootSurfaceMaterialBox` binds an immutable original rational NURBS
surface, its original basis/weights and full closed UV restriction to a sealed
root-solid material enclosure. Every touched original knot span is included;
invalid/escaping UV ranges, exhausted span work and uncertified periodic seams
cannot create the owner. An image supplied by the caller is never authority.
The owner can bind an existing Cartesian certificate or automatically compute
the full original interval image and qualify it against the root solid.

`QualifiedPairedRootSolid::qualify_cavity_in` subtracts one admitted root solid
from a single original admitted host body, which may already have cavity shells.
Before topology assembly it exhausts BOTH directed original parameter forests:

1. Every retained inner boundary point must lie inside original host material.
2. Every retained host boundary point must lie outside the finite inner region.

Each accepted cell owns a whole-original-image material certificate. Failed
image candidates subdivide; complete source-domain exclusion can discard only
whole cells outside the actual retained source face. Nonempty original spans
are all visited. Root source roots cover the entire qualified outer Jordan
disk box, including its boundary. Host source roots cover all natural knot
spans; holes and other trims require native whole-cell domain exclusion.
No witness by itself establishes separation or containment. Both complete
covers, connected finite root material and its original embedded boundary
establish actual containment, excluding an enclosed host cavity or boundary.
A contradictory material proof on a retained source cell rejects the candidate.
Unfinished work, contact or unrepresentable refinement cannot grant an owner.
A breadth-first queue prevents a narrow trim-boundary chain from starving
retained interior cells that already prove a contradictory material signature.

Coverage cells and source-domain queries are shared separately per direction.
Each target material attempt has its explicit per-query allowance; source root
band/winding work is shared through the forward forest. Thus total work is
bounded by the coverage allowance times the declared target-query policy.
Independent original host flux and the root solid's independently admitted
volume are subtracted with outward interval arithmetic. A positive interval
within the requested result error is required (default result error 1e-3).
This uses actual source geometry, not a scalar supplied by a caller.

Assembly retains original host topology and appends original root vertices,
world curves, UV curves and faces. Vertex/edge/loop/face/shell indices and the
root-pcurve world-edge binding are remapped together. ALL appended shell face
uses reverse, while physical source curves and selectors remain unchanged.
The root shell becomes an explicit inner shell of the host body. The resulting
mixed topology passes the original binding/topology validator before the new
immutable material owner is returned. Its point query evaluates the proved
set difference; work policy applies separately to the two operands. Replay
freshly verifies both sources, both full covers and the independent volume.

Qualification exercises inner genus-zero and annular genus-one quartic cavities
with independent reference result volumes 27 minus the analytic root volume,
original U reflection, explicit inner-shell orientation, original root/selector
preservation, material/void/boundary point queries, source identity and span
budgets, insufficient cover work, insufficient volume accuracy, and a disjoint
host refusal. The first topology trial correctly failed because its appended
root-pcurve world-edge index was not remapped; both edge references now shift.

This is a native single-original-host/single-paired-root-inner cavity owner,
not a general Boolean or general shell forest. Multiple mixed hosts/inners,
arbitrary nesting forests, touching shells and healing, complete all-face-pair
SS and finite trim, singular/coincident strata, periodic/pole material,
intersection/union/XOR for intersecting bodies, full lineage and actual
bridge/WASM delivery remain in the original goal. Broader feature parity
(offset/replace/hollow/thicken, general fillet/chamfer, quad topology, G2 solver
and construction history) is not established by this qualification slice.

Frozen qualification: 20 native tests passed; architecture remains at 54 pinned
oversized files and zero layer violations. Both directions accept 2 inner and
6 host patches for these cases. Nonreflected cavity volumes are enclosed by
[26.996715021744155, 26.99721477861792] (inner lens) and
[26.99234129436827, 26.992840837788645] (annular lens). The independent analytic
references lie inside these intervals. The complete native/ray logs and source
hashes are frozen in `docs/qualification/ss-root-cavity-2026-11-04/`.
