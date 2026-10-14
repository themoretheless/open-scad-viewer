# Parameterwise retained decomposition error

`curve_decomposition_certificate::inspect` compares one original NURBS knot
span with one actual retained rational Bezier under normalized traversal.
Original positive-weight homogeneous blossom controls are obtained with
outward interval arithmetic; rounded extraction controls never replace the
original curve. Both curves use the same coordinate origin and homogeneous
scale, which cancel from their rational difference.

Let A(t)/a(t) be the original curve and B(t)/b(t) the retained curve, each of
degree p in Bernstein form on [0,1]. Their difference has numerator
N(t) = A(t)b(t) - B(t)a(t). Product coefficient k is the sum over i+j=k of
binomial(p,i) binomial(p,j) / binomial(2p,k) times the corresponding homogeneous
cross difference. Every coefficient operation is rounded outward. The
Bernstein convex hull encloses N over the complete span. Positive denominator
controls give a(t)b(t) >= min(a_i) min(b_i). Dividing the outward Euclidean
numerator norm by this separated positive denominator bounds both directed
parameterwise distances. If positivity, layout, numeric range or the complete
product-pair budget cannot be proved, no error upper bound is returned.

The current implementation supports degrees through25; the required binomial
coefficients (n<=50) fit exactly in binary64 and their integer construction
fits u64. It refuses periodic source/retained encoding and expects an actual
single Bezier target; a caller must supply all nonempty original knot spans.
Its explicit product budget counts (p+1)^2 coefficient pairs per span.

This is a curve certificate. To transfer it to retained sweep walls, bind
each actual face to both corresponding retained endpoint Beziers and prove
their weights agree along the ruled direction. Then each actual ruled point
is a linear blend of those endpoint curves, and the maximum of their bounds
also bounds the whole wall span. Compose that bound outward with the original
trajectory/interpolation and correction bounds. All spans, sections and faces
must be covered under shared budgets before returning an aggregate.

Caps additionally require actual retained region validity, original ideal
material ownership, per-loop parameter pairing and the existing certified
plane transfer. Use original endpoint contour error plus correction and
decomposition errors before applying the parallel/general filled-region
lemma. The curve certificate alone supplies neither wall/cap ownership nor
regularity, global embedding, shell orientation or smoothness. Transport and
body integration of this new contribution remain pending.

## Filled retained caps

The endpoint binder keeps the original outer/hole partition and reproduces the
retained decomposition policy: direct coefficient copies for already segmented
clamped curves, native decomposition otherwise. Every nonempty original knot span
is paired with its normalized retained Bezier curve. Both endpoints share one
product budget; incomplete coverage returns no endpoint pair.

The actual B-rep cap coedges must match these decomposed curves, allowing cyclic
start and reversal within each wire, with the outer and hole roles unchanged.
The existing planar-region audit then proves identity of the filled retained
region with the decomposed contours. This does not claim exact identity with the
original NURBS contours. Each endpoint decomposition error is added outward to
the ideal endpoint contour error and optional correction displacement before
applying the parallel-plane epsilon or general sqrt(2) epsilon transfer.

Original ideal-domain ownership and projection remain mandatory independent
premises. Initially the low-multiplicity ring had no ideal-domain certificate.
The exact-insertion route below now closes that obligation for this fixture;
public WASM tests certify its complete boundary budget, with and without a hole.
The constructor-owned complete boundary certificate can now set
`continuousBound=true` when all its premises are certified. The scalar
decomposition report alone does not establish this flag; arbitrary rounded
source decompositions still require original-span topology certification.

## Original profile domains with exact binary64 insertion

An additional sufficient path accepts clamped nonperiodic profiles whose
homogeneous knot insertion is exactly representable. Each original binary64
number is decoded into a signed odd i128 mantissa and a power of two. Checked
integer arithmetic verifies every multiplication, sum, knot difference, division
and final dehomogenization against the candidate binary64 result. Overflow,
underflow, a nonrepresentable quotient, or a mantissa alignment outside the
checked range refuses this identity path. No epsilon equality is used.

Knot insertion raises each internal knot to degree multiplicity and copies each
active Bernstein block with its original parameter domain. Consequently the
result is exactly the original rational curve, and existing continuous trim
simplicity, separation and winding certificates apply to the original profile.
The direct-copy path remains unchanged for already segmented profiles.

