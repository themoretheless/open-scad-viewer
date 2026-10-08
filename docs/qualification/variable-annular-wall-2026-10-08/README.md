# Original annular wall coverage

## Intrinsic same-face evidence

All 31 original surfaces of each of four variable annular bodies were checked
for continuous exclusion of chords aligned with endpoint normals at sin² <=
1e-4. Sixteen full-chart certificates per body succeeded; fifteen remain
unproven. The original model is serialized before/after and must remain equal.

The certified curved faces include 2/3 for increasing radius and 3/12 for
decreasing radius. This is full parameter-chart coverage, not finite sampling.
The remaining curved charts need adaptive coverage or explicit treatment of
singular domains. Failure to certify does not establish a thin region.

Distinct-face pairs, material-volume admission and whole-wall minimum remain
separate requirements. No global thickness or wall safety is claimed.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-wall -- /tmp/self-wall.json
```

`self-wall.json.gz` records face identifiers, exact scope, bounds and reasons.
## Initial whole-wall audit

All 496 original face pairs, including 31 self pairs, are enumerated in each
body. The conservative lower bound remains zero: 91 pairs per body still have
zero lower bounds. The material chord is unproven because its volume stage is
unproven. No wall interval or convergence is claimed.

Per body: 17 curved self pairs remain unresolved, 14 planar self pairs are
excluded, 42 coplanar pairs and 172 endpoint-normal pairs are excluded. The
remaining pairs carry distance/control-hull bounds, including 8 or 10 refined
face-distance bounds depending on arc direction.

`whole-wall-initial.json.gz` preserves every face-pair identifier and reason.
This initial route does not allocate the explicit whole-chart linear/quotient
injectivity budget. A later route must separately qualify embedded boundary,
material roles and orientation before the material-chord claim can succeed.

## Adaptive same-face coverage

At minimum threshold 4.74999 mm and sin² <= 1e-4, adaptive complete-chart
pair coverage certifies two further curved surfaces in each body. Combined
with intrinsic results, 18/31 same-face cases are certified. Remaining face
indices: 0,5,7,8,10,16,17,20,21,24,25,28,29. The report preserves pending
regions and exclusion counts; finite work is not treated as safety.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-wall -- /tmp/adaptive.json adaptive
```

An explicit volume audit now accepts separately bounded whole-chart
injectivity work. Its regression requires embedded boundary, consistent
material roles and independent outward orientation together; invalid budgets
are refused. Original default admission behavior remains unchanged.

## Initial explicit-volume result

With explicit chart injectivity work, all four bodies pass original boundary
agreement, exact joins, trim validity and all 31 face injectivity checks.
Pair classification remains incomplete under the initial 256-cell/4096-domain
per-pair budget. Boundary admission therefore refuses volume admission, and
nesting/orientation stages are correctly not run. No volume claim is made.

`volume-initial.json.gz` preserves the original stage results. A repeat uses
the earlier successful embedding audit's per-pair 4096-cell/100000-domain
limits, global 1000000-cell/8000000-domain limits, and explicit independent
orientation work. Pair reasons are recorded to distinguish remaining causes.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-volume -- /tmp/volume.json
```

## Regular-face refinement

Repeating the eleven regular unresolved charts at 1024 cells/8192 spans per
face gives no additional certificates in any of the four bodies. The reports
retain their explicit selected-face list; omitted singular faces 0/10 remain
unqualified. The verified total remains 18/31, not 29/31.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-wall -- /tmp/refined.json adaptive 1024 5,7,8,16,17,20,21,24,25,28,29
```

The next algorithm investigation is derivative-column scaling in the original
intrinsic chord inequality. Any stronger bound must preserve the full original
pair domain and pass independent analytic and rejection regressions.

## Complete original volume admission

All four bodies pass the repeated native volume audit. Exact boundary/joins,
valid trims, 31 injective faces, all face-pair classification and embedded
boundary pass together. Material nesting roles are consistent. Each original
shell independently proves outward orientation on its first attempted ray.
`proven=true` therefore covers all volume prerequisites for these specimens.

`volume-qualified.json.gz` records all stage outcomes and individual pair
reasons. This supersedes the initial low-budget volume failure while preserving
that evidence. It does not certify global minimum wall thickness, unique regular
pole tangency, general fillets or named-part STEP acceptance.

## Balanced intrinsic differential bound

The native self-chord inequality now also evaluates a fixed positive diagonal
rescaling of derivative columns. With D diagonal, chord=M D (D^-1 delta) and
n.(J D)=0; the original intrinsic inequality therefore remains valid. All
products and ratios use outward intervals. The original bound is retained,
and the smaller of two proved upper bounds is used. Unsupported or nonfinite
balancing does not weaken the original proof.

Nine native intrinsic/adaptive tests pass. An affine parameter-unit regression
uses an independent parabola chord oracle; it explicitly demonstrates an
inconclusive original bound and a successful balanced certificate for the same
original geometry. Strong curvature, collapsed/discontinuous charts and a
rational half-cylinder with a short exact normal chord remain refused.

Repeating the same eleven annular faces at the same 1024-cell budget gives no
additional certificates. The balanced report is preserved as evidence; total
coverage remains 18/31. The change improves parameter-unit robustness, but
these annular charts still require a stronger geometric exclusion or further
complete adaptive work. No new WASM/UI package is claimed for this change.

## Original secant direction exclusion

Adaptive pair coverage now bounds the original position difference and each
original endpoint normal. If one endpoint has a strictly insufficient squared
cosine with every enclosed secant, the entire pair is excluded. Fixed positive
normalization and outward interval arithmetic preserve the original geometry.
Overlapping position boxes, singular normal bounds and exhausted work do not
produce a certificate. Work counts include every attempted normal span.

Ten native tests pass, including direct oblique-secant exclusion, overlapping
boxes and zero-work refusal, and a direct true half-cylinder diameter refusal.
The existing short-normal-chord full-coverage rejection also passes. Evidence:
`secant-tests.log.gz`. The controlled eleven-face comparison completed for all four original bodies
at unchanged 4.74999 mm threshold, 1e-4 squared-sine tolerance and
1024 cells/8192 spans per face. Every selected face remains unresolved.
Full same-face coverage therefore remains 18/31. Face 5 no longer reaches
the earlier floating-point resolution stop: 195 or 265 original pair domains
are excluded by direction, and calculation reaches the cell-work limit.
Other selected faces have 6–261 direction exclusions. These are covered
subdomains, not extra complete-face certificates. Reports and original-source
identity checks are retained in `self-wall-secant.json.gz` and its log.
No new WASM/UI integration or whole-wall certificate is claimed.

The focused native B-rep regression
`source_support_shell::tests::admitted_annular_body_checks_original_radius_patches`
passes after the secant change (one test executed). It checks original body
ownership, bounded adaptive work, all 27 face reports, private proof reuse,
public-report mutation resistance, and refusal to assert whole-wall coverage
while curved self or distinct face pairs remain unresolved. Evidence:
`secant-source-body-tests.log.gz`. This fixed-radius body regression does not
replace the four variable-radius specimen audits or named-part acceptance.

## Expanded original pair coverage

The four-body repeat at 4096 cells/32768 spans per selected regular face
proves complete same-face coverage for faces 7 and 8 in every specimen.
Face 7 uses 1689 cells/7312 spans and face 8 uses 1083 cells/4877 spans.
Both retain the original 4.74999 mm threshold and squared-sine tolerance
1e-4, with zero pending domains. Combining these two distinct faces with
the previously recorded 18/31 full-chart results gives 20/31 per specimen.
Eight other regular faces remain unresolved at the expanded budget.

Separately, face 5 remains unresolved even at 10000 cells/80000 allowed
spans: the two radius orders leave 19 and 11 pending domains, spending
50617 and 49906 spans respectively. Direction exclusions number 2303
and 1937. These facts do not prove a thin wall or global wall safety.
Singular end faces 0/10 remain outside this regular-face audit.

Both completed reports and logs are preserved in
`self-wall-secant-regular4096.*.gz` and `self-wall-secant-face5.*.gz`.
Original model serialization is checked unchanged for every case.
No new WASM/UI package is claimed for this native change.

## Combined original differential minors

The intrinsic inequality also uses the sum of lower bounds for the three
squared coordinate minors of the original mean differential. By
Cauchy--Binet, their sum is a lower bound for det(M^T M), and
sigma_min(M)^2 >= det(M^T M)/trace(M^T M). This retains all source
differentials and the existing single-minor bound. Non-separated minors
contribute zero to the lower sum. All arithmetic is outward rounded.

Eleven native tests pass, including a direct oblique differential enclosure
with a strictly stronger combined bound and the independent original parabola
oracles and short true normal-chord rejection. `gram-tests.log.gz` retains
the results. Annular improvement for this bound remains to be measured.

