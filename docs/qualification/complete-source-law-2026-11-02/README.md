# Complete-patch source curvature law

Added `Patch::inspect_complete_source_curvature_law`: adaptive partition of the entire normalized [0,1] x [0,1] patch, including both source seams. All leaves are retained; no cell-limit or coefficient-budget exhaustion can become a successful qualification. A proven violation in any cell rejects the complete patch even when other regions remain unresolved.

Four source-law regressions pass, including exact area coverage, both boundary seams, zero-work refusal and invalid cell-limit refusal. On the constant-source rebuilt G2-profile fixture, 64 leaves give 19 proven violating regions, 36 unresolved regions and 9 within-tolerance regions. This contradicts whole-patch radius-law admission; the previously qualified central band remains a separate scoped result.

The source law uses inverse-mm tolerance 0.001, radial-position tolerance 0.001 mm and unit-normal chord tolerance 0.001. One shared 2,000,000 coefficient budget includes construction and inspection. The test takes about 40 seconds in this build configuration. No production shape or application admission is changed. Next geometry work must reconcile the transition profile with its full source-law requirements, rather than suppressing the seam regions or weakening their tolerances.
