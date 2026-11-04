# Independent SS clock reconciliation

`QualifiedSsAlgebraicClock::equals_clock` proves whether two immutable clocks
select the same exact real number. It validates a fresh degree/bit/operation
policy and checks the intersection of their original isolators. Disjoint
isolators return false. A point intersection requires exact zero of both
polynomials. For a nonpoint intersection it computes the exact rational
Euclidean gcd, rejects a constant gcd as unequal, and counts distinct gcd roots
in the intersection with a fresh exact Sturm sequence. Exactly one common root
proves equality because each original clock already owns exactly one distinct
root in its isolator. Equal polynomials or overlapping boxes alone are not proof.
The query returns the equality result and its counted rational work.

`OriginalSsRootPoint::same_source_point` combines that scalar proof with the
same qualified original graph definition; mixed binary/algebraic equality uses
the exact binary rational value. Literal `same_selector` retains its old owner
semantics. Different charts are not equated by scalar clock coincidence.

`QualifiedOriginalSsPort::rebind_equivalent` first proves source-point identity,
then binds the chosen canonical owner to its original target graph and phase
with the existing membership policy. `OriginalSsRootWire::reconcile_selectors`
performs this for adjacent endpoints of a bounded (at most 256 edge) contour and
rebuilds each affected original edge. Returned contours satisfy ordinary literal
shared-owner closure, so existing RootBrep vertex collection and replay need no
approximate or special equality rule. Each independent join has the supplied
explicit clock policy; returned ports retain their original geometry policies.
No welding is authorized by rounded positions, tolerance or interval overlap.

Qualification includes different polynomials sharing an irrational root,
repeated roots, constant gcds, equal polynomials selecting different roots in
overlapping isolators, point/nonpoint isolators in both directions, exhausted
work and distinct rational clocks rounding to the same binary64 value. The
paired BRep test splits an original SS circle edge with independently authored
endpoint polynomials, rejects ordinary literal closure and a deficient equality
budget, reconciles the exact selectors, then constructs/replays the nine-edge
BRep and both original UV projections.

This is same-graph point reconciliation, not arbitrary physical branch assembly.
Remaining work includes source-derived third-face constraint polynomials,
cross-source face-local junctions, chart transitions with different free axes,
complete singular/periodic arrangements and winding, finite material trim,
general intersecting Boolean/healing, lineage/history, actual bridge/WASM calls
and the wider quad/offset/fillet/G2 requirements.
