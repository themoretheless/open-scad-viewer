# Continuously bounded differential-field storage

`editing::bounded_differential::curve(source,field,max_error,max_work)` constructs
an original-clock field on every original nonempty fundamental curve span.
`Field` selects derivative order1..3, squared speed, spatial unnormalized binormal
or squared curvature. Scalar values occupy the first coordinate; remaining
coordinates are zero. The error is Euclidean field-value error, with units of
the selected field. Rounded fields carry no separate sign/positivity or
exact-invariant certificate; near a zero, only the error enclosure is promised. It is not spatial-path displacement or an evaluator's
floating-point rounding estimate.

The exact APIs retain their exact-storage-or-refusal contract. The bounded API
first tries the same exact storage. If its poles/weights cannot be represented
exactly, it proposes rounded Cartesian poles and one globally scaled positive
weight row. No proposal is authority: actual stored binary64 values are lifted
back to exact rationals before the complete error proof.

The source target is built from original homogeneous coefficients. Joint exact
content reduction is checked by fresh cross-products against the uncancelled
target. Source weights prove original derivative denominators nonzero; squared
curvature additionally requires original squared velocity's strictly positive
Bernstein coefficients BEFORE cancellation. Approximation cannot fill a
stationary point or borrow an old source/regularity certificate.

For a reduced target `N_i/D` and stored output `n_i/d`, the residual polynomial
is `R_i=N_i*d-n_i*D`. Positive Bernstein denominator coefficients give a closed
lower bound `L=min(B(D))*min(B(d))`. With `M_i=max(abs(B(R_i)))`, the continuous
Euclidean error is bounded by `sqrt(sum((M_i/L)^2))` over the entire closed span.
All arithmetic producing the squared bound is exact and resource bounded.
Acceptance compares it to the exact square of the caller's binary64 tolerance.
A bounded search through ordered positive-float bits returns an outward upper
float using exact squared comparisons, avoiding underflow even for subnormals.

Outputs retain absolute original clocks and independent one-sided knot traces;
no global differentiability, seam matching or regularity across source knots is
inferred. No sampled acceptance, adaptive fitting or degree reduction occurs.
The source target, reduced denominator and proposed field may be conservatively
refused even when some alternate construction would meet the requested error.

Output degree25, shared1m-work/4096-bit, geometry, weight and conditioning limits
are unchanged. Residual hulls have degree at most50 (two admitted degree25
polynomials); no output-degree limit is widened. Original curvature regularity
conversion retains its degree25 restriction. Invalid inputs/field/order/tolerance/
budget are errors. Any work, degree, denominator, output or tolerance refusal
discards every earlier piece AND every earlier bound. Exact APIs and their
work accounting remain unchanged.

Native qualification passed; see `docs/qualification/nurbs-bounded-differential-2026-11-03/receipt.json`. JSON/WASM/Rush/editor/STEP, topology and
whole-application delivery are separate; there is no serialized proof authority.