## Complete expanded-budget regular-face audit

The completed pre-Gram secant audit at 10000 cells/80000 spans per
selected face proves faces 20, 21, 25 and 29 in all four specimens.
Their cell counts are 9687, 6857, 7915 and 9055; span counts are
49397, 33994, 39517 and 43498. Each has zero pending domains.
Together with the distinct previously qualified faces, retained native
proof coverage is 24/31 per specimen at unchanged threshold/angle.

Regular faces 16, 17, 24 and 28 remain unresolved at this budget.
Together with middle face 5 and singular ends 0/10, seven original
charts remain without full same-face qualification. Distinct-face
wall coverage and global minimum thickness are separate open work.
Reports/logs: `self-wall-secant-regular10000.*.gz`.

The focused original B-rep ownership/coverage regression also passes
after the combined-minor change (one executed test). Evidence:
`gram-source-body-tests.log.gz`. The controlled combined-minor
4096-cell specimen comparison remains running; this expanded-budget
result must not be attributed to that newer bound.

The combined-minor comparison at 4096 cells/32768 spans completed
for all four specimens and selected faces 5,16,17,20,21,24,25,28,29.
It gives no additional full-chart certificates at that budget.
`self-wall-gram4096.*.gz` preserves all unresolved original domains.
The subsequent 20000-cell run separately caps spans at 100000;
the diagnostic example now records both independent actual budgets.
The larger run has completed; its complete results are recorded below.

## Remaining regular side charts qualified

All four original specimens fully qualify faces 16,17,24,28 with the
combined-minor bound and 20000-cell/100000-span caps. Their cell
counts are 12253,7197,10265,13867 and span counts
62437,36314,51523,66069 respectively. All have zero pending domains.
The threshold remains 4.74999 mm and squared-sine tolerance 1e-4.
Original model serialization remains unchanged for every case.

Combining these four distinct source charts with the previous native
results gives retained full same-face qualification for 28/31 charts
in each specimen. Middle chart 5 and singular end charts 0/10 remain
unqualified. This is not a global thickness certificate: distinct
source-face wall pairs, material chord coverage and a proved minimum
interval are still required. General fillets, named-part acceptance
and packaging the new native logic into WASM/UI remain open.
Evidence: `self-wall-gram20000.json.gz` and its completed log.

## Whole original normal numerator grid API

`cad_predicates::rational_surface_projected_jacobian_bounds` returns
all exact coefficient signs and outward coefficient magnitude bounds
under one predicate context. It retains the same original input scaling
across projections. An exhausted query returns neither a partial sign
net nor a partial magnitude net. This exposes the complete grid needed
to verify a vanishing boundary factor before using limiting normals.
It does not itself qualify a singular endpoint or any wall pair.

Seven projected-Jacobian tests pass, including whole-grid dimensions,
original plane normal signs/magnitudes, agreement with the selected
coefficient query and complete refusal at insufficient work. Evidence:
`normal-grid-tests.log.gz`. The remaining pole factor and continuous
secant qualifications still need to consume and verify this native API.

## Entire pole normal numerator factor verified

The original whole-grid single-context queries reach ResourceLimit on
these higher-degree poles. The new row-bound API permits complete
row queries and original exact coefficient queries when a row exhausts
its context. Seven predicate tests pass including whole/row/selected
coefficient agreement and exhausted-work refusal.

The original diagnostic now checks every row and every column for
all three normal components at both ends of each of the four bodies.
All eight original grids are complete, and their first two Bernstein
rows are exactly zero for every column, proving the common U^2
factor. Exit inputs are reversed copies and original model definitions
are verified unchanged. This extends earlier rail-only factor checks
to the full meridian domain; it does not prove a regular pole plane,
G1 continuity, self-chord exclusion or global wall thickness.

`annular-pole-normal-grid.*.gz` retains the failed bounded full query;
`annular-pole-normal-grid-refined.*.gz` retains complete grids and
exact work totals. These verified grids are the next input to bounded
limiting-normal and secant qualification.

## Owned native factored-normal rectangles

`nurbs_core::factored_surface_normals::prepare` reconstructs all
original normal coefficient rows under an explicit total exact-work
budget and per-query predicate limits. Every leading row coefficient
must have exact Zero sign. Only then does the private source-bound
fact retain the divided Bernstein net. Exit U reversal is corrected
for the original oriented normal. Rational denominators and chart
lengths are positive, so the factored numerator preserves directions
on the regular interior. Its boundary values enclose one-sided limits.

Rectangle queries use outward de Casteljau restriction on the original
normalized parameter interval. The three native tests cover boundary
and reversed orientation, an independent exact-integer cubic graph
normal formula, missing factor, missing work and out-of-chart refusal.

All eight original specimen end charts qualify this factor fact at
18.3–21.4 million exact operations per chart, below the 100 million
cap. Original source pointers and unchanged model definitions are
checked. Full-chart, full-pole and both pole rail point bounds are
recorded in `pole-normal-native.json.gz`; its completed log and
`factored-normal-tests.log.gz` retain execution evidence.
This factor/normal fact is not yet consumed by whole self-chord
coverage and does not establish end wall thickness or a unique pole
plane. Retained whole self-face qualification remains 28/31.

## Factored original normals consumed by secant coverage

`qualify_with_factored_normals` accepts only a sealed exact-factor fact
whose immutable source pointer is the same original surface. Its
rectangle bounds replace ordinary endpoint normal boxes in the
secant-direction exclusion; a query covers one original Bezier span.
Exact factor preparation work is independently bounded and reported
by the specimen audit. Coverage cell/span limits still apply.

Twelve native chord tests pass, including a bounded pole secant
that becomes excluded with limiting normals, foreign-source refusal,
missing-work refusal and retention of the real half-cylinder diameter.
Subnormal normal/direction scales conservatively refuse normalization
rather than overflow a reciprocal. `factored-chord-tests.log.gz`
retains the results.

All eight original end charts were audited at 1024 cells/8192 spans
after exact factor preparation. No complete self-face certificate
is produced. Entry face 0 spends 1024 cells/5120 spans with 1025
pending domains and no direction exclusions. Its unresolved boxes
contain the shared collapsed point. Exit face 10 has 193 or 249
direction exclusions but leaves 15 or 19 pending domains.
`self-wall-factored-poles.*.gz` preserves every result and source
identity checks. Coverage remains 28/31; a polar intrinsic secant
argument is needed for the collapsed diagonal, rather than excluding
that region or treating it as a regular chart.

## Polar differential primitive and verified rational boundary

The native polar primitive proves S_v=t² B when both authored leading
control rows and weights are constant in V. Original homogeneous
V derivatives then have exactly zero leading rows, and Bernstein
factor removal gives bounded B on the closed chart. A nonzero chord
is a mean differential times (delta_u,delta_v integral(t²)); positive
weighted averaging preserves the column enclosures. The intrinsic
normal/chord inequality therefore handles the collapsed diagonal
without requiring a nonsingular original UV Jacobian.

Three polar tests pass with independent quadratic differential formulas,
original-source and reversed-end bounds, successful local chord proof
where the ordinary chart proof refuses, missing-work and absent-factor
refusal. Twelve existing chord/coverage tests also pass, including true
normal-chord rejection. Reflected parameter endpoints use outward
interval operations. The primitive supports clamped Bezier degrees
2–8 in U and 1–8 in V.

The repeated eight-end 1024-cell audit gives no new complete proofs.
A separate single-cell support diagnostic records one structural
factor refusal on every source end. Their V-dependent rational
weights require factoring the full rational derivative numerator
H_v W - H W_v, rather than the simpler separate homogeneous factors.
No unsupported primitive is admitted. Evidence is retained in
`annular-self-wall-polar-poles.*.gz`, `annular-polar-support.*.gz`,
and the polar differential/chord test logs.

The middle-face combined-minor 20000-cell/100000-span audit also
completed for all four specimens. Both radius orders exhaust 100000
spans at 19586 or 19747 cells, leaving 18 or 11 pending domains.
No complete certificate is produced (`annular-self-wall-gram-middle.*.gz`).
Retained coverage stays 28/31. All broad roadmap requirements remain
open to the extent not separately qualified.

## Exact rational derivative numerator and actual leading term

The predicate API now returns bounded original row or coefficient
signs/magnitudes for H_axis W - H W_axis, with a common positive
scale across XYZ and normalized U/V. Missing work returns no partial
row. Eight projected-Jacobian/predicate tests pass, including rational
V-varying weights whose derivative really has an exact U² factor,
row/selected-coefficient agreement, both axis dimensions, positive
U derivative, work refusal and invalid row refusal.