This sufficient route does not cover arbitrary rounded decompositions. A small
Hausdorff decomposition error alone cannot prove original simplicity or winding;
profiles failing exact insertion continue to require an original-span topology
certificate. Periodic profiles remain unproved by this route.

Nonperiodic unclamped endpoint profiles now have an additional exact-blossom
route in Rust. Each active span's Bernstein controls are homogeneous blossom
values at repeated lower/upper endpoints. Checked dyadic operations must prove
every interpolation and dehomogenization exactly; a rounded result refuses the
identity route. Output is limited to 4096 Bernstein controls. These exact
segments feed the existing continuous contour simplicity, separation and
winding certificates. This extends original-domain ownership when exact
extraction succeeds; it does not accept arbitrary rounded extraction.

`unclamped_exact_blossoms_certify_hollow_original_domains` exercises a quadratic
unclamped closed contour and an inner loop, all four original active domains,
decomposition bounds, rounded-dehomogenization refusal and periodic refusal.
Focused native exact-extraction tests passed 2/2
(`sweep-unclamped-domain-native-final.log`). The first diagnostic run used a
weight producing an exactly representable mean and correctly accepted it; the
negative fixture now requires a nonrepresentable 1/3 mean. The native sweep
regression suite passed 153/153 (`sweep-unclamped-domain-sweeps.log`). WASM
publication and public constructor qualification of this new route remain
pending; the previously published kernel does not include this change.
Public WASM regressions have now been added in `nurbsSweepContourAudit.test.ts`
and `nurbsProgressiveMiter.test.ts`: exact unclamped hollow contour ownership,
rounded extraction refusal, source immutability and ideal endpoint material
domains through the miter adapter. Direct `vue-tsc --noEmit` passed
(`sweep-unclamped-domain-types.log`). The new kernel build was started; its
result and these public runtime tests are not yet qualified.
A full-constructor regression was also added in
`sweepRetainedDecomposition.test.ts`: ideal hollow material domains, retained
wall/cap decomposition and the constructor-owned complete boundary certificate.
Its direct type check passed (`sweep-unclamped-domain-constructor-types.log`).
Runtime qualification still awaits the live kernel optimizer; release Rust
compilation succeeded in 1m10s, but that is not a completed WASM publication.

The build subsequently completed successfully: 10,739,811 bytes, identical
generated/public SHA256
`225525c162285c242aa7580e4d3049d2cc25c230124cd4b9bd98d9c38f9b9a8e`.
The four public suites produced 41 passing tests and one failing full-constructor
regression (`sweep-unclamped-domain-public.log`). Contour and ideal endpoint
domain adapters passed. The constructor stopped earlier in the Rust wall audit:
`inspect_wall_geometry_with_loops` in `level_certificate.rs` requires clamped
endpoint poles to prove loop closure and therefore rejects this exactly closed
unclamped source. Keep the failing regression as the completion requirement;
full constructor qualification is not achieved. This wall-ownership closure
gate is the next concrete integration gap to fix.

The Rust wall-ownership gate now accepts exact active-domain endpoint closure
for nonperiodic unclamped profiles via the same checked blossom extraction.
Clamped profiles keep their direct endpoint-pole proof. Rounded extraction or
unequal active endpoints still refuses ownership; no tolerance was introduced.
The native regression accepts exact closure and rejects a displaced seam while
using zero wall-analysis budgets, deliberately proving only the ownership gate.
Focused tests passed 2/2; the sweep suite passed 153/153
(`sweep-unclamped-wall-closure-native-final.log`,
`sweep-unclamped-wall-closure-sweeps.log`). A new kernel build is live
(`sweep-unclamped-wall-closure-wasm.log`); its publication and the previously
failing full-constructor runtime regression remain pending.

The full-constructor regression now also builds the checked-in Rush fixture
`examples/rush/miter-unclamped-hollow.r` and requires the complete boundary
certificate in both construction and nativeGeometry evidence. Direct type
checking passed (`sweep-unclamped-rush-types.log`). The Rust release build for
the wall-closure change succeeded; its live optimizer has not yet published
the new kernel. Public runtime and Rush outcomes are still pending.

