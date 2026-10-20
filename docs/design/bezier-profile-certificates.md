# Checkable Rust certificates for retained Bézier profiles

The topology certificate certifies **topology of the exact represented planar profile**.
An additional measure certificate proves an exact enclosing interval for its area.
Neither certifies a prior construction/Boolean operation, tessellation, or
three-dimensional solid validity. The editor's numerical area is a separate value.
The existing `geometryStatus: numerical_uncertified` therefore remains valid.

## Interface

- Rust: `brep_core::profile_certificate::{certify, verify}`.
- Geometry ABI: `brep_profile_certify` and `brep_profile_verify_certificate`.
- TypeScript transport: `certifyBrepProfile`, `verifyBrepProfileCertificate`.
- Standalone checker: `crates/brep-core/examples/verify_profile_certificate.rs`.
  It reads one JSON request on stdin: `loops`, `toleranceMm`, `certificate`.
  The optional argument `certify` generates a certificate; the default
  `verify` checks a supplied certificate. Failure exits with status 1.

The checker returns an opaque Rust `VerifiedProfile` only after checking every
claim. The wire response has scope `represented-profile-topology`. A caller
must verify again against the current source after editing or loading it.
Persisting a status string is not a verification step.

## Source and arithmetic

Every admitted finite binary64 coordinate, weight and knot is converted to its exact
integer ratio, including subnormal coordinates. The existing curve validator's
positive-weight and conditioning limits still apply. Structural layout, degree,
periodicity and tolerance bits are bound in a canonical source string.
Hexadecimal bit strings preserve signed zero and avoid JavaScript integer
rounding. No hash collision assumption is needed.

The certificate concerns the supplied stored definition, which may itself be a
rounded result of an earlier operation. It does not turn that snapshot into an
authenticated original authored input and does not assert equivalence to its
construction history.

The checker uses `num-bigint`/`num-rational`, not the numerical profile
validator, tessellator, generic intersection driver, or the expansion-based
production root finder. It shares exact-arithmetic helper routines with witness
generation, but does not trust that search, its intermediate decisions or
caller flags. Schema, version, source binding, every witness, witness census,
orientation and nesting are checked anew.

## Checked claims and soundness

1. **Representation and closure.** Positive-weight planar
   curves of degree 1..8 are admitted, including periodic and multi-span storage.
   Every active knot interval is restricted in exact homogeneous arithmetic.
   Rational de Boor values at degree+1 distinct parameters determine its exact
   Bernstein polynomial; no rounded poles are emitted. Every endpoint join is an exact rational
   equality. Empty material has a complete empty certificate.
2. **Exact subdivision.** Homogeneous de Casteljau at 1/2 partitions each
   original span without generating rounded Euclidean poles or knot values.
3. **Span injectivity.** Each piece supplies a coordinate. For its rational
   derivative numerator `X'W-XW'`, exact polynomial GCD and Yun square-free
   decomposition identify odd root multiplicities; a rational Sturm chain
   excludes odd interior roots. The first nonzero coefficient and endpoint
   delta establish the sign. The nonzero numerator has only finitely many
   stationary zeros, so the coordinate is strictly monotone. Even zeros and
   stationary endpoints are allowed.
4. **All-pair separation.** There is exactly one witness for every unordered
   pair of pieces. An exact supporting projection separates positive-weight
   control hulls. For adjacent pieces, a weak separator is allowed only when
   open spans strictly leave the support plane or the other span has only the
   shared endpoint on it. Flat/flat and extra endpoint contacts are rejected.
5. **Algebraic cusp joints.** Sentinel 65535 requests an independent rational
   implicit-polynomial check for same-wire pairs whose hulls overlap. Exact
   common homogeneous factors are cancelled; the denominator of the original
   positive-weight curve has no finite zero. A rational moving-line nullspace
   and the resulting implicit identity are checked exactly. Sturm excludes
   every open-domain eliminant root, including repeated roots. Every endpoint
   eliminant zero must be an allowed exact endpoint equality. Extra eliminant
   roots cause refusal, not dismissal. This is a sufficient, conservative
   proof; it does not claim that every eliminant root is a finite intersection.
