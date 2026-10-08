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
