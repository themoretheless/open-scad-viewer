# Exact internal knot insertion

`nurbs_core::editing::exact_knot_insertion::insert` refines a validated nonperiodic
B-spline without publishing rounded geometry. Unlike ordinary floating point knot
insertion, it can refuse an otherwise valid operation when the new Cartesian
poles or positive weights cannot be stored exactly as binary64.

The parameter is a literal finite binary64 value strictly inside the original
absolute domain. A batch inserts between one and `degree` copies, keeping the
existing internal multiplicity limit `degree`, degree limit, and 256-pole cap.
Periodic seams and boundary clamping are outside this operation's contract.

The proposal lifts the original stored poles and weights to exact rational
homogeneous coordinates, applies Boehm insertion with exact rational arithmetic,
and retains the rational net between batch steps. Shared storage conversion uses
one global positive weight gauge, never a separate gauge for each pole. Every
stored coordinate and weight must round-trip to its original exact rational.
An independent existing B-spline definition predicate then compares the source
and actual output across their original parameter domain. Only `Equal` publishes
the candidate. Every mathematical refusal returns a literal source clone and
zero inserted knots; invalid requests return an input error.

Proposal and final predicate share at most 1,000,000 work units. Proposal numbers
retain the existing 4096-bit guard; no numerical, topology, runtime, memory or
qualification limits are raised. This proves the rational curve law, not bitwise
identity of floating evaluators or transferability of downstream source-bound
certificates. WASM, Rush, editor and exchange qualification remain separate.