6. **Orientation.** If every pair has a hull witness, replacing each piece by
   its chord is a simultaneous injective homotopy: its monotone coordinate
   stays monotone and its image stays in its hull. Exact polygon area therefore
   gives the original orientation. Algebraic witnesses require a separate
   exact Green-integral sign proof. Polynomial contributions are integrated
   as rational polynomials. Rational contributions use complete homogeneous
   Bernstein range bounds, never samples; integer outward rounding to a
   256-bit dyadic grid bounds the summed interval. Rational integration uses an
   exact polynomial expansion of the reciprocal squared weight and a complete
   geometric-series tail bound over the whole interval.
7. **Nesting.** Different-wire hulls must be strictly separated. Thus another
   wire's query vertex stays outside every span during chord replacement,
   preserving winding even when a same-wire pair uses an algebraic witness.
   Exact half-open polygon winding determines depth; material-left orientation
   is checked against parity.

Strictly monotone pieces and the complete all-pair census imply simple closed
wires. The verified orientation and nesting define a regular planar material
region. These implications assume correctness of the Rust checker, integer and
rational arithmetic libraries, compiler and runtime. This is a checkable Rust
certificate, not a proof assistant extraction or a third-party certification.

## Refusal and bounded work

The checker accepts at most 32 wires, 128 source curves, 128 active knot spans and 256 exact pieces,
with at most eight geometric subdivision levels. Rational orientation permits
at most 8192 cells and 12 levels. Source binding is bounded to 65536 bytes.
Polynomial division refuses coefficient
numerators/denominators beyond its bit budget. Unknown fields, unknown versions,
missing/extra pair witnesses, source changes and false claims fail.

Not every valid region has a version-1 witness. Examples include externally
touching distinct components (the numerical editor can retain these), cases
with unavoidable extra eliminant roots, and cases exceeding subdivision or
arithmetic budgets. They return `BREP_PROFILE_CERTIFICATE_UNPROVEN`; this means
unproved, not invalid or intersection-free.

## Extended retained representations

General polynomial quadratics, noncircular positive-weight rational quadratics
and rational cubics now use the general profile kernel. Original poles/weights
are retained in normalization, Booleans and extrusion. Nonuniform affine
transformations of circles therefore retain rational elliptical arcs.

Numerical Green area for rational spans uses outward homogeneous jets through
order five and Simpson's fourth-derivative remainder. Failure to meet its
enclosure budget refuses orientation. The production exact root finder also
cancels common homogeneous factors by aligned fraction-free pseudo-quotients,
checking identities against every original coefficient before reduction.

Euclidean offset of a general conic/cubic is still outside the line/circle
offset carrier family and explicitly refuses for filled-region operations.
An additional normal-offset curve API now supplies a pointwise error proof;
it does not silently substitute for a filled-region offset with round joins.
Production Bézier admission now supports clamped spans of degree 1..8. Scalar
line/common-coordinate and stationary-root reductions also support degree eight.
The general paired implicit reducer remains cubic; other pairs use the existing
conservative curve intersector and explicitly refuse unresolved results.

## Exact area certificates

`certify_measure` and `verify_measure` use scope
`represented-profile-topology-and-area`. The certificate embeds a topology
certificate, an integration depth and two reduced decimal integer ratios in mm².
The verifier rechecks topology and reconstructs the Green enclosure independently;
it rejects a claimed interval that excludes either computed bound. A wider valid
interval is allowed. Decimal strings avoid rounding through JavaScript/JSON.
Unknown fields/scopes, noncanonical ratios, nonpositive denominators, stale
sources, and excessive work are refused. The producer's requested maximum width
does not relax arithmetic or subdivision budgets.

Geometry ABI: `brep_profile_certify_measure`,
`brep_profile_verify_measure_certificate`. The standalone Rust checker supports
`certify-measure` / `verify-measure` and accepts `maximumAreaWidthMm2` when generating.

## Exact partial curve agreement

`certify_curve_agreement` and `verify_curve_agreement` use scope
`represented-curve-affine-parameter-agreement`. Two supplied parameter intervals
define a correspondence, including reversed traversal. Their interior knots are
pulled back into a complete common parameter partition. On every interval the
checker reconstructs exact homogeneous nets and verifies both polynomial
identities `Xa*Wb-Xb*Wa=0` and `Ya*Wb-Yb*Wa=0`. Positive weights exclude poles.
This supports different degrees, common homogeneous factors, partial overlap,
and multi-span storage without numeric tolerances or sampled equivalence.
Source bits and both parameter intervals are bound in the certificate.

