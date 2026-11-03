# Exact original-clock surface partial fields

`nurbs_core::editing::exact_surface_derivative::surface(source, Axis::U|V,
max_work)` constructs the vector field dS/du or dS/dv on every original nonempty
fundamental UV knot cell. Arbitrary validated tensor knots/degrees, nonuniform
or repeated knots, unclamped axes and periodic encodings are admitted as input.
Output is a complete U-major/V-minor sequence of clamped nonperiodic surfaces,
retaining each original absolute UV interval. Cell edges and corners own their
one-sided traces; adjoining values and periodic seam smoothness are not assumed.

Exact Cox-de Boor polynomial extraction operates on the original stored bits.
For each coordinate the derivative numerator is A_u W-A W_u (or its V analogue),
and the denominator W². Local coordinates are scaled by the exact original
span width. Sparse bivariate arithmetic and one ledger cover every cell.
Proved constant vector fields, including zero, reduce before output degree
checks. General bivariate factor cancellation and mathematically minimal degree
are separate work; chosen representations exceeding degree25 are refused.
There is no exhaustive search of alternate degree/basis representations; a
refusal does not prove that every other exact representation is impossible.

A single global rational storage gauge spans every pole of an output UV net.
Independent row/column gauges would change a general tensor field. The actual
binary64 stored coordinates and weights are re-lifted and compared to the
original uncancelled derivative through full bivariate polynomial identities.
Exact positive storage and existing geometry limits are mandatory; any late
cell refusal discards the whole result. The existing 4096-bit and1,000,000-work
guards, degree25 and source net bounds remain unchanged.

This is native vector-field geometry construction, owned by the existing
`editing` domain. Pointwise differential APIs remain supported. No regularity,
normal, trimming, B-rep/solid, numerical evaluator-bit or old source-certificate
transfer is issued. JSON/WASM/Rush/editor/exchange integration is separate.
