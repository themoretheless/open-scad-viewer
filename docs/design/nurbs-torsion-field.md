# Original-source torsion fields

`editing::exact_derivative::torsion(source,max_work)` constructs the scalar
carrier `[tau,0,0]` on every original nonempty span of a validated spatial curve.
`editing::bounded_differential::torsion(source,max_error,max_work)` uses the same
original target with continuous Euclidean field-value error admission. The public
bounded `Field` enum is unchanged. Exact storage and bounded storage remain
separate contracts; there is no silent precision downgrade.

For original homogeneous velocity V/W², acceleration A/W³ and jerk J/W⁴,
`tau=W*((V cross A) dot J)/norm(V cross A)²`. Construction uses original jets,
not rounded intermediate derivative curves. BEFORE cancellation, the original
squared-binormal numerator must have strictly positive full-span Bernstein
coefficients, with proof degree at most25. This sufficient check excludes zero
curvature and stationary sources; unresolved coefficients also refuse and do
not prove geometric singularity. Planar curved sources can have exact zero
torsion, while straight curves cannot borrow a zero numerator to fill 0/0.

Torsion is invariant under affine clock reversal/stretch and proper rigid
motions. A spatial reflection reverses its sign; uniform positive world scale k
scales its value by 1/k. Absolute original clocks and independent one-sided knot
traces are retained. No global knot smoothness, unit Frenet frame or topology
certificate is issued.

Output degree25, geometry/weight/conditioning limits and shared1m-work/4096-bit
limits are unchanged. A late guard, storage, tolerance or budget refusal discards
all earlier pieces and bounds. Exact refusal does not rule out alternate exact
representations. Bounded storage proves actual stored-bit error over the whole
span; it is not a sampled estimate or evaluator-arithmetic certificate.

Native qualification is in progress. JSON/WASM/Rush/editor/STEP and complete
application delivery remain separate.