The exact check is also attempted before numerical duplicate agreement in the
production Boolean and even-odd normalization paths. Failure is not a negative
intersection decision: the existing conservative handling continues.

Geometry ABI: `brep_curve_certify_agreement`,
`brep_curve_verify_agreement_certificate`. Standalone modes:
`certify-agreement` / `verify-agreement`.

## Projective parameter agreement

`certify_projective_curve_agreement` / `verify_projective_curve_agreement` add
scope `represented-curve-projective-parameter-agreement`. A positive supplied
scale defines `phi(u)=scale*u/(1+(scale-1)*u)` on the normalized domains. Knot
preimages are computed exactly, every common interval is checked, and local
positive powers rescale the second homogeneous Bernstein net. Both coordinate
identities are verified again. Source bits include the map scale. Negative,
zero, nonfinite or excessively conditioned scales are refused before division.
This covers this family of nonlinear correspondences, not every nonlinear map.

ABI operations: `brep_curve_certify_projective_agreement` and
`brep_curve_verify_projective_agreement_certificate`. Standalone modes:
`certify-projective-agreement` / `verify-projective-agreement`.

## Endpoint contact sets

`certify_endpoint_contacts` / `verify_endpoint_contacts` prove the complete
intersection image set of two clamped spans is exactly their common endpoints,
using a checked support-plane witness or an exact annihilator/root census.
Scope: `represented-endpoint-intersection-set`. Both zero and two common
endpoints are supported; coincident interiors and undisclosed intersections
are refused. This does not relabel a touching compound or touching hole as a
manifold solid. A two-arc lens is now admitted by the profile certificate using
exact Green orientation instead of its degenerate two-vertex chord polygon,
and by extrusion using the independently checked endpoint contact set.

ABI operations: `brep_curve_certify_endpoint_contacts` and
`brep_curve_verify_endpoint_contacts_certificate`. Standalone modes:
`certify-contacts` / `verify-contacts`.

## Checkable normal-offset approximation

`certify_offset_curve` / `verify_offset_curve` prove scope
`represented-normal-offset-curve-approximation`. The output is an explicitly
approximate polyline over a complete increasing partition of the source active
domain. On each interval the checker reconstructs the original homogeneous
net and exact tangent numerator `T=(X'W-XW',Y'W-YW')`. Rational Bernstein ranges
and integer square-root enclosures bound the unit left normal `(-Ty,Tx)/|T|`.
The rational Bernstein coefficients of `C(u)-L(u)` account for the actual
binary64 output endpoints. Adding the signed normal displacement yields a
whole-interval Euclidean error bound against the parameterized normal offset.
Thus samples, numerical error estimates and producer success are not trusted.

An explicit regularity check enforces exact position and positively aligned
tangent direction at internal knots and periodic seams. Corners, singular
normals, unproved intervals, collapsed parameters and exhausted budgets refuse.
Zero distance permits curve approximation without inventing a normal at a cusp.
Limits are 2048 output segments, 8192 producer visits, 24 subdivision levels,
deviation in [1e-9,10] mm and distance within +/-1e6 mm. Certificate vector sizes
are checked before decoding their elements. The output has
`geometryStatus: bounded_approximation`; it is not an exact retained offset.

ABI operations: `brep_curve_offset_certified` and
`brep_curve_verify_offset_certificate`. Standalone modes:
`certify-offset` / `verify-offset`. Filled-region offsets, corner joins and
global offset self-intersection cleanup require separate proofs.

## General algebraic parameter correspondence

`certify_nonlinear_curve_agreement` / `verify_nonlinear_curve_agreement` use scope
`represented-curve-nonlinear-image-agreement`. No map formula or numerical
inversion is supplied. For two retained single-span parameter intervals, the
producer finds a nonconstant polynomial relation `H(s,t)` over exact rationals.
The checker independently verifies:

- both source coordinate projections are strictly monotone (isolated stationary
  jets are allowed), and corresponding endpoint images agree exactly;