Every original end was checked for the first two U rows of Q_v.
All first rows are exactly zero, but all eight end charts have at least
one exactly nonzero second-row coordinate. First-order terms cannot
be discarded or replaced by a quadratic factor. The magnitudes in
`rational-derivative-poles.json.gz` are predicate-scaled polynomial
coefficients, not physical distance errors. Failed quadratic-factor
admission is therefore supported by exact signs, rather than a
budget stop or an interval that merely contains zero.

This does not contradict the already verified U² factor in the normal
numerator: the derivative's first-order term is along the pole
longitudinal direction, so its transverse normal product can have
higher vanishing order. A complete general proof still needs to
qualify that decomposition on the original source and bound the
resulting weighted chord. The new derivative API supplies exact
coefficients for that next step. `rational-derivative-poles.log.gz`
and `rational-derivative-tests.log.gz` retain execution evidence.
Retained self-face coverage remains 28/31; no whole-wall or new
WASM/UI qualification is claimed.

## Exact transverse rational derivative decomposition

New bounded predicate queries compute Q_k d_a - Q_a d_k for
normalized U or V, with d=P_(1,0)-P_(0,0) taken from the same
authored original source. The chosen anchor must have exact nonzero
direction. A floating-point heuristic chooses the anchor index only;
the direction and cancellation are recomputed by exact expansions.
Row and selected-coefficient APIs preserve all exact leading terms
and return no partial row when work is exhausted.

All eight original end charts prove zero leading transverse U row
and zero first two transverse V rows, for both non-anchor coordinates
and every V coefficient. This establishes an exact U factor in the
transverse U numerator and an exact U² factor in transverse V. The
previously measured nonzero tangential first-order V term is retained.
No original control point or weight is changed. Original model
serialization is checked unchanged in every case.

Derivative decisions now include a positive coefficient normalization
scale. It accounts for shared source normalization and the exact
Bernstein product scale; transverse expressions have one additional
power of the shared input scale. This allows magnitude bounds to be
converted to authored coefficients when the denominator is separated
from zero. Normal-Jacobian decisions keep this metadata absent.

Nine predicate tests pass. Independent rational fixtures retain a
nonzero leading tangential term, distinguish an added 1e-12 transverse
component, compare U/V and row/cell queries, recover authored endpoint
coefficients (two for the ordinary derivative, one for the transverse
expression), and refuse insufficient work or a zero anchor. Evidence:
`transverse-derivative-tests.log.gz` and
`transverse-derivative-poles.json.gz` with its completed log.

These exact factors still need source-owned preparation, interval
bounds for the full remaining nets, a separated longitudinal
derivative, and the weighted mean-chord inequality before they can
exclude the collapsed diagonal. They do not yet add whole self-face
certificates or global wall qualification. Retained coverage is 28/31.


## Source-owned rational weighted chord decomposition

The new sealed certificate retains the original immutable surface and pole
orientation. Exact original coefficient predicates prepare the complete nets
in S_u=a lambda+u E and S_v=a u alpha+u² B. Every removed coefficient has
exact Zero sign; the nonzero first-order tangential term alpha is retained.
Coefficient bounds are divided by their positive normalization scale and
original W². Queries refuse a longitudinal lambda interval containing zero.

For a straight UV segment, let L=integral(lambda), T=integral(u),
R=integral(u²), A=integral(u alpha), and F=integral(u E).
Then chord=(a+F/L)(delta_u L+delta_v A)
+(integral(u² B)/R-(F/L)A/R) delta_v R.
Positive weighted means and T²<=R enclose both columns by
M1=a+E_box*[0,u_max]/lambda_box and
M2=B_box-E_box*(alpha_box/lambda_box)*[0,1].
Endpoint tangent frames lie in these same bounds. R=0 implies the entire
segment lies on the collapsed rail and its geometric chord is zero.
This does not assert a unique regular tangent plane at that rail.

The intrinsic exclusion uses the existing interval minor/singular-value bound
and positive diagonal column balancing. Each rational query pays five net
restrictions; exact preparation work has its own reported budget. Complete
coverage checks that differential and normal facts have identical original
source pointers and pole orientation. Insufficient work retains pending pairs.

Three preparation/integration tests pass, including independent polynomial
S=(3u,3u²v,u³) chord and endpoint-frame formulas, reversed poles, retained
rational tangent terms, an exact nonzero transverse perturbation, foreign
source refusal, and bounded incomplete coverage. The narrow rational fixture
rectangle is excluded; its wider rectangle was not certified by these bounds.
The twelve existing chord/coverage regressions also pass. Evidence:
`owned-rational-pole-tests.log.gz` and `rational-chord-tests.log.gz`.

The four original-body audit is running with unchanged 4.74999 mm minimum,
1e-4 squared-sine tolerance, 1024 cells and 8192 spans per end face. No new
whole-face or whole-wall certificate is claimed before its completed report.
Retained previously proven self-face coverage remains 28/31. Distinct-face
wall pairs and global minimum wall thickness remain unqualified. This native
calculation has not been included in a new WASM/UI build.


## Completed rational pole coverage audit

The native four-body audit completed successfully as a process, with all eight
source-owned rational preparations qualified. This is not whole-wall admission.
Both sweep directions have identical coverage counts for each radius order:

| Radius order | Face | Cells | Spans | Rational polar exclusions | Pending |
| --- | --- | ---: | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 861 | 8192 | 235 | 133 |
| 0.5 → 1.25 | 10 | 908 | 8192 | 0 | 16 |
| 1.25 → 0.5 | 0 | 864 | 8192 | 216 | 194 |
| 1.25 → 0.5 | 10 | 937 | 8192 | 0 | 19 |

Exact rational preparation costs 1,536,798–1,666,299 operations per pole,
separate from normal preparation and coverage restrictions. No pole face
receives a complete self-wall certificate. Original thresholds and geometry
are unchanged. The unresolved entry pair reaches U≈1e-12 with V endpoints
near 0 and 0.245; refining each endpoint rectangle alone does not shrink
their convex union. The next proof must handle these separated angular
endpoints and coupled near-pole scales, rather than merely raise work limits.
Exit pairs remain at finite U widths. Evidence: `rational-self-wall.json.gz`
and its completed log. Previously qualified same-face coverage stays 28/31.

A fourth independent test uses direct rational quotient derivatives, nonzero
first-order tangential V derivatives, both reflected ends, and translated,
scaled knot domains. Its endpoint frames lie in the prepared interval matrix;
all source points, weights and knots remain unchanged. Four tests pass.


## Endpoint-normal weighted matrix exclusion

A pair-specific check now uses the original factored normal on each endpoint
rectangle. For chord=M z, Cauchy--Schwarz gives |n.chord|²<=||n^T M||²|z|².
The same separated minor sum and trace lower-bound sigma_min(M)², so the
ratio bounds cosine against every enclosed endpoint normal. A fixed positive
column scaling preserves the inequality. Missing matrix rank or nonzero
normal lower bounds refuse exclusion. This pair-specific proof is counted as
a direction exclusion; it never creates an intrinsic certificate for the union.
Five mean-matrix net restrictions and up to two normal restrictions are charged.
The earlier intrinsic query is still separate, so this currently duplicates
five restrictions when that query fails. Preparation budgets remain unchanged.

Thirteen chord/coverage tests and four rational preparation/integration tests
pass. A matrix with large tangent-plane variation still excludes its plane
normal; actual aligned chords, zero normals and deficient matrices refuse.
The rational fixture at U~1e-12 and V separation 0.1 is excluded, including
reflected exit. V separation 0.245 remains unqualified because independent
coefficient bounds include a singular mean matrix. The latter refusal is a
regression, not relaxed or removed to obtain a passing test.

The completed four-body audit retains 4.74999 mm, squared-sine 1e-4,
1024 cells and 8192 restrictions per pole. Both directions again match:

| Radius order | Face | Cells | Spans | Direction exclusions | Pending |
| --- | --- | ---: | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 513 | 8192 | 191 | 57 |
| 0.5 → 1.25 | 10 | 503 | 8192 | 195 | 17 |
| 1.25 → 0.5 | 0 | 512 | 8192 | 204 | 40 |
| 1.25 → 0.5 | 10 | 500 | 8192 | 180 | 14 |

Pending counts compare different adaptive partitions and do not measure a
proved fraction or alone establish faster convergence. None of these faces
is completely certified; retained same-face coverage stays 28/31. Evidence:
`endpoint-self-wall.json.gz`, its log and both endpoint test logs. A tighter
correlated rational quotient/mean bound remains needed near the collapsed rail.
Global wall qualification and WASM/UI packaging remain open.


## Correlated rational coefficient quotient bounds

The source-owned certificate now builds W² in its exact tensor Bernstein
product degrees using original weights and outward interval arithmetic.
Numerator nets are degree-elevated without changing their original polynomial.
Every binomial integer product is exactly representable for admitted degrees
<=8. The homogeneous net packs the elevated numerator and common W²
coefficient together. Restriction uses the same original blossom operations.

