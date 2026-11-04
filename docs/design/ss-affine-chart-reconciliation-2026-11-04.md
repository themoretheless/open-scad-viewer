# Exact endpoint reconciliation across affine SS charts

`QualifiedSsAlgebraicClock::equals_affine` proves
`self_root = scale * other_root + shift`. A nonzero exact rational scale and
shift transform the first polynomial by exact coefficient Horner composition
and its selected isolator by the inverse affine map. Negative scales reverse
the bounds. The common-root gcd/Sturm comparison consumes the same fresh
operation/degree/coefficient policy as composition; no transformed midpoint or
rounded polynomial is used. Rational inputs must be canonical. Zero scale,
noncanonical input and exhausted work cannot authorize equality.

`OriginalSsRootPoint::same_source_point_in_charts` supports two regular charts
of the same ordered original surface pair and same original free axis. It
obtains their exact knot-span affine frames, proves equality of the clocks in
that shared original coordinate, then proves membership of the entire original
point enclosure in the target's unique root graph. Equal free coordinates alone
cannot equate two branches. Binary/algebraic and binary/binary combinations use
exact rational coordinate arithmetic. Clock and geometry have separate explicit
query policies. Different free axes, ordered source pairs, nonlinear blowdowns
and inferred periodic phase changes are not supplied by this proof.

Port endpoint reconciliation now consumes this chart-aware identity. The
existing wire reconciliation therefore returns ordinary literal shared-owner
closure across admitted chart changes. RootBrep remains the consumer of those
ordinary common vertices/edges, without a coordinate-tolerance welding rule.

Qualification includes native/original and deflated/local charts of the same
spatial line with nonunit knots, nonzero shifts and an irrational endpoint:
original clock = 8 * local clock - 5. It covers both directions, mixed
binary/algebraic and binary/binary selectors, wrong roots and exhausted clock
or original-geometry budgets. A real circle contour is reconciled across two
different graph domains and retains winding one; equal free clocks on upper
and lower branches are refused. Clock tests also cover negative scales, exact
inverse maps, invalid inputs and resource limits.

This establishes affine changes of the same free coordinate. Remaining work
includes arbitrary changes of free axis, cross-source physical junctions,
original third-face constraint generation, complete singular/periodic branch
arrangements and winding, finite material trim, general intersecting
Boolean/healing, lineage/history, actual bridge/WASM execution and the wider
quad/offset/fillet/G2 requirements.

Follow-up: different free axes can now reconcile when exact original-equation
clock consequence and selected-root membership prove identity; see
`ss-cross-axis-clock-consequence-2026-11-04.md`. This is a sufficient proof,
not arbitrary branch or finite-trim completeness.
