# Native mathematical gate repairs — 2026-10-03

The full sweep/miter goal remains active. These are local unpublished native repairs; packaged geometry WASM remains the previous qualified 47e6d7c9bd822449f4d845e59ba669a45dbcab1a48c9059285422a51815802e3 baseline.

Bézier extraction no longer advances past the final element when checking total control-row coverage. Affine partition of unity is checked along each operator row (the combination forming one extracted control), not each column. The tensor-surface test now supplies six v-controls as required by its ten knots and degree three. An explicit disconnected-basis regression preserves the existing separate-node requirement. Six tests passed, including random weighted curve reproduction, C0 knot joins, tensor-surface reproduction and element quadrature.

Interval Newton uses outward-rounded derivative coefficients and intersects Horner and centered mean-value derivative enclosures. A derivative separated from zero and strictly same-sign endpoint values proves absence by monotonicity. Centered function bounds consume the rigorous derivative enclosure too. Nine tests passed, including close roots, tangent roots, extrema, independent Brent comparison, coefficient rounding and decreasing-function absence.

Latest observed full primary native run: 729 passed, three failed, 35.27 seconds. All remaining failures are in normal_cone: spherical curvature enclosure too loose, cylindrical draft enclosure too loose, and small spherical offset unproved. This is not all-green evidence. Concurrent primary edits continue, so this log qualifies only the source observed by that run. Source copies and hashes for the two repaired modules are archived alongside targeted and full test logs.

The previous G1 slice independently passed 27 STEP fixtures and 44 wide/narrow UI scenarios with 480 assertions. Those packaged-runtime results do not qualify these newly edited native sources. Rebuild, source correspondence, broad global guarantees and publication/CI remain incomplete.