For positive restricted denominator coefficients D_i, N/D is a convex
combination of N_i/D_i, with weights B_i D_i/D. Each coefficient quotient
therefore gives a valid rational hull that retains the shared denominator.
Nonpositive coefficient denominators refuse the query. The original weight
restriction and four rational nets retain the five-restriction query charge.
This tightens lambda, alpha, E and B separately; their integrated mean
correlations remain bounded by the earlier conservative inequality.

Four rational pole tests, six normal-alignment tests and thirteen chord tests
pass (23 total). Independent rational quotient formulas still enclose both
reflected/scaled-domain frames. A shared-denominator constant ratio stays
near one despite weights ranging 2..10; nonconstant point queries contain
1/(2+8u), and a zero denominator refuses. The earlier angular separation
0.245 fixture now qualifies with the improved bound at U~1e-12. The unchanged
analytic quotient oracle, source ownership and budget tests accompany this
change in expected admission. Evidence: the three `correlated-*-tests.log.gz`
files. A fresh original four-body audit has been dispatched with unchanged
1024/8192 limits; no complete pole or whole-wall claim is made before its result.


### Completed correlated quotient audit

The original four-body run completed, again with all exact pole preparations
qualified and no fully qualified pole face. Both directions match:

| Radius order | Face | Cells | Spans | Direction exclusions | Polar exclusions | Pending |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 524 | 8192 | 154 | 40 | 124 |
| 0.5 → 1.25 | 10 | 494 | 8192 | 155 | 0 | 9 |
| 1.25 → 0.5 | 0 | 527 | 8192 | 152 | 46 | 109 |
| 1.25 → 0.5 | 10 | 497 | 8192 | 149 | 0 | 13 |

Uncertain entry pairs now approach U~1e-11 (forward radii) or U~1e-10
(reverse radii), with V near 0 and 0.393202419. Exit uncertainty is at finite
U widths. This is a different adaptive partition: pending counts and exclusion
counts are not a certified fraction or proof of monotonic convergence versus
previous runs. Whole self-face coverage stays 28/31 and global wall thickness
is still unqualified. Evidence: `correlated-self-wall.json.gz` and its completed
log. The next proof must preserve additional coupling in E, alpha and lambda
or bound the original transverse endpoint difference directly, while retaining
all exact leading terms and both closed sides of the collapsed diagonal.


## Confined path in longitudinal coordinate and V

A second source-owned chord representation uses X=S_anchor and V. Separated
lambda=S_anchor,u proves U is uniquely determined by (X,V) wherever the inverse
path is confined to the original chart. Exact pole factors give a constant
collapsed rail position at U=0. In these coordinates:

- S_X = a + u E/lambda;
- S_V at fixed X = u² (B - E alpha/lambda).

Preparation preserves the Schur-complement correlation in a single polynomial
quotient: (B_num lambda_num - E_num alpha_num)/(W² lambda_num).
Tensor Bernstein products and degree elevation use outward arithmetic. Degree
products reach 32, so binomial factors are multiplied as intervals instead of
casting a potentially inexact integer product. Signed rational coefficient
quotients are admitted only when all restricted denominator coefficients share
the same separated sign; their normalized convex weights then remain positive.

The path takes an X leg at the first V, then a V leg at the second X. Its
chord is M1 delta_X + M2 delta_V integral(u²), with columns enclosed by the
above frames. Both original endpoint normals are perpendicular to these frames.
If the weighted vector vanishes, the geometric chord vanishes. A coordinate
matrix is tried before the existing UV mean matrix; each attempted matrix
pays five net restrictions and each queried endpoint normal pays one. A failed
coordinate path does not silently receive a free fallback query.

Confinement is checked on the original normalized rectangle enlarged from the
pole to min(1,4*u_max). The signed minimum lambda over this enlarged rectangle
must give an available longitudinal range at least as large as the endpoint
range. The endpoint range uses lambda over the entire pole-to-endpoint prefix,
not merely the endpoint rectangle. An earlier experimental audit used the
latter insufficient bound and is intentionally excluded from qualification
(`/tmp/annular-coordinate-self-wall.json` is not accepted evidence). A fresh
corrected audit is running into `annular-coordinate-prefix-self-wall.json`.

Six rational-path tests, six normal tests and thirteen chord tests pass
(25 total). Independent polynomial chord formulas verify both legs; direct
rational quotient derivatives verify the correlated frame on both reflected,
translated/scaled charts. Nonmonotone and unconfined charts refuse. A dedicated
prefix regression has nearly equal local endpoint derivatives around U=.99
but incompatible X ranges on the two V sides; the corrected prefix bound
refuses its invalid inverse path. Signed-negative denominators preserve valid
quotient bounds; mixed coefficient signs refuse. Evidence: the three
`coordinate-*-tests.log.gz` files. Retained whole-face coverage remains 28/31;
no complete wall or WASM/UI integration is claimed here.


### Completed corrected prefix audit

The original-body audit with the corrected pole-to-endpoint prefix completed
against the implementation in commit 5c83a87a. No pole face is fully certified.
Both directions have matching counts at the unchanged 1024/8192 limits:

| Radius order | Face | Cells | Spans | Direction exclusions | Pending |
| --- | --- | ---: | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 392 | 8192 | 109 | 44 |
| 0.5 → 1.25 | 10 | 382 | 8192 | 123 | 12 |
| 1.25 → 0.5 | 0 | 392 | 8192 | 101 | 42 |
| 1.25 → 0.5 | 10 | 384 | 8192 | 122 | 12 |

Uncertain entry rectangles now have U widths ~1e-4 and V around 0 and 0.396.
Exit uncertainty remains at finite U. These partitions do not establish a
certified surface fraction. Whole self-face coverage stays 28/31. Accepted
evidence: `coordinate-prefix-self-wall.json.gz` and its completed log. The
older audit lacking the prefix bound remains excluded from qualification.

## Simultaneous coefficient restriction and classified pair evidence

Normalized rational quotient nets now restrict all coefficients through two
closed de Casteljau splits per axis. Point and full-domain cases retain their
closed endpoints. Each interval blend is intersected with the parent coefficient
hull, preserving the exact convex combination. The original knot-span blossom
helper stays the route for generic position/Jacobian hulls; rational quotient
queries use the simultaneous unit-domain restriction. Independent rational
quotient, scaled/reflected frame, prefix-confinement and true-chord refusals
still pass: 6 path tests, 6 normal tests, and 13 chord tests.

Pair reports separately track whether the coordinate path was attempted,
available, and actually excluded a pair. Coverage and the native audit expose
attempts, confinement/factor refusals and successful coordinate-path exclusions;
an absent budget is not counted as a path refusal. Fixture tests verify the
successful mode and that insufficient coverage work records zero attempts.
Per-face elapsed milliseconds include ordinary qualification, exact normal and
differential preparation, and adaptive coverage. This is not UI latency.

An ignored native microbenchmark compares 50 identical restrictions of a
16x8 polynomial net. It validates a constant shared-denominator quotient and
independent 1/(1+u+v) values. In this debug run, the existing blossom restriction
took 3300.411 ms and simultaneous splitting 364.877 ms (9.045x ratio). This is
one debug microbenchmark under unmanaged host load, not release speed or
end-to-end CAD latency. Evidence: `split-restriction-benchmark.log.gz` and the
three `split-*-tests.log.gz` files (25 regressions plus the benchmark passed).
A fresh four-body run with classified evidence and splitting is active;
whole-wall qualification and WASM/UI integration remain open.


### Completed simultaneous restriction audit

The original four-body run with simultaneous restriction and classified path
reports completed. It reproduces the corrected-prefix partitions, pending
counts and uncertainty rectangles above; no pole face is fully certified.
Both sweep directions match these classified counts:

| Radius order | Face | Coordinate exclusions | Coordinate attempts | Path unavailable |
| --- | --- | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 68 | 354 | 3 |
| 0.5 → 1.25 | 10 | 0 | 381 | 381 |
| 1.25 → 0.5 | 0 | 53 | 347 | 3 |
| 1.25 → 0.5 | 10 | 0 | 384 | 383 |

These counters distinguish actual coordinate-path proofs from UV-matrix or
Cartesian secant exclusions. Entry paths contribute certificates for individual
pairs; exit paths fail the pole-based confinement or separated factor checks
almost everywhere. This identifies local inverse-coordinate charts as the
next exit-face proof direction; raising the same global prefix budget does
not resolve those failures. Entry uncertainty still needs additional coupled
bounds around V separation ~0.396.

