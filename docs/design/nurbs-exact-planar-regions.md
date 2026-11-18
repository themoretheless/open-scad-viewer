# Exact affine filled disks and annuli

`nurbs_core::primitives::exact_planar_regions::radial(center, [axis_u,axis_v], [inner,outer], max_work)`
constructs the entire planar radial region `0<=inner<outer`:
`center + axis_u*r(v)*cos(u) + axis_v*r(v)*sin(u)`,
where `r(v)=(1-v)*inner+v*outer`, domain `[0,1]²` and the angular clock is the
proved rational circle clock. This is one family, not separately counted wrappers.
The older annular-sector API still requires a positive inner radius and remains
unchanged. The new API covers full affine elliptical disks and annuli with
exact binary64 storage or explicit refusal.

The degree2×1, 9×2 positive-weight tensor has a repeated longitude seam and
nonperiodic clamped storage. A disk has all V=0 controls exactly at its center;
the image remains intrinsically regular there, unlike a cone apex.
`Report.kind=Some(FilledDisk/Annulus)` distinguishes the collapsed center from
a real inner boundary. Refusals return `surface=None, kind=None`.

Every affine pole sum is computed from original binary64 leaves as an exact
rational and must round-trip through binary64. Nonrepresentable sums, overflow,
dependent axes and proof exhaustion never produce rounded or partial surfaces.
Rank is proved using a nonzero exact 2×2 frame minor instead of squaring a tiny
normal into a Gram determinant. This avoids unnecessary fourth powers and
admits independent subnormal frames within the existing rational bit guard.

A freshly proved circle is charged to the same ledger. The actual stored basis,
dimensions, positive weights and all homogeneous poles are bound to its tensor
with a linear radial profile under one global gauge. The exact plane equation
checks all three coordinates; the selected minor binds the two source-plane
coordinates. Projection alone could miss out-of-plane damage, and radial
distance alone could miss an angular phase change. Both are guarded and
independently tested on complete polynomial coefficients over all four cells.

Full circle traversal and strictly increasing nonnegative radius prove complete
injective disk/annulus coverage modulo the seam and, for a disk, its collapsed
center. A nonzero plane normal supplies regular intrinsic plane support.
No B-rep seam pairing, vertex fan, oriented shell or solid is inferred.
The tests compare both disk contours literally with the exact frustum's end
sections; that agreement alone does not create a closed oriented B-rep body.
With U angular and V outward radial, the parameter normal is opposite
`axis_u cross axis_v`; solid-cap orientation remains a topology obligation.

One shared `1..=1_000_000` ledger covers reference, construction and proof;
the existing 4096-bit rational guard remains. Tiny/near-dependent frames can
still make floating-point derivatives or normals ill-conditioned. No numeric
normal-error or evaluator-bit guarantee is supplied. Editing/reweighting the
freshly returned net requires new qualification; metadata is not a transferable
source-bound certificate.

Qualification: [receipt](../qualification/nurbs-exact-planar-regions-2026-11-03/receipt.json).
Native Rust only; JSON/WASM/Rush/editor/STEP and current-source distribution
remain separate qualification work.