The STEP export matrix now includes `rush-unclamped-hollow.step` from this
Rush source, requiring both complete boundary and native Solid certificates.
Its independent analytic volume is 125/8 mm³: each quadratic quadrant has
signed area 5/12, the four quadrants give 5/3, the quarter-scale hole removes
1/16 of that area, and the extrusion length is 10 mm. Export and OCCT
qualification remain pending until the new kernel and constructor pass.

The wall-closure kernel build completed: 10,740,350 bytes, identical generated
and public SHA256
`2dd56e829d8853d6b373d0df7482e544a68bb3495d83cc5d3094da6cedb24972`.
The first constructor rerun correctly rejected the test hole's same orientation
as the outer contour (`hole-orientation`); the source and fixture now reverse
the hole. The complete constructor/Rush regression then passed, followed by
all four public suites, 42/42 tests in 49.80s
(`sweep-unclamped-wall-closure-public-final.log`). The new unclamped fixture's
complete boundary certificate is true and within budget. This result is specific
to exactly extractable nonperiodic profiles; periodic and rounded-original
topology remain open. Fresh STEP matrix export is live
(`sweep-unclamped-step-export.log`); OCCT results remain pending.

Fresh STEP export and independent OCCT verification subsequently completed:
35/35 selected cases passed with 504 whole-domain cap coedge checks. The
unclamped hollow case has a native Solid certificate and imports as a valid
closed Solid with 10 faces, 24 edges and volume 15.625000000000002 mm³ versus
the independent 125/8 mm³ reference. Details and limits are recorded in
`docs/qualification/sweep-coverage-2026-10-01/sweep-unclamped-domain.md`.
This closes the fixture's STEP qualification, not periodic/rounded-original
topology or the all-mode library requirements.

## Periodic active-domain extension (native qualified, publication pending)

Exact blossom extraction now also accepts periodic original profiles when
every span is exactly representable and the first/last active endpoint points
agree exactly. Validation's tolerance on exterior knot translation alone does
not prove this seam. A valid almost-periodic exterior-knot perturbation refuses
the exact route. Original contour ownership then uses the same exact extracted
segments, including holes.

The decomposition error certificate now accepts periodic originals on individual
active spans: the stored rational basis is unchanged by periodic continuity
metadata. It still requires nonperiodic clamped retained segments and does not
prove seam closure by itself. Native regressions cover all four periodic spans,
translation bounds and product-limit refusal. Focused exact extraction tests
passed 2/2, decomposition tests 3/3, and sweep tests 153/153; direct type checking
passed. Logs use the `sweep-periodic-original-domain-*` and
`sweep-periodic-decomposition-native.log` prefixes.

Retained wall/cap adapters now permit periodic source decomposition, preserving
source periodicity equality between sections. The full periodic hollow boundary
constructor regression has been added; its outcome is pending the new WASM
build (`sweep-periodic-original-domain-wasm.log`). The last published unclamped
kernel does not include this extension. Full public, Rush, STEP and UI periodic
qualification is not yet achieved.

Rush previously could not author this storage mode: `nurbs_curve` exposed no
periodic argument. Its Rust text lowering now accepts the optional flag, converts
it to the schema boolean and the canonical runtime preserves it (default false
unchanged). Native frontend 9/9 and runtime 56/56 tests passed. The separate
language WASM rebuilt successfully (2,278,075 bytes). Public compilation of
`examples/rush/miter-periodic-hollow.r` confirmed both curve nodes retain
`periodic:true` (`sweep-periodic-rush-public-compile.log`). A full constructor/Rush
regression also requires the complete boundary certificate and nativeGeometry
evidence. Direct type checking passed. The geometry WASM build remains live;
that combined runtime regression and periodic STEP/UI qualification remain
pending.

The periodic kernel was published at 10,740,948 bytes, generated/public SHA256
`1a1870a538a799d272087ab2a3168b197031ccfe80fd11ad87b3f4b1ca8f1ac4`.
Five public suites produced 43 passing tests and one constructor failure
(`sweep-periodic-original-domain-public.log`): the Rust section-loft constructor
explicitly rejected periodic original curves before its existing decomposition
and exact-join checks. That restriction has now been removed while preserving
bounded 3D input, span correspondence, exact joins and cap orientation audit.
Native retained-loft tests passed 2/2, including a periodic hollow body's valid
10-face/24-edge topology and refusal for wrong hole orientation or a broken
join (`sweep-periodic-section-loft-native.log`). Broader loft regressions and a
new kernel build are live; complete public periodic qualification is pending.
The broader native rational-loft regressions subsequently passed 9/9
(`sweep-periodic-section-loft-regression-native.log`). Kernel publication remains
pending (`sweep-periodic-section-loft-wasm.log`).

