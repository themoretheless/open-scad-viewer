# Next: source-qualified Jordan loops and complete branch ownership

Updated 2026-11-13: removed 5 completed items (two qualified original faces
with whole-chart injectivity, injective affine root-graph carriers, native
graph-window membership, strict opposite ordering at shared endpoints, and the
topological circle/unit-winding argument with its regressions). Evidence:
`qualify_paired_interior_loop` in
`crates/brep-core/src/model/mixed_root_model/closed_cut/jordan.rs` and
`ss-interior-jordan-progress-2026-11-04.md` with its native qualification receipt.

Regular affine interior joint and paired-face Jordan admission is implemented,
plus certified cross-carrier joints on interior knot lines (free-axis case).
Non-free-axis knot unions, seams, all physical branch ownership, singular/coincident strata
and mixed-solid admission below remain open.

## Open admission step

Updated 2026-11-15: the first knot step is implemented. A loop branch that
crosses an interior knot is carried by two span carriers; the joint between
them is certified by `OriginalSsRootEdge::qualify_knot_successor`
(`crates/nurbs-core/src/intersection/ss_intersection/root_curve/knot_joint.rs`)
and admitted by `qualify_loop_neighborhoods` in `jordan.rs`. The certificate
requires two affine deflated carriers over the same original free coordinate,
spans equal except across one interior continuous knot (multiplicity at most
the degree), a shared selector whose exact free clock is the knot on both
carriers, two qualified one-sided graph windows `[a, K] x C` and `[K, b] x C`
whose union contains the open box `(a, b) x int C`, and the joint plus shortened
incident subarcs strictly inside `C`. Otherwise it returns
`NURBS_SS_KNOT_JOINT_UNPROVED`. Regression: `tests/ss_knot_joint.rs`
(nurbs-core) and `loop_crossing_an_interior_knot_has_certified_jordan_neighbourhoods`
(brep-core `mixed_cut_regions`).

Still open in this step:

1. End-to-end paired admission with material for a knotted support. The
   Jordan neighbourhood half accepts the knotted loop, but the knotted curved
   body is not a qualified closed solid: the shared-boundary certificates in
   `crates/brep-core/src/contact/shared_boundary/` accept Bezier (single-span)
   faces only, so face pairs meeting the knotted face stay unclassified. The
   brep-core regression keeps a tripwire on that refusal.
2. Knots crossed by a coordinate other than the common free axis, joints
   whose selector is not exactly on the knot, knots of multiplicity above the
   degree, and periodic seams: all still refuse (no seam proof is claimed;
   the same one-sided argument would additionally need native seam identity
   and phase-shifted windows). Poles stay refused through the excluded pole
   stratum.

## After that

Complete-census ownership is implemented for one closed component
(`ss-interior-census-progress-2026-11-04.md`), and the two-component
arrangement with nested holes is qualified (`ss-census-arrangement-progress-2026-11-04.md`,
`ss-nested-hole-progress-2026-11-04.md`). Still open: positive qualification of
multiple distinct components tied to their loops via actual shared root
selectors and graph membership (do not connect components merely because
interval boxes overlap); with all components and original rims, the full finite
arrangement, region-wide connected-region qualification and material choice,
and sewing of exact mixed boundaries. Complete cross-source contact ownership
and mixed-body admission plus independent volume still precede a general
Boolean completion claim. Coincident 2D regions, arbitrary singular strata,
seams and poles remain in the full objective throughout this sufficient
regular-loop step.
