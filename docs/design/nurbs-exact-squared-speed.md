# Exact original-clock squared-speed curve fields

`editing::exact_derivative::squared_speed(source,max_work)` constructs the
scalar field `C'(u) dot C'(u)` on every original nonempty fundamental span of
a validated2D/3D NURBS curve. It uses a curve as a scalar-field carrier:
`[squared_speed,0]` or `[squared_speed,0,0]`; these are not spatial positions.
This is one capability family, not one entry per spatial component or span.

The native construction extracts the original homogeneous source polynomials,
forms physical-clock first-derivative numerators, sums their squares and uses
W^4 as denominator. It stores no rounded intermediate derivative curve. Joint
univariate content reduction and a single whole-piece storage gauge precede a
fresh proof against the ORIGINAL uncancelled squared-speed target, using actual
stored binary64 bits. Existing first/higher derivative behavior is preserved.

Nonuniform, repeated, unclamped and periodic encodings retain absolute original
clocks and complete span cover. Output pieces are clamped/nonperiodic and own
one-sided traces independently. Reversing the clock preserves squared speed at
corresponding parameters; stretching it by a factor k scales squared speed by
1/k^2. Orthogonal spatial transforms preserve the scalar field.

Zero speed, including isolated interior stationary points, is valid. A nonnegative
field may have negative Bernstein poles; no pointwise control-pole test is used
to assert strict positivity or regularity. This API does not construct speed's
square root or arc-length parameterization, and does not issue regularity,
topology or source-certificate authority.

The shared1m-work/4096-bit, output degree25, geometry, weight and conditioning
limits are unchanged. Invalid source/budget is an error. A late work, degree,
bound or exact-storage refusal discards every earlier piece. Refusal does not
prove that all alternate representations are impossible. JSON/WASM/Rush/editor/
STEP and current-source application delivery remain separate qualifications.