Per-face elapsed time across all eight cases was 623.740–1868.949 ms,
including ordinary qualification, both native preparations and adaptive
coverage. This completed run has no paired end-to-end timing baseline;
the debug microbenchmark ratio must not be substituted for its speedup.
Evidence: `split-self-wall.json.gz` and the completed log. Original model
serialization remains unchanged in every case. Retained whole self-face
coverage stays 28/31. Distinct-face wall checks, complete minimum wall
thickness, general arbitrary-body fillets and WASM/UI packaging remain open.


## Local inverse-coordinate path confinement

The certificate also prepares original anchor positions relative to the exact
pole origin, as a positive-weight rational net. Local path queries enlarge U
on both sides inside the original chart, prove separated signed lambda there,
and compare the whole endpoint X range with boundary X ranges over every V.
For increasing X, max(X_left)<=min(X_endpoints) and
min(X_right)>=max(X_endpoints) prove a unique confined inverse path by the
intermediate value theorem and monotonicity. Decreasing X uses the signed
orientation. The constant pole rail and monotonicity justify clipping only
outward rounding below signed zero when the left boundary is the exact pole.
No such clipping is used at a finite local boundary.

Six restrictions (lambda, endpoint position, two boundary positions, and two
correlated frames) are charged for each local attempt, separate from the five
prefix-path restrictions. Pair evidence records local attempts, availability,
and actual exclusions. Failed paths retain unresolved coverage. Seven rational
path tests and thirteen chord tests pass (20 focused tests). Direct rational
quotient formulas cover increasing and decreasing local charts in a globally
nonmonotone original surface, plus translated/reflected frames. The invalid
inverse path at U=.99 remains refused. A pair regression spends exactly 12
units for prefix refusal, local preparation and one endpoint normal; an
11-unit budget has no exclusion and keeps its actual charged work.

The four-body run completed with original thresholds, geometry and 1024/8192
limits; both directions match:

| Radius order | Face | Cells | Local exclusions | Local attempts | Local unavailable | Pending |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 309 | 25 | 239 | 3 | 49 |
| 0.5 → 1.25 | 10 | 294 | 0 | 292 | 184 | 12 |
| 1.25 → 0.5 | 0 | 309 | 16 | 238 | 3 | 44 |
| 1.25 → 0.5 | 10 | 293 | 1 | 293 | 181 | 13 |

No pole face is fully certified. Retained whole self-face coverage remains
28/31; pending counts belong to different partitions and are not certified
fractions. Evidence: `local-self-wall.json.gz`, its completed log, and both
local test logs. Current angular coverage still compares unfactored normals,
which can vanish on the pole even when sealed factored limits are separated;
reusing the exact factored limits is the next angular qualification step.


## Exact factored normal angular compatibility

Necessary endpoint-angle compatibility now reuses the sealed original normal
factor on both endpoint rectangles, paying two source restrictions. Previously
this stage used unfactored normals that vanish on the pole and lose directional
information. The pure bounded-normal comparator is shared with the ordinary
source-surface path; it performs no geometry queries and does not itself prove
ownership or positional agreement. Coverage establishes source ownership before
supplying its two exact factor bounds. All one-sided limits remain covered;
this does not assign a unique regular tangent plane to the collapsed rail.

Seven normal tests and thirteen chord tests pass (20 focused regressions).
An independent polynomial S=(3u,3u²v,u³) has factored normal (-u²,0,1):
its pole and U=1 limits have squared sine 0.5, while unfactored original bounds
remain unresolved at the pole. Anti-parallel limits remain angular-compatible,
and a zero normal refuses. Existing source identity, work-limit, genuine
short-normal-chord and whole-chart subdivision regressions remain passing.

The four-body 1024-cell / 8192-restriction run completed. Both directions match:

| Radius order | Face | Cells | Factored angular exclusions | Pending |
| --- | --- | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 299 | 47 | 9 |
| 0.5 → 1.25 | 10 | 292 | 62 | 12 |
| 1.25 → 0.5 | 0 | 287 | 42 | 9 |
| 1.25 → 0.5 | 10 | 294 | 62 | 10 |

Remaining entry pairs are now at finite U widths (roughly U=.25–.5), rather
than stalled U~1e-12 pole limits. This changed the next action: a larger bounded
qualification run was justified for those remaining finite regions. It does
not establish a certified fraction or complete face admission.

At 4096 cells / 32768 restrictions, pending entry counts are 4/6 and exit
9/10 for the two radius orders. At 12500 cells / 100000 restrictions:

| Radius order | Face | Cells | Factored angular exclusions | Pending |
| --- | --- | ---: | ---: | ---: |
| 0.5 → 1.25 | 0 | 3458 | 746 | 6 |
| 0.5 → 1.25 | 10 | 3488 | 738 | 10 |
| 1.25 → 0.5 | 0 | 3346 | 684 | 8 |
| 1.25 → 0.5 | 10 | 3514 | 729 | 14 |

All original serialization and thresholds are unchanged; no pole face gets a
complete certificate at any of these limits. Pending counts refer to different
partitions, not monotonic coverage percentages. Retained whole self-face
coverage stays 28/31. The matrix stages currently query the same endpoint
normal facts repeatedly before the angular stage; sharing those restrictions
and ordering inexpensive exclusions earlier is an available bounded-work
improvement. Evidence: the factored-angular test logs and all three completed
`factored-angular*-self-wall.json.gz` reports with their logs.

### Floating candidate probe, not certified thickness evidence

`variable-annular-pole-chord-probe` samples the unchanged original entry face
in all four bodies. U=.01/.001/.0001, first V=.00018, and second V starts in
.35–.45 with bounded local refinement. A floating bisection in [.5U,2U]
approximately matches the original longitudinal anchor coordinate, then ranks
chords by their smaller squared cosine to the two evaluated normals. Every
output includes parameters, original points, normals, distance and coordinate
residual; original model serialization is checked unchanged.

The best sampled/refined minima are around 0.1186, far below the admission
threshold 0.9999. This search found no aligned candidate in its restricted
window; it does not prove absence of thin chords elsewhere, nor certify its
floating angles or distances. Candidate data are deliberately not promoted to
an exact witness or a wall certificate. Evidence: `pole-chord-candidates.json.gz`
and its completed log. Full minimum wall, general fillets, STEP/part acceptance,
P0 and new WASM/UI packaging remain open.

## Endpoint normal reuse

Commit 7e4354a6 reuses each endpoint bound between the three pole matrix
modes. All seven rational-pole and thirteen chord/coverage tests passed.
The unchanged 1024-cell / 8192-restriction audit retained pending pairs on
both pole faces in every prototype: forward radii 8/13, reverse radii 7/11.
Both spatial directions matched. Whole same-face qualification stays 28/31;
distinct-face wall thickness remains unqualified. Archived normal-cache
JSON/logs describe this first change only.

The next source-owned cache binds both normal boxes to the immutable original
surface, pole orientation and exact pair rectangles. Angular and Cartesian
secant stages share it, charging the two restrictions once. The thirteen
chord/coverage tests include cached exclusion with zero additional budget,
foreign-source and changed-pair rejection, and incomplete-coverage refusal.
The separate pair-cache prototype audit is still running; no qualification
claim follows from these unit tests.

## Shared pair cache and early angular exclusion

The source-owned pair normal cache now also feeds all three pole matrix
modes. Its source/pair identity is checked before budget refusal. Normal
queries are paid once upstream; cached projections consume no new normal
restriction. The independent local inverse-chart fixture still refuses an
insufficient matrix budget, and altered pairs are rejected even at zero
budget. Seven pole and thirteen chord/coverage tests passed.

Coverage now attempts the necessary angular condition before the expensive
pole matrices. This changes traversal partitions under a fixed limit; cell
and pending counts are not percentages of qualified area. The unchanged
1024/8192 prototype audit is running as annular-all-cache-self-wall. Whole
wall qualification remains unproven until that audit and the remaining
same-face/distinct-face obligations succeed.

## Angular/secant cache audit completed

The pair-cache audit completed for all four unchanged original prototypes at
1024 cells / 8192 restrictions. Both spatial directions matched. Forward
radii retain 9/14 pending pairs on faces 0/10; reverse radii retain 9/12.
No pole face qualified. Archives annular-pair-cache-self-wall JSON/logs
belong to df2fb62c, before early angular ordering. Preparation work remains
separate; no whole-wall minimum or improved area percentage is asserted.

## Early original Cartesian exclusion

Original position distance and cached endpoint secant alignment are now
checked immediately after angular separation, before the three pole mean
matrices. Their bounds, thresholds and accounting are unchanged. This is
only a change in proof scheduling. Thirteen chord/coverage tests passed,
including the rational-cylinder short-normal-chord counterexample and
insufficient-budget rejection. The unchanged 1024/8192 audit runs as
annular-early-secant-self-wall; its result is pending.

