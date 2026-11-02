# Exact common-factor cancellation of a rational tensor surface

`editing::rational_surface_factor::cancel(surface, axis, max_work)` cancels a
polynomial factor shared by every homogeneous coordinate and weight function
on one selected Bezier axis. That axis must be nonperiodic, clamped and contain
exactly `degree + 1` poles. The untouched axis retains its entire original basis,
including multiple spans and periodic storage. Original absolute UV is retained.

The proposal lifts actual binary64 coordinates/weights into exact rationals,
converts Bernstein coefficients to powers, computes a common monic polynomial
GCD over ALL control rows, and checks both division remainders and multiplication
reconstruction. It tries primitive degree through original degree minus one,
allowing degree elevation when needed for positive representable storage. A
constant primitive uses the existing minimum valid degree1. One global storage
gauge covers the whole tensor net. Every Cartesian pole and weight must be
exactly representable and pass existing Surface validation.

Each selected-axis row is a rational control function, not necessarily an
actual surface isocurve. Final verification proves its complete literal curve
law with the existing independent B-spline predicate. Denominator polynomials
are freshly extracted from the actual source/output weights by a different
Cox-de Boor local-power recurrence. For every row j, it proves
`Ws_j * Wc_0 == Wc_j * Ws_0`. Source and output anchor denominators are strictly
positive on the complete selected domain by their positive Bernstein weights.
Thus all homogeneous rows have the SAME rational scale function. Summation in
the unchanged second-axis basis preserves the complete tensor law. Independent
factor cancellation of rows with different factors does not establish this and
is refused, even when every row alone is reducible.

One ledger covers proposal, exact storage and final proofs, with the existing
1,000,000-work maximum and4096-bit rational guard. Existing degree1..25,
positive-weight, conditioning and256-poles-per-axis limits are unchanged.
Refusal at any point returns the literal complete source surface; invalid
inputs/requests return errors. This is exact algebraic geometry, not bitwise
floating-evaluator identity, reuse of old source-bound topology certificates,
or a qualification of JSON/WASM/editor/STEP integration.