- `H(s,0)` and `H(s,1)` have opposite strict signs for `0<s<1`, using exact Sturm
  root counts, and the leading coefficient in `t` has no interior zero;
- pseudo-remainders of both homogeneous coordinate equations modulo `H` vanish.

The boundary signs give a real `t` for every `s`. The checked division identities
then give equality of both coordinates; the leading-coefficient check excludes
exceptional interior divisions. Strict projection monotonicity proves uniqueness
and, with matched endpoints, complete image coverage in both directions.
Endpoint coverage follows by continuity. This includes algebraic, non-rational
maps such as `t=s^(2/3)` between `(60s²,s⁴)` and `(60t³,t⁶)`.

The producer uses primitive polynomial PRS in `Q[s][t]`; the verifier checks the
supplied relation and does not trust that search. Common homogeneous factors
are canceled only through checked exact quotients; positive source weights
exclude zeros of those factors on the active domain. Coefficient counts, degrees,
integer sizes and work are bounded. Unknown schema fields, altered coefficients,
wrong branches, stale inputs and false image coverage fail. The API currently
requires one active span per supplied interval and a shared monotone projection;
unpartitioned multi-span or non-monotone images can still be unproved.

Production Boolean duplicate admission and even-odd cancellation now attempt
this exact proof, including reversed traversal, while retaining original curves.
ABI: `brep_curve_certify_nonlinear_agreement` and
`brep_curve_verify_nonlinear_agreement_certificate`. Standalone modes:
`certify-nonlinear-agreement` / `verify-nonlinear-agreement`.

## Isolated intersections and self-intersections

`certify_curve_intersections` / `verify_curve_intersections` use scope
`represented-isolated-curve-intersection-set`. The self mode requires exactly
identical sources. It removes the diagonal `t-s` by checked exact division.
A nonzero Sylvester resultant is reconstructed with fraction-free Bareiss
arithmetic. The verifier checks a complete root census in `[0,1]`, then the gcd
of both original coordinate equations on every projected fiber. Exact division
and Sturm counts account for all roots and multiplicities; samples and residual
thresholds never prove completeness. Self events are canonical unordered pairs;
diagonal stationary roots are recorded separately from actual self crossings.
A strictly monotone coordinate can instead prove an empty self-intersection set;
that mode does not claim a complete stationary-jet census or regularity.

The current event producer isolates dyadic parameters to at most 40 bisection
levels, and requires exact binary64 transport of every normalized and global
parameter. Non-dyadic/algebraic events, nonisolated fibers, vanishing eliminants,
unpartitioned multi-span inputs or exhausted budgets explicitly refuse. Existing
native intersection paths remain available; this certificate adds a complete
exact fallback for the admitted cases rather than replacing them with samples.
Degree-eight self events are applied to the original parent before trimming,
so normalization preserves their ownership across stationary splitting.

General high-degree endpoint-only proofs use sentinel 65534 in endpoint/profile
certificates. They reconstruct the nonzero eliminant and all endpoint fibers;
other-wire nesting and manifold restrictions remain unchanged. Sentinel 65535
retains its previous moving-line annihilator semantics.

ABI: `brep_curve_certify_intersections` and
`brep_curve_verify_intersections_certificate`. Standalone modes:
`certify-intersections` / `verify-intersections`.

## Joined normal-offset wires

`certify_offset_wire` / `verify_offset_wire` use scope
`represented-joined-normal-offset-wire-approximation`. Inputs are connected
positive-weight source curves, signed distance, maximum deviation, open/closed
state, join style and an absolute miter extension limit. Exact source knot cells
are reconstructed without converting their original poles into a fitted curve.
Every cell has the preceding whole-interval normal-offset proof with one quarter
of the total error allowance. Its original one-sided jets define each join:

- `bevel`: a straight segment between the two exact normal-offset endpoints;
- `miter`: the intersection of the exact offset tangent lines, bounded through
  rational boxes and square-root enclosures, with an enforced extension limit;
- `round`: the circular minor arc centered on the exact common source point.
  Exact cross/dot signs prove directed angular coverage with no skipped or
  backward edges. Radial projection bounds account for rounded vertices; each
  subarc is at most 90 degrees, so `sagitta <= chord²/(4*radius)` bounds the entire
  arc/chord deviation. Antipodal rays use the documented counterclockwise tie.