## Full cache / early angle audit completed

The afe440f0 audit finished at unchanged 1024/8192 limits. Both spatial
directions match: forward radii retain 8/12 pending pairs on faces 0/10,
reverse radii 8/13. No pole face is certified; whole same-face coverage
remains 28/31. All-cache JSON/log archives record original geometry identity
and classified proof attempts. Counts reflect changed traversal, not area.

Factored normal restriction now intersects every interval de Casteljau blend
with its parent coefficient hull and its parameter with the closed unit
interval. These are mathematically valid intersections: the exact blend is
convex. Four normal tests passed, including independent original cubic
normals and constant high-degree coefficient hulls at closed endpoints.
This improvement has not yet been qualified on the prototype bodies.

## Early Cartesian audit completed

The 372c3ed8 audit completed on all four prototypes at 1024 cells / 8192
restrictions. Forward radii retain 6/17 pending pairs on faces 0/10; reverse
radii retain 7/15, identical for both spatial directions. No pole face
qualified. The first unresolved forward entry pair is
U[.125,.25] V[.25,.5] versus U[.25,.5] V[.5,.75]. Its exit pair is
U[0,.015625] V[.25,.28125] versus U[.0625,.09375] V[.34375,.375].
These finite rectangles require stronger path/frame bounds; traversal
counts alone do not establish either safety or a thin witness.

The thirteen chord tests after convex normal restriction also passed.
The convex prototype audit is still running; its first completed cases
have exactly the same classified counts as the early Cartesian audit.
No whole-wall safety or qualification improvement is inferred.

## Convex restriction audit and tighter inverse charts

The convex-normal audit completed with exactly the same classified pending
counts as early Cartesian exclusion: 6/17 forward radii and 7/15 reverse
radii, matching both spatial directions. No pole face qualified.

Local inverse paths now additionally try margins of one and one quarter
parameter width after the original three-width margin. Each attempt still
pays six restrictions and independently proves signed monotonicity and
confinement of the complete inverse path. Endpoint normals retain the same
source-owned cache. The rational analytic regression at U[.45,.55],
V[0,.01] refuses the broad nonmonotone extension and accepts the tighter
chart, enclosing independently calculated derivatives. Seven pole and
thirteen chord tests pass. The unchanged 1024/8192 prototype audit is
running as annular-tight-path-self-wall; qualification remains unproven.

## Executed local restriction accounting

Local inverse attempts now return their executed restriction count. A
separated-lambda refusal executes one restriction; successful confinement
and both frame bounds execute six. The caller reserves six before entry
and charges the actual count afterward. No geometric predicate changed.
Seven pole and thirteen chord tests passed, including exact one/six counts,
independent rational frames, original source/pair identity and insufficient
budget refusal. The 1024/8192 audit runs as annular-local-accounting-self-wall.

The preceding tight-path audit is still running. Its forward-radius cases
already exclude 8/24 pairs via local inverse charts on faces 0/10; the
corresponding early-secant audit excluded zero this way. This establishes
that the added chart proof can exclude new pairs, not whole-face coverage
or a percentage gain. Those partial cases retain 8/13 pending pairs.

## Tight path audit complete; all pair matrix accounting

The 24ad9f5d audit completed at unchanged 1024/8192 limits: forward radii
retain 8/13 pending pairs, reverse radii 9/15, identical in both spatial
directions. Local inverse paths exclude 8/24 pairs for forward and 29/23
for reverse radii on faces 0/10. No pole face qualified. These counts
describe proofs found under a changed traversal, not area percentages.

Pair matrix accounting now also reports executed prefix and UV weighted
restrictions. Tests verify early failures at three and two restrictions,
respectively; successful paths retain all original restrictions. Each mode
still reserves its worst-case cost before execution. The independent local
fixture now spends three prefix + six local + one endpoint-normal
restriction: ten succeeds, nine refuses. With both endpoint bounds paid
upstream, nine additional restrictions succeed and eight refuse. Seven
pole and thirteen chord tests passed. Original predicates and geometric
thresholds are unchanged; whole-face/global wall safety remains unproven.

## Local accounting audit complete

The 4d5e144c audit completed on all four unchanged prototypes at 1024/8192.
Forward radii retain 7/11 pending pairs on faces 0/10, with 8/25 local path
exclusions. Reverse radii retain 11/11, with 29/26 local path exclusions.
Both spatial directions match. No pole face qualified; whole same-face
coverage stays 28/31. JSON/log archives retain full uncertainty rectangles
and classified attempts. The all-accounting audit for 191a0add is running
separately with the same limits and thresholds.

## Endpoint matrix Gram eigenvalue bound

Endpoint projection now bounds the largest eigenvalue of MᵀM by the
minimum of its original trace upper bound and an outward Gershgorin bound.
For every enclosed M, lambda_min >= det(MᵀM)/lambda_max_upper; the same
original separated minor sum and normal norm lower bound are retained.
No sampled endpoint normal or path is admitted.

An independent constant-matrix chord oracle checks 24 nonzero directions.
The upper cosine-squared estimate is below .32 where the previous
trace-only estimate exceeds .60. Fourteen chord/coverage and seven pole
tests passed, including rank/normal refusals, short aligned normal chords
and bounded-work refusal. Prototype qualification for this new bound has
not been run; the preceding all-accounting audit is still live. Whole
same-face coverage remains 28/31 and global wall safety is unproven.

## All-accounting audit complete and dominating Gram eigenvalue

The 191a0add audit completed at unchanged 1024/8192 limits: forward radii
retain 7/10 pending pairs on faces 0/10; reverse radii retain 11/13.
Both spatial directions match. No pole face qualified. Archives retain
the exact uncertainty rectangles and original serialization invariant.

Endpoint projection now also evaluates the closed-form largest eigenvalue
of a dominating symmetric 2x2 Gram matrix: upper diagonal norms and the
absolute upper off-diagonal dot product. Increasing either diagonal or
the absolute off-diagonal increases its largest eigenvalue. All operations
are outward; the square-root upper endpoint is rounded upward. The minimum
of this, Gershgorin and trace bounds still covers every original mean
matrix. The new unequal-column independent chord fixture gives a cosine
upper below .31 (Gershgorin alone exceeds .328). Fifteen chord/coverage
and seven pole tests passed. This closed-form change has not yet been
audited on the prototype bodies. The preceding Gershgorin prototype audit
is still running. Whole wall safety remains unproven.

## Gershgorin prototype audit completed

The b4c8cf53 audit completed on all four unchanged original prototypes at
1024 cells / 8192 restrictions, minimum 4.74999 mm and sin² 1e-4.
Forward radii retain 6/8 pending pairs on faces 0/10; reverse radii retain
10/12. Both spatial directions match; no pole face qualified.

The forward entry uncertainty is U[.4375,.5] V[.375,.5] versus
U[.375,.5] V[.25,.375]; its exit uncertainty is
U[.125,.25] V[.375,.5] versus U[0,.25] V[0,.25].
Reverse entry is U[.3125,.375] V[.25,.3125] versus
U[.375,.4375] V[.125,.25]; reverse exit is
U[0,.0625] V[.375,.4375] versus U[.0625,.125] V[.3125,.375].
These are finite unresolved regions, not certified defects or percentages
of unsafe area. Whole same-face coverage remains 28/31; distinct-face
wall coverage and global minimum thickness remain unqualified.

The closed-form prototype audit for 182d25e2 is running separately as
annular-gram-closed-form-self-wall with unchanged limits and thresholds.

## Direct lower Gram eigenvalue

The closed-form upper-eigenvalue prototype audit completed at 1024/8192.
Its JSON/log archives retain every case and original identity. No pole
face qualified.

Endpoint projection now independently lower-bounds the least eigenvalue
using lower column norm squares and the absolute upper column dot product.
The least eigenvalue increases with either diagonal and decreases with
absolute off-diagonal. Outward arithmetic and an upward-rounded square
root give a valid lower bound. The stronger of this and the separated
minor / largest-eigenvalue bound is used. Zero lower bounds refuse.

An independent interval-matrix oracle checks 72 nonzero chord directions
across nine actual enclosed matrices. Its cosine upper is below .13,
where the minor route alone exceeds .18. An actual aligned chord still
returns an unexcluded bound of one. Sixteen chord/coverage and seven
pole tests passed. The unchanged prototype audit runs as
annular-gram-lower-self-wall; whole-face/global wall safety is unproven.

## Complete unresolved endpoint obligations

Coverage reports now preserve every remaining closed endpoint-pair
rectangle in original coordinates as unresolved_pairs; the audit JSON
exports unresolvedPairs. The previous uncertain field retains the last
pair for compatibility. A budget or resolution refusal returns the full
remaining stack; a qualified certificate has no unresolved obligations.
These records are unproven checks, never claimed defects.