That build subsequently completed: 10,743,643 bytes, generated/public SHA256
`6cfe87add9229b03991147ada749311bd52c74ce94f4a4ceab47619d31f8764e`.
All five public suites passed 44/44 tests in 50.45s
(`sweep-periodic-section-loft-public.log`). The exactly extractable periodic
hollow source now passes the complete constructor and Rush chain with
`continuousBound=true` and within-budget evidence in nativeGeometry. Fresh
36-case STEP export is live (`sweep-periodic-step-export.log`); independent OCCT
verification and periodic UI qualification remain pending. This does not prove
arbitrary rounded periodic contours or all-mode global guarantees.

Fresh export and OCCT verification subsequently passed all 36 selected cases,
including the periodic hollow fixture, with 520 whole-domain cap coedge checks.
The new imported Solid's volume matches 125/8 mm³ to relative error
1.1368683772161603e-16. Its native eight profile seams certify exact G2, and a
public regression now checks the one-unit-short work-budget refusal; the
four-test retained-decomposition suite passed. Details and remaining scope:
`docs/qualification/sweep-coverage-2026-10-01/sweep-periodic-domain.md`.

The STEP exporter now includes the periodic Rush hollow fixture with mandatory
periodic storage, complete boundary and native Solid gates and the independent
125/8 mm³ analytic volume. Its fresh export and OCCT checks await the complete
constructor gate. The existing 35-case qualified report remains unchanged.

The interval decomposition-error path itself already covers nonperiodic
unclamped endpoint spans. The native regression
`unclamped_endpoint_spans_keep_parameterwise_error_and_work_limits` checks all
three active spans of a quadratic rational curve, including both endpoint
spans, an interior retained-pole displacement, the exact product budget and
source immutability. This does not close original contour topology or permit a
periodic source in the decomposition-error API. Native focused result: 3/3
tests passed (`sweep-unclamped-decomposition-native.log` in the qualification
directory).

## Actual retained wall domains and active body coverage

Matching ruled-surface coefficients alone is insufficient for a two-sided
wall-set bound: a clipped UV domain can omit ideal wall points while retaining
identical coefficients. Both exact-copy and numerical-decomposition binders
therefore require each actual wall face to have no holes and an outer wire of
four normalized, clamped rational degree-one pcurves. Their endpoint poles must
traverse precisely the four unit-square sides, allowing cyclic start and either
orientation. Positive weights make each pcurve a monotone bijection onto its
straight segment, so the filled UV region equals the complete unit rectangle.

The single retained body must own all its shells, and its closed shells must
reference every stored face exactly once. Omitted, duplicated and orphan faces
cannot supply coverage. Owner/shell/face cardinalities are checked before walks;
face work is limited by maxFaces plus the two possible end caps. The complete
maximum applies only after both domain and body coverage obligations succeed.

These checks bind geometric face sets; they do not replace original-law/frame
certification, world-coedge agreement, shell winding, embedding or ideal-profile
ownership. Full continuousBound remains unpromoted.

## Native ownership and wall-domain audits (2026-10-04)

The ownership premise now runs in Rust through
`sweep_retained_body_coverage_audit`; its public WASM integration is qualified.
The rectangular trim and exact surface/pcurve/world-edge composition has also
been implemented in Rust as `sweep_retained_wall_domain_audit`. Its native and
raw compiled WASM ABI fixtures pass. The host resolves references and preserves
one shared `maxExactWork` across wall faces, subtracting each native report's
consumed work. Cyclic starts and complete reversal preserve the domain, while
wrong world-edge identities and an exhausted exact budget refuse. Shell winding,
embedding, and full ideal-to-retained continuousBound remain separate.
The optimized public wall-domain integration is still awaiting packaging and
regression; this paragraph is not a claim that that integration has passed.