The independent checker binds and verifies every cell, output vertex, join,
style, distance and tolerance; closed seams have the same coverage checks.
Output is a polyline with a Hausdorff bound to the complete joined target wire.
Work and output are bounded to 128 source cells and 2048 output segments. Miter
parallel/reversal cases may refuse; bevel and round provide explicit alternative
styles. Singular source normals and unproved error claims refuse.

This proves the joined normal-offset wire, not global filled-region clipping,
Minkowski erosion/dilation, offset injectivity, tessellation accuracy or 3D solid
validity. The existing exact line/circle filled-region offset is not silently
relabelled. New wire output has `geometryStatus: bounded_approximation`.
ABI: `brep_profile_offset_wire_certified` and
`brep_profile_verify_offset_wire_certificate`. Standalone modes:
`certify-offset-wire` / `verify-offset-wire`.

## Algebraic intervals, multi-span images and global offset cleanup (2026-10-16)

`represented-algebraic-curve-intersection-set` represents isolated events by
squarefree rational polynomial factors, exact isolating intervals and a checked
rational-univariate parameter map. It never asserts that an irrational root is
an exact binary64 parameter. A separating projection `z=s+k*t` is selected from
seven bounded candidates. Unit-checked modular Euclid splits nonunits into
coprime factors; the checker reconstructs the resultant, factor census, radical
fiber gcd and substitution identities. Exact Sturm counts cover all projected
roots, including outside-domain roots, reverse self pairs and stationary
points on the removed diagonal. Returned numeric intervals are outward-rounded.
The requested width bounds exact source-domain parameter enclosures; binary64
outward conversion can add at most one representable step per endpoint.

Production Boolean and authored-loop normalization use this checked census when
older isolated-event or numerical searches are unresolved. Midpoints used for
numerical trims do not inherit the exact-root certificate. Rounded trims and
constructed solids retain their numerical validation status.

`represented-multispan-curve-image-agreement` walks exact source knot cells in
image order. A source knot may have an irrational preimage in the other curve.
Each step proves the common bivariate relation on the overlapping image range,
with common monotone coordinates, exact endpoint order and boundary quotient
signs. The checker reconstructs the full walk and refuses omitted tails,
reordered cells, changed relations and discontinuous source seams. Local axes
may change between steps, so closed piecewise curves need no global monotone
coordinate. Admitted degree is 1–8 with positive weights; at most 128 exact cells
per curve, 256 steps and six subdivision levels. A proof requiring a different
local partition remains unresolved rather than being accepted numerically.

`represented-offset-positive-region-regularization` combines source topology
and joined-wire bounds with a global exact rational segment arrangement. All
crossings and collinear overlaps are split, signed winding jumps are aggregated,
and only boundaries between positive and nonpositive winding are retained.
Components and holes are assigned exactly. Rational intersection vertices are
rounded only after checking a separate half-tolerance displacement budget,
no new crossings/contacts, ring orientation and hole ownership. The checker
reconstructs the global arrangement from the embedded checked wires rather
than trusting producer output rings.

This last scope certifies positive-fill cleanup of bounded offset wires. It is
not a universal Minkowski erosion/dilation certificate: collapsed erosions and
singular distance sets require a separate distance-to-source-region proof. The
existing exact line/circle region-offset operation is preserved. The new
`brep_profile_offset_regions_certified` operation returns individual component
profiles, the full certificate and explicit bounded-approximation metadata.
Wire approximation is bounded by half the requested deviation and output vertex
rounding by the other half; discarded internal wire segments are not included
in a global raw-wire Hausdorff claim. Input: at most 1022 wire edges; arrangement:
at most 32768 incidences and 8192 atomic edges, with bounded side-witness search.
Budget exhaustion, zero jets in offset sources, unresolved hole ownership or
topology-changing output rounding produce an explicit refusal. Point contacts
between separate positive components do not certify a manifold compound solid.

All three scopes have independent native Rust checker modes (stdin JSON,
maximum 2 MB): `verify-algebraic-intersections`, `verify-multispan-agreement`,
and `verify-offset-regions`. The checker is executable proof validation over
represented input bits; its implementation is part of the trusted code base,
not a machine-checked proof of the Rust implementation itself.
