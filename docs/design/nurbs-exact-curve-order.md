# Exact original-clock higher derivative curve fields

`editing::exact_derivative::curve_order(source,order,max_work)` constructs
vector fields of order1,2 or3 over every original nonempty span of a validated
2D or3D NURBS curve. This is one capability family; individual orders or spatial
components are not counted separately. The order ceiling matches existing
rational pointwise jets. Rational derivatives can remain nonzero above the
polynomial source degree.

The existing `curve` entry point and order1 retain the same first-derivative
arithmetic, output and work trace. Higher orders use the original homogeneous
numerators and denominator, physical span widths, and the exact recurrence
`d(N/W^k)=((dN)W-kN(dW))/W^(k+1)`. No rounded intermediate curves or finite
differences are used. Globally common univariate content is cancelled jointly
across all homogeneous channels, one output storage gauge covers each curve,
and actual stored bits are proved against the ORIGINAL uncancelled target.

Unclamped, periodic, nonuniform and repeated-knot storage is supported on its
original fundamental domain. Output pieces are clamped and nonperiodic, ordered
by original span, with absolute original clocks. One-sided derivatives at knots
or periodic seams are preserved independently, without global smoothness claims.
The original source is immutable. Invalid source/order/budget is an error; a late
work, degree, storage, denominator or bound refusal discards the entire result.

The existing1m-work/4096-bit, degree25, geometry, weight and conditioning limits
are unchanged. Exact storage is required; refusal does not establish that all
alternate representations are impossible. Native construction does not qualify
JSON/WASM/Rush/editor/STEP, topology, regularity or current-source application
artifacts. Independent verification uses the original Cox-de Boor basis and a
generic quotient-pair recurrence, including physical clocks and actual output.
