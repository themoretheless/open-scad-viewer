# Exact algebraic clock binding at original SS nodes and poles

The common endpoint binder now accepts algebraic selectors at an original
blowup node or collapsed-pole endpoint when exact equality is proved.
`QualifiedSsAlgebraicClock::equals_rational` checks the query rational's
canonical form, the polynomial degree/bit/work policy, membership in the owned
isolating interval, and exact polynomial zero. Unique-root qualification then
identifies this rational with the selected root. A different root of the same
polynomial outside the isolator cannot authorize equality. The query reports
its counted arithmetic operations and never replaces the stored clock.

`OriginalSsRootPoint::equals_binary_parameter` converts a supplied finite
endpoint to its exact rational value. Algebraic selectors delegate to that
exact proof using their explicit clock policy. Original blowup source-node
binding and collapsed-pole binding consume this proof before using the existing
original-node/world-image authority. Ordinary affine graph membership and exact
periodic phase validation retain their existing rules. Failure of the equality
policy propagates as an error; it cannot fall back to coordinate proximity.

Qualification checks a repeated rational root selected by a nonpoint isolator
without any bisection, a crossing-node join between different blowup charts,
nearby unequal parameters, an equality budget exhausted after successful clock
construction, a repeated root at an actual cone pole with a different target
root domain, and a nearby nonpole point. Native exact clocks, old regular
ports, periodic paths, root BRep and mixed topology transfer are regression
coverage. These are exact endpoint-authority checks, not complete singular
branch enumeration or general periodic/pole trim qualification.

Remaining full-goal work includes generated third-face constraint polynomials
with original-source provenance, common-root identities between independently
owned algebraic clocks, cross-source face-local junctions/winding, complete
singular and periodic arrangements, finite trim with material selection,
intersecting Boolean and healing, lineage/history, bridge/WASM execution and
the wider quad/offset/fillet/G2 requirements.
