# Authored trimmed multipatch shells in Rust

`brep_core::trimmed_shell::assemble` constructs one connected closed shell from
explicitly owned 3D edges, vertices, surfaces and UV trim loops. Each patch
supplies its outer wire, optional holes and shell orientation. Pcurves follow
wire traversal; a coedge's reversed flag applies to the shared 3D edge.
Automatic seam matching, tolerance sewing and solid-volume authoring are
separate operations.

Admission requires every gate:

- Valid topology definitions, endpoint ownership and UV wire structure.
- Exactly two opposite uses of every nondegenerate edge, one connected face
  network and one closed connected link cycle at every vertex. Unused geometry
  and pinched vertex fans do not produce an admitted shell.
- Full-interval original edge/pcurve/surface agreement for every use. Matching
  endpoints or a handful of samples do not suffice.
- Independent UV trim-region validity, including outer/hole topology.
- At least C0 basis continuity across every internal surface knot and
  whole-natural-chart spanwise surface regularity. An inconclusive chart proof
  is unresolved, never a declaration of a singular surface.

The returned model exists only when every gate passes. It contains one closed
Shell and no Body. Failed or exhausted proofs return model=None and retain
closure, trim and regularity diagnostics with per-face/unvisited information.
The function does not repair a candidate implicitly.

Limits include 1..512 faces, up to 15 holes per face and the kernel's entity
and coedge ceilings. Degenerate pole edges and edges whose endpoint IDs are
the same are refused. Millimetre tolerance lies in [1e-10,1e-2]; UV tolerance
is positive. Agreement and regularity budgets are independently shared across
all faces, at most 1e6 each. Trim pair/cell budgets are at most 1e5 and trim
classification work is at most 1e6. Positive limits are mandatory.

Seam tolerance compares each lift with its common edge; two adjacent lifts
therefore lie within twice that tolerance. This is tolerance-qualified
geometric closure, not exact equality. Interior basis C1 continuity, global
embedding, self-intersection freedom, outward solid orientation, volumetric
validity, automatic branch inference, periodic seam ownership, editor and STEP
integration remain separate qualifications.

Independent tests reconstruct a cube and a rational hollow loft with annular
cap trims, check all gates and closed incidence, reject missing patches and
exhausted proof work, and demonstrate that a bulging pcurve with unchanged
endpoints cannot authorize a shell.