Sixteen chord/coverage tests passed, including complete-empty and
incomplete bounded-domain obligations. The prototype example compiles.
The direct-lower audit and its deeper 12500-cell / 100000-restriction
comparison were launched before this reporting change and do not include
the new list field. They retain their original single uncertainty field.
Both use 4.74999 mm and sin² 1e-4. No qualification is claimed while the
deep audit is running.

## Direct lower prototype baseline completed

The b30b5def 1024/8192 audit completed on all four original prototypes.
Forward radii retain 6/12 pending pairs, reverse radii 7/13, matching both
spatial directions; no pole face qualified at this limit. Archives contain
original identity checks and uncertainty rectangles.

The deeper 12500/100000 audit is confirmed running. Its first forward
radius / positive-direction entry face has a complete certificate after
2721 cells and 70628 restrictions with zero remaining pairs. This partial
progress is not a qualification of every entry face, the exit face, the
middle face, distinct-face wall pairs or the global wall minimum. The
final audit JSON and all four original-serialization checks remain pending.

## Completed-case report checkpoints

The prototype example now atomically replaces its JSON report after each
complete case, after its original model serialization comparison. Explicit
completeAudit / completedCases / expectedCases fields distinguish a
partial run from all four complete cases. The temporary sibling is renamed
only after successful full serialization/write. A short selected-face run
completed all four cases, emitted completeAudit=true and counts=4, and
left no temporary file. It is reporting smoke evidence, not pole coverage
or application crash-recovery qualification. Archives retain that scope.

The existing deep audit predates checkpoint reporting and remains live.
Its first positive-direction / forward-radius case has a full entry face
certificate (2721 cells / 70628 restrictions) and seven remaining exit
face obligations after 4245 cells / 100000 restrictions. The remaining
three prototype cases are not yet completed. No global wall safety follows.

## Proven local margin classifications

A pair exclusion now records the margin of its actually successful local
inverse path. Coverage aggregates local_margin_exclusions for margins
[3,1,.25], and audit JSON exports localMarginExclusions. Non-local and
unexcluded paths have no winning margin. These counts classify accepted
proofs; they do not measure area or assert a global wall minimum. The
independent rational path regression confirms its original three-width
proof classification. Seven pole / sixteen chord tests and example
compilation passed. The ordering and admission predicates are unchanged.

The deep audit still running predates this reporting change. Both forward
radius entry faces now have complete original-chart certificates with
2721 cells / 70628 restrictions and zero pending pairs, for spatial
directions ±1. The positive-direction exit still has seven pending pairs
at 100000 restrictions. Negative exit and reverse-radius cases are not
yet completed; no all-prototype or global wall qualification is claimed.

## Complete margin classification and tight-first scheduling

The 0711a50f 1024/8192 audit completed with explicit completeAudit=true
and four original-identity cases. Forward radii have winning-margin counts
[0,1,30] / [0,6,20] on faces 0/10; reverse radii [0,0,35] / [0,6,22].
Both spatial directions match. Classified sums equal local exclusions and
every unresolvedPairs list equals pending, with its last pair matching
uncertain. Full JSON/log archives retain these checks' source data.

For budgets >=25, pair proofs now try quarter-width, one-width, then
three-width local charts after the prefix path. Five prefix restrictions,
three six-restriction local paths and two normal restrictions fit within
25, so all prior alternatives remain available. Smaller budgets retain
the previous broad-first order. The independent rational fixture still
qualifies at ten restrictions and refuses at nine; higher budgets also
retain its original confined proof. Seven pole and sixteen chord tests
passed. The fixed 1024/8192 tight-first audit is running separately; no
whole-wall or performance improvement is claimed before completion.

## Direct Gram lower bound for ordinary intrinsic charts

The direct least-eigenvalue bound now also strengthens ordinary intrinsic
chord qualification: normal projection variation / least-eigenvalue
lower bound is combined with the existing separated-minor estimates.
The same original Jacobian hull covers every mean and endpoint derivative.
This does not require a pole factor or an inverse-coordinate path.

Gram entries are normalized before their discriminant is squared and
rescaled outward afterward. Regression includes 1e200 squared column norms
and an anisotropic constant matrix with a 1e100 column, preserving finite
bounds. Sixteen chord/coverage and seven pole tests passed, including
independent curved chord oracles and aligned-short-chord refusal.
The 12500/100000 ordinary middle-face audit is running as
annular-intrinsic-gram-middle-self-wall. Whole-wall safety remains unproven.

The preceding tight-first 1024/8192 prototype audit completed and is
archived separately. Its JSON includes all four unchanged models, every
unresolved pair and the proven local margin classifications.

## Scalar interval product and full NURBS regression

A one-second live debug qualification sample identified iterator overhead
in finite checking and four-product extrema. Interval::mul now uses scalar
checks and the exact same left-to-right min/max order, including initial
infinities, outward rounding and numeric refusals. The differential edge
regression covers signed zeros, subnormals, huge bounds and invalid inputs.
Paired debug 100000-product runs: old 19331/20005/20352 µs, scalar
5651/6296/6856 µs. This is not a UI/release or whole-qualification speedup.
The complete NURBS suite passed: 708 passed, zero failures, two ignored
benchmarks; the product benchmark was also run explicitly.

## Direct lower deep audit completed

The b30b5def audit completed all four unchanged originals at 12500 cells
and 100000 restrictions, before later intrinsic/scheduling/report changes.
Both forward-radius entry faces qualify completely: 2721 cells, 70628
restrictions, zero pending. Both forward exits retain seven pending pairs
after 4245 cells / 100000 restrictions. Both reverse entries retain five
after 3941 / 100000; both reverse exits retain five after 4099 / 100000.
All four original serialization comparisons passed. Complete JSON/logs
are archived. Middle face, distinct-face wall pairs and the global minimum
remain separate open obligations. Current intrinsic middle audit is live.

## Original continuous mean matrix pair projection

Ordinary rectangle qualification now retains an opaque mean-matrix fact
only after every intersecting original derivative span and continuity
check succeeds. The fact owns its immutable surface and rectangle. Pair
endpoints must lie inside that rectangle; normals are bounded from the
same original source, or a source/pair-checked factored normal cache.
Endpoint matrix projection is checked before expensive inverse paths.
Missing derivative work yields no fact; missing normal work never excludes.
This extends projection beyond pole charts without inferring from samples.

The independent planar polynomial S=(u,10v+10u²,0) has large tangent
variation that refuses the ordinary intrinsic estimate, while complete
original mean projection excludes the pair with one paid normal span.
Zero budget refuses; foreign rectangles and incomplete derivative coverage
refuse. Seventeen chord/coverage and seven pole tests passed, including
the genuine short normal-chord counterexample.

The preceding ordinary Gram middle-face audit completed all four original
models at 12500 cells / 100000 restrictions: 19 pending forward and 11
reverse, matching spatial directions. None qualified. Its complete JSON
and every unresolved pair are archived. The new original-mean middle
audit runs at 20000 cells / 100000 restrictions; no wall safety claim
follows until its remaining obligations are closed.

## Mean matrix algebraic precondition and completed pole audit

Projection now checks its matrix lower bound before requesting endpoint
normal restrictions. A nonpositive bound means this estimate cannot prove
exclusion; it does not declare singular geometry. Source and pair cache
validation still occurs first. The collapsed polynomial regression refuses
without spending normal restrictions. Seventeen chord/coverage and seven
pole tests pass; diff whitespace validation passes.

The complete 7f9b72b7 pole audit preserves all four original models. Forward
entry faces qualify at 2643 cells / 60966 restrictions. Forward exits retain
four pairs at 4448 / 100000; reverse entries retain five at 4211 / 100000,
and reverse exits retain seven at 4387 / 100000. Both spatial directions
match. Complete JSON and logs are archived with all remaining rectangles.
This predates the ordinary mean-matrix and algebraic precondition changes.
Distinct-face wall pairs and global wall thickness remain unqualified.

## Paid original endpoint normal reuse

Ordinary angular inspection now seals its two complete original normal
enclosures with private source and pair ownership. Secant and mean-matrix
projection reuse these same boxes without extra restrictions. No cache is
created for an incomplete pair. Source and exact endpoint rectangles are
validated before a zero-budget refusal; copied foreign geometry is rejected.
Already-paid boxes can be projected even when the remaining query budget
is zero. Independent curved graph and planar polynomial regressions verify
zero additional restrictions, foreign-source and wrong-pair refusals, and
missing-work refusal. This changes work accounting, not geometric tolerance.

## Closed endpoint exchange coverage

