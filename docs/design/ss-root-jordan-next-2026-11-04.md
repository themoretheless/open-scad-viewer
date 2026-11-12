# Next: source-qualified Jordan loops and complete branch ownership

Updated 2026-11-13: removed 5 completed items (two qualified original faces
with whole-chart injectivity, injective affine root-graph carriers, native
graph-window membership, strict opposite ordering at shared endpoints, and the
topological circle/unit-winding argument with its regressions). Evidence:
`qualify_paired_interior_loop` in
`crates/brep-core/src/model/mixed_root_model/closed_cut/jordan.rs` and
`ss-interior-jordan-progress-2026-11-04.md` with its native qualification receipt.

Regular affine interior joint and paired-face Jordan admission is implemented.
General knot unions, all physical branch ownership, singular/coincident strata
and mixed-solid admission below remain open.

## Open admission step

1. If two one-sided knot charts lack one common neighborhood, prove the UNION
   of their source-equivalent boxes covers an open original-root neighborhood;
   otherwise retain an unresolved joint. Do not extrapolate a knot polynomial.
   Today the admission refuses any edge touching a knot, seam or pole with
   "Loop needs explicit one-sided knot/seam/pole neighborhood admission"
   (`jordan.rs`).

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
