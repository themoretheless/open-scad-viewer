# Exact internal tensor knot insertion

`editing::exact_surface_knot_insertion::insert` refines U or V of a validated
rational tensor surface at a literal finite parameter strictly inside its
original domain. The selected axis must be nonperiodic; the unedited axis can be
periodic. The same degree, internal multiplicity, 256-poles-per-axis, positive
weight and numeric admission limits apply. Boundary clamping is not this edit.

All selected-axis rows are lifted from the actual stored bits into exact
rational homogeneous coordinates and refined without intermediate rounding.
Only then is a **single global gauge** chosen for the entire tensor net. Every
stored Cartesian coordinate and weight must round-trip exactly, or the operation
returns the original surface and zero inserted knots.

Final verification uses the actual proposed output, not its intended rational
construction. Each row's complete rational parameter law is verified by the
existing original B-spline identity predicate. Independently, local-power
Cox-de Boor polynomials compare its complete scalar denominator against the
source on every union knot cell. All rows use the same positive scale, anchored
by the source/output first corner weight (internal insertion preserves that
homogeneous corner before global storage). Rational row identity plus common
proportional denominators implies proportional numerators with that same scale;
linear combination in the untouched basis therefore proves the full tensor law.
Constant rows cannot hide independent row-gauge changes.

Proposal, storage, row geometry and denominator proofs consume a **single**
ledger of at most 1,000,000 work units. Exact rational proposal/denominator
arithmetic retains the 4096-bit guard. Mathematical refusal at any point,
including a later row or final proof, returns a literal source clone. Invalid
input/request errors are distinct. No evaluator-bit identity, downstream
certificate reuse, topology, WASM, Rush, editor or exchange claim is made.

Qualification distinguishes operation-specific checks from the whole library
harness. The default library harness compiles and its two denominator tests
pass. The no-codec whole-library harness currently fails with 79 pre-existing
codec/transport-dependent test errors. That failure is archived, not suppressed.
The no-codec denominator target compiles the exact same production denominator
and bounded-arithmetic source files in a focused integration harness; the actual
library dependency graph remains codec-free. Full library/workspace checks are
therefore still an open separate requirement.