The original ced1acad mean-matrix audit completed all four unchanged models:
forward middle faces retain 15 pairs after 14565 cells / 100000 restrictions;
reverse retain 15 after 14633 / 100000. Full JSON/logs are archived. These
counts describe budget exhaustion and do not establish defects.

The subdivision now treats a diagonal rectangle A×A as LL, LR and RR, with
LR also representing RL by endpoint exchange. All splits are closed. Chord
length and squared alignment with either endpoint normal are invariant under
reversal, so coverage of the representative proves the exchanged pair too.
The independent product-coverage regression checks both parameter axes,
interior points, shared split boundaries and diagonal endpoints. The actual
short normal chord counterexample remains a refusal test. JSON schema 2
explicitly marks unresolved pairs as representatives including exchange.
No asymmetric distinct-face operation uses this reduction.

## Complete original rectangle cache

An in-call cache binds one immutable original surface and one squared-sine
tolerance. Exact parameter endpoint bits key complete ordinary derivative
and continuity reports. Only complete reports enter the cache; complete
refusals may be reused but never promoted to certificates. Reuse consumes
zero new derivative restrictions, preserving the original outward bounds.
There is no cache across surfaces, edits or qualification calls. Reports
count completeRectangleCacheHits. Twenty focused tests pass, including
repeated partial two-span refusals, complete reuse with no new query budget,
independent closed-product budget coverage and the short normal chord. The
calculation example passes cargo check. Geometry tolerances remain unchanged.

The complete 512-cell / 4096-restriction middle-face audit preserves all
four original serializations. Forward radii use 2071 restrictions, reuse
311 complete rectangles, and make 89 mean-matrix exclusions; 15 pending
representatives remain. Reverse radii use 2072 restrictions, reuse 318
rectangles, and make 99 mean-matrix exclusions; 13 remain. Both spatial
directions match. These finite-budget results do not qualify any additional
face. Full schema-2 JSON and logs are archived.

## Original normal rectangles reused across pairs

An in-call cache binds one immutable original surface. It retains only
complete original normal enclosures keyed by exact parameter rectangle
bits. Angular inspection creates an opaque pair fact only when both original
rectangles are available. A first completed endpoint can be reused later
without promoting its missing partner to coverage. Both rectangles and
tolerance are validated even when query budget is zero. Reversed endpoints
reuse the same boxes; pair facts still reject foreign source and pair.
Twenty-one chord/coverage tests and seven normal-alignment tests pass.
The independent partial-pair regression checks one paid endpoint, refusal
with its partner missing, one additional paid endpoint, free exchanged pair,
and invalid rectangle/NaN/foreign-source refusal. Report field
originalNormalRectangleCacheHits counts endpoint box reuse, separately from
derivative rectangle reuse. Tolerances and outward enclosure rules remain
unchanged. The full 100000-cell / 100000-restriction middle audit is running
with the preceding derivative-cache version.

The complete 51405859 middle audit at 512 cells preserves all four originals.
Forward models use 1175 restrictions with 896 normal rectangle cache hits;
reverse use 1193 with 879 hits. Previous derivative-only caching used
2071 / 2072 restrictions. Traversal and results match: 15 / 13 pending
representatives, 89 / 99 mean-matrix exclusions, and 167 / 178 total direction
exclusions. This measures fewer source queries, not UI speed or wall safety.
Full JSON/logs are archived.

Completed historical middle audits are archived with all four unchanged
originals. The 34ee6647 algebraic precondition audit uses 14721 / 14822 cells
and 100000 restrictions, retaining 15 / 10 pending pairs. The 822cba7b
in-pair normal reuse audit reaches 20000 cells with 90780 / 91372
restrictions, retaining 17 / 11. Both spatial directions match. Different
traversal depth means pending counts are not monotonic coverage percentages.
Neither audit qualifies an additional face.

## Original span restriction through closed tensor splits

Derivative hull queries now normalize original positive knot spans with
outward interval subtraction/division. Lower normalized endpoints round
down and upper endpoints up; exact original span boundaries map to 0/1.
The resulting closed unit rectangle contains every requested original point.
Tensor de Casteljau restriction then encloses the polynomial hull without
separate blossoms for each output coefficient. A nonseparated span-length
bound retains the old blossom path. Out-of-span input is rejected before
clamping. This supports arbitrary original span positions and scales.
Independent degree-2/1 polynomial formulas verify full, interior and point
rectangles on unit and offset spans, including exact endpoint values.
Eight normal, 21 chord/coverage and seven pole tests pass.

Full native nurbs-core library verification passes: 714 passed, zero failed,
two ignored microbenchmarks. Complete test log is archived.

The complete 64ad8556 symmetric middle audit preserves all four originals.
Radii [0.5, 1.25], direction 1.0: 20000 cells, 90368 restrictions, 14 pending representatives.
Radii [0.5, 1.25], direction -1.0: 20000 cells, 90368 restrictions, 14 pending representatives.
Radii [1.25, 0.5], direction 1.0: 20000 cells, 90412 restrictions, 13 pending representatives.
Radii [1.25, 0.5], direction -1.0: 20000 cells, 90412 restrictions, 13 pending representatives.
No additional face is qualified. Full JSON/logs are archived.

## Whole pending-pair diagnostics through Body and worker transport

Native Body face diagnostics now preserve every unresolved original closed
pair representative. The bridge exports unresolvedPairs and the explicit
endpointExchangeSymmetric flag. Transport checks the finite ordered shape,
exact pending count, exchange flag and last uncertain pair. Older packaged
responses without both extension fields remain readable; a partial or
inconsistent extension is rejected. The UI identifies each unresolved face
and its pending region-pair count without asserting a thin defect.

Six transport tests, 35 combined wall/worker/preview protocol tests and Vue
type checking pass. Native bridge restoration passes with array/count/last
obligation assertions. Original annular body radius/wall/seam acceptance
passes: axial wall encloses 6 mm and radial wall encloses 15 mm. The previous
fixed expectation of 16 reused self-face facts became stale when 18 faces
qualified; it is replaced by exact equality of issued and transported face
sets, plus retention of at least the 16 preceding qualified classes. The
whole-wall certificate still refuses, and limiting pole seam normals remain
unresolved. Complete passing logs are archived. This is not acceptance of
all requested bracket/flange/housing operation chains. WASM repack is next.

The complete de826db5 derivative-cache deep middle audit preserves all
four originals with a 100000-cell / 100000-restriction cap. Forward radii
retain 17 pending representatives after 25700 cells, reverse retain nine
after 25940 cells; both exhaust 100000 restrictions and match spatial
directions. All remaining rectangles are archived with complete JSON/logs.
This predates cross-pair normal reuse and general span tensor restriction.
No additional self-face certificate or global wall minimum follows.

The complete cb295499 tensor-restriction middle audit preserves all four
original models under 100000-cell / 100000-restriction caps. Forward radii
retain 11 pending representatives after 51004 cells; reverse retain 15
after 54723 cells. Both use the full restriction cap, and spatial directions
match. Full schema-2 JSON/logs include all remaining original rectangles
and both derivative and normal cache counters. No additional face is
qualified; different subdivisions preclude interpreting pending counts as
a coverage percentage. Distinct-face and global wall minimum remain open.

## Prepared packaged-WASM obligation regression

The real worker regression now requires the full native unresolved pair
arrays and explicit exchange marker. It retains the 16 original qualified
face identities from source-wall-adaptive-worker.json.gz:
1,2,3,4,6,9,11,12,13,14,15,18,19,22,23,26. Stronger native coverage may add
faces, while issued face facts and reused self-pair facts must match exactly.
Array length and last obligation must match the pending/uncertain fields;
a deliberately emptied unresolved array is rejected. Whole-wall refusal
and bounded work assertions remain. Vue type checking passes. Three
controlled-port contract tests pass for superseding restoration, abort/retry
and stale continuous coverage replies. The packaged binary requirement is
not yet executed because mandatory WASM optimization is still live.

## Packaged WASM worker — 2026-10-08

The packaged geometry WASM is 11,987,293 bytes, SHA-256
`d00b9f1dd638fcb6fe1477d849735a8b7230bbce066ddcbd8214f4db1d23e2b7`.
The real-worker continuous wall test passed with this binary (15.03 s for
three requests). Adaptive coverage retains all endpoint-exchange representative
obligations and reports 253 unproven body pairs. Whole-wall qualification stays
false; this is not a global thickness or general self-intersection certificate.

All 26 source-body-worker and wall-transport tests passed. Cancellation and
supersession tests include controlled worker ports; they do not prove rendered
UI cancellation. Type checking and production packaging passed. The measured
packed kernel grew from 3,987,584 to 3,997,424 bytes; bundle limits retain their
previous small margins. Evidence: `packaged-wall-worker.json.gz`,
`packaged-worker-tests.log.gz`, and `packaged-dist-verify.log.gz`.
