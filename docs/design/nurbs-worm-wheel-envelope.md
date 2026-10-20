# ZA worm-wheel envelope contact foundation

API: `nurbs_core::worm_wheel::contacts` and search-free
`verify_contact_brackets`. NURBS-0088 remains planned: these are original-source
pointwise contact certificates, not NURBS wheel faces or an owned wheel body.

The [general envelope condition](https://www.mdpi.com/1996-1944/15/18/6412) is the
zero triple product of source tangents and relative generating velocity. This
implementation derives it for the nominal ZA source already implemented in
`worm`, using a perpendicular, nonintersecting cylindrical pair:

- Worm axis is Z, wheel axis is Y through (a,0,0).
- Worm pitch radius rp=q*m/2; wheel pitch radius rw=z2*m/2; a=rp+rw.
- h=hand*m*starts/2 and speed ratio eta=-hand*starts/z2.
- One selected original axial-rack working flank has slope k=+/-tan(alpha).
- Source point is (r*cos(theta),r*sin(theta),z), with
  z=profile_z(u)+start*pi*m+h*theta+axial_shift.
- At worm rotation phi, beta=theta+phi and the fixed-frame point is
  (r*cos(beta),r*sin(beta),z).

With the unnormalized source normal S_r cross S_theta, direct differentiation
gives the exact meshing condition

```
normal dot (worm_velocity-wheel_velocity) = eta*r*F(beta)
F(beta) = A*cos(beta)+B*sin(beta)-C
A=r+k*z, B=-h*z/r, C=rp
```

The cancellation C=a+h/eta=rp uses the original exact input relations; stored
binary64 approximations of pi or a sampled normal are not used as evidence.
Outward nominal worm parameter boxes, Taylor trig and interval arithmetic bind
all coefficients to the actual original spec, source flank, start, U and roll.
The source is the infinite analytic continuation of that working flank, not
an assertion that the corresponding point lies on a particular finite worm.

If D=A^2+B^2-C^2 has strictly negative upper bound, no contact angle exists.
If its lower bound is positive, A,C>0 and two disjoint brackets inside the
principal period have proved opposite endpoint signs and nonzero derivatives,
there are exactly two simple roots modulo 2*pi, both covered by those brackets.
The analytic binary64 atan2/acos candidates only propose brackets; independent
outward sign and derivative tests authorize the result. Tangent or uncertain
D, unsupported principal charts and unrepresentable requested accuracy remain
explicitly Unresolved, never false exclusions or fabricated contacts.

Each immutable contact owns beta, worm rotation, the actual wheel-frame point
box, source-normal box and relative-velocity box. The report privately owns the
original spec and parameters. Replay reconstructs every certificate without
running the floating-point candidate search. Wheel coordinates use
Ry(-eta*phi)*(fixed_point-[a,0,0]). Normal orientation is source S_r cross S_theta;
no outward wheel/material orientation or surface regularity is asserted.

Limits: original supported ZA spec, wheel teeth 12..256, original start index
0..starts-1, profile U in [0,1], actual signed worm roll in [-8,8], axial shift
within +/-1e6 and angular tolerance in (0,0.1] radians. Full sheet coverage,
continuation through tangencies, curvature interference, NURBS envelope fitting,
stock/root/tip trimming, complete tooth placement and owned wheel B-rep remain
open. Pointwise roots cannot substitute for those qualifications.

Tests cover both flanks and hands, independent source normals/relative velocity
and wheel-coordinate equations, independently finite-differentiated generating
motion triple products, original multi-start phase binding, complete root pairs,
exclusions, near-tangent refusals and corrupted/missing/overlapping brackets.
Numerical differentiation is an independent regression oracle; it is not the
interval contact certificate.

## Explicit contact-angle chart and whole-rectangle regularity

`enclose_chart` changes independent parameters from (U,theta) to (U,beta).
For original r(U), axial profile z0(U), k and h:

```
K = k*cos(beta) - h*sin(beta)/r
Z = (rp-r*cos(beta))/K
source_theta = (Z-z0-start*pi*m-axial_shift)/h
worm_phi = beta-source_theta
wheel_psi = eta*worm_phi
```

This directly and uniquely solves the original meshing equation whenever h*K
is nonzero. There is no floating-point root search. The chart passes through
angular-root folds where the earlier fixed-source-roll inverse query must stay
Unresolved. It remains an analytic continuation of the original source flank;
finite source/cutter stock membership is not inferred.

Outward interval automatic differentiation reconstructs every original source
position, first partial derivative and Jacobian cross product on the entire
requested U/beta rectangle. Signed denominators use signed outward division.
A regularity receipt requires at least one cross-product component strictly
separated from zero throughout that whole cell. Positions and derivatives are
bound to original module/pressure/hand/start/tooth data, including all parameter
and trig rounding. Point evaluation rounding remains separate.

`cover_chart` adaptively partitions the complete requested original rectangle.
Its private flat tree owns all parent cuts, both children, source domains and
whole-cell bounds. Search-free replay checks node census, duplicate/cyclic
references, strict parent/child coverage and every original Jacobian receipt.
No leaf is dropped. A denominator, arithmetic range, Jacobian or resource
failure remains an unqualified leaf; `complete()` is false. This is local
regularity and complete coverage of the *requested chart*, not global sheet
injectivity, absence of curvature interference outside it, full cutter/stock
domain coverage or an owned wheel.

Domain limits: U in [0,1], beta in [-1.4,1.4], at most 65535 nodes and depth 32.
The Taylor wheel-phase range must be proved on each admitted cell. Degenerate
intervals are allowed for point enclosures; covers require a nonempty 2D domain.
Tests compare full-box positions and AD derivatives to independent formulas
and finite differences, cross a near angular-root fold, replay full U covers
through beta +/-0.3 for both hands/flanks and multi-start source phase, and
preserve incomplete status for singular/under-budget rectangles. The numerical
sum of leaf areas is a regression oracle; the checked tree proves coverage.
NURBS envelope interpolation, actual saved-patch regularity/embedding and wheel
stock/root/tip/B-rep assembly remain open.

## Certified quintic NURBS envelope grids and lossless packing

`fit_chart` constructs unit-weight quintic tensor NURBS patches over the full
requested original chart rectangle. Six dyadic local nodes
[0,1/8,3/8,5/8,7/8,1] define the Lagrange interpolant per axis. The shared analytic
expression is evaluated by both first-derivative interval algebra and a
sixth-order Taylor algebra whose coefficients are f^(k)/k!. Product, quotient
and trig recurrences propagate outward bounds on the entire original cell;
finite differences and sampled residuals never authorize approximation error.

Let W bound the normalized six-node product, Lambda bound the univariate
Lagrange operator norm, and Tu/Tb bound the Euclidean sixth Taylor coefficients
throughout a cell. Tensor interpolation error is bounded by

```
Eu = Tu * delta_u^6 * W
Eb = Tb * delta_beta^6 * W
E_interpolation <= Eu + Lambda * Eb
```

W and Lambda are independently bounded from the actual interval Bernstein
coefficients of the original cardinal polynomials and their dyadic subdivisions.
The second tensor term uses the Lagrange operator norm; it is not incorrectly
treated as an independent positive Bernstein interpolation.

Each exact affine sample location is outward-enclosed in global original
parameters before source evaluation. Ideal interpolant poles are reconstructed
from those source boxes by interval Lagrange-to-Bernstein conversion. The maximum
Euclidean difference between every actual saved pole and its ideal pole box
bounds the entire stored-patch perturbation by positive Bernstein partition of
unity. This includes sample position, source/trig/parameter, basis conversion
and saved-control rounding. Adding this bound to E_interpolation qualifies the
actual saved surface against the original analytic envelope. Future point
algorithm rounding is separate.

`certify_envelope_grid` independently replays original domains, full grid census,
every actual pole/knot/weight, whole-source regularity and actual saved-patch
Jacobian hull bounds. The constructor refines the full uniform grid anisotropically
until the unchanged caller tolerance and both regularity predicates pass. It
never drops a singular or under-budget cell. Work is bounded by 4096 patches.
The opaque owner retains original source data, controls and proof and replays
all checks. Exact identical shared control rows/columns prove actual C0 seams;
higher-order seam continuity and global embedding remain unqualified.

`packed_surfaces` merges up to 6x6 original Bernstein cells per stored Surface,
respecting the kernel's 32-control-per-axis limit. Full degree-five internal
knot multiplicity and direct original pole-bit copying preserve every cell's
mathematical definition without fitting, arithmetic on poles or tolerance loss.
`verify_packed_grid` reconstructs and compares all original packed definitions.
Packing is exact C0 representation compression and does not raise continuity
or assert regular derivatives across internal multiple knots.

Verified example: default descending flank, U=[0,1], beta=[-0.15,0.15], requested
world tolerance 1e-4. Thirty-two quintic cells pack into two NURBS surfaces; the
continuous original-source error upper bound is 1.0773757602446802e-8 in model
units. This is an example result, not a universal patch-count/performance claim.
Tests cover interval Taylor series, cardinal quintic reproduction/operator
bounds, dense source comparisons, original-spec/control/basis corruption,
resource refusals and exact packed-definition replay with numerical position
regressions. The numerical comparisons are oracles, not continuous certificates.

Full stock/cutter contact-domain selection, root/tip/stock transitions, whole
tooth placement, global face embedding and owned wheel B-rep remain open.
NURBS-0088 remains planned until those native figure requirements are met.


## Original stock-cylinder intersection, 2026-10-16

`worm_wheel::intersect_stock_cylinder(spec, flank, domain, radius, cells)` proves one original envelope crossing of a specified wheel-axis cylinder for **every** beta in the requested interval. It uses the rotation-invariant analytic squared radius `(r*cos(beta)-center)^2 + Z^2`, not a fitted surface or sampled sign grid. Interval automatic differentiation bounds its U derivative over the complete Cartesian product of a contiguous U partition and a closed angular cover of at most 16 intervals; all cells must have the same strict sign. Opposite strict original endpoint signs then prove existence and uniqueness continuously. A complete cover of at most 16 angular intervals sharpens endpoint sign bounds for both derivative and sign bounds. Independent endpoint contraction retains all roots over the angle interval; intervals containing zero are never interpreted as empty.

The returned private `StockIntersection` replays the original source proof. `section(beta)` isolates a requested angle, `position_bounds()` encloses its original wheel-frame positions, and `tangent_bounds()` uses implicit differentiation `dU/dbeta = -G_beta/G_U`. `stock_interior()` binds the blank material side to the proved monotonicity: lower U for increasing radius, higher U for decreasing radius, with the boundary included. This concerns the original cylindrical blank and does not prove swept-cutter contact admissibility. A derivative or denominator that cannot be separated from zero is refused; no assumption of monotonicity is taken from a caller flag. The U subdivision count is 1..4096 (at most 65536 derivative rectangles including angle coverage) and radius is finite, positive and at most 1e8 model units. Original chart restrictions still apply.

Three focused integration tests cover both flanks/hands, independent source root bisection at 101 angles per configuration over beta [-0.15,0.15], pointwise edge enclosures, independent finite-difference tangent checks, absent crossings, an actual low-tooth radial fold, invalid angles/domains and proof limits. Together with contact/chart/fit regressions, 17 integration checks pass. This supplies a certified implicit stock edge, **not** a fitted trim curve, swept-cutter admissibility proof or closed wheel. NURBS-0088 remains planned.


### Continuous original stock-trimmed charts

`StockIntersection::enclose_trimmed_chart(t)` now encloses the true crest-to-stock parameterization `S(crest + (U_stock(beta)-crest)*t, beta)`, with local t in [0,1], for either original flank. It requires the whole original U domain [0,1]. The report includes original positions, t/beta derivatives, conservative regularity and univariate sixth Taylor coefficients `f^(6)/6!` for both axes. This remains an analytic-source bound, not a fitted stock surface.

For beta derivatives, interval implicit differentiation solves each Taylor coefficient from `F_U U_n + remainder_n = 0` through order six. The opaque stock proof supplies the original root enclosure; original `F_U` must exclude zero over it. Here `F=rho^2-R_stock^2`; its constant radius contributes nothing to coefficients n>=1. All residual coefficients are reevaluated from the shared original envelope expression, with outward arithmetic. A separate independent square-root series regression verifies this recurrence through order six; the stock chart matches independent source positions and finite-difference first jets on both flanks. Singular/insufficient interval bounds refuse rather than manufacturing a derivative proof.


### Stock-chart NURBS approximation

`fit_stock_chart(&StockIntersection, tolerance, max_patches)` reuses the original quintic interpolation/error and lossless packing implementation with an explicitly owned stock source. U in its grid is local crest-to-stock t, not original profile U; `stock_source()` makes that distinction observable. Each source cell recomputes the implicit root and sixth coefficients. Sample position boxes include root-isolation and parameter rounding. `certify_stock_grid` independently replays actual stored controls, weights, knots, source regularity, saved-patch regularity and original-stock error; packed replay still adds zero error. An original-profile grid never acquires stock meaning by a caller flag.

The tested default two-flank strip beta [-0.01,0.01], requested world tolerance 1e-3, uses two quintic cells packed into **one NURBS Surface per flank**. Continuous original-source errors are 2.3910317955793932e-8 (descending) and 2.2937546534879198e-8 (ascending). Independent original source point oracles, altered actual poles, altered stock radius and insufficient tolerance/budget are covered. These are working flank surfaces, not the complete wheel: generated root transitions, stock-tip/axial boundaries, tooth replication and owned body construction remain open. The wider left-handed two-start source, original start 1, beta [-0.15,0.15], passes at requested tolerance 1e-4 with 64 cells packed into four surfaces and error upper 4.4763921189421845e-9. The final regression has 23 integration checks plus two Taylor algebra checks. Ephemeral original-source constants are prepared once per admission and rebuilt on replay; sample-root reuse is confined to immutable, exactly matching angle-domain evidence.


## Generated root sources and NURBS, 2026-10-16

`root_chart(spec, RootPart::{CrestEdge(flank), CrestCylinder}, beta)` owns the original ZA contact proof. `fit_root_chart` returns a separate `QualifiedRootGrid`; a root cylinder never receives a fictitious working-flank role. `certify_root_grid` and `verify_packed_root_grid` replay the actual saved definitions and the original root source. Working-flank and root grids share private interpolation/packing data while keeping distinct public source roles.

Let Rc be the original worm crest radius, rp/rw the worm/wheel pitch radii, h the signed axial rise per worm radian, eta=-h/rw, c=cos(beta), C=Rc*c-rp and K=k*c-(h/Rc)*sin(beta). For a convex crest edge, Zf=-C/K is the original flank contact position and Z=t*Zf, t in [0,1]. The actual original edge has its original crest phase z0; theta=(Z-z0)/h, phi=beta-theta. The ordinary worm/wheel rigid transformations then give its generated surface. It is not a loft between unrelated boundaries.

The radial crest-cylinder normal velocity is Vc=-eta*Z*c. The outward flank velocity is Vf=s*eta*Rc*(1-t)*C, with s=+1 on the descending side and -1 on the ascending side. Therefore Vc*Vf=[eta^2*Rc*c*C^2*s/K]*t*(1-t). Admission proves c>0, C>0, s*K<0 and a strictly negative interval factor over the complete beta interval. The two original outward normals have opposite velocities in the interior and admit a nonnegative normal-cone combination; endpoint contacts include the respective cylinder/flank normal. The original ZA crest is convex by the validated positive profile widths and its outward radial/flank half-spaces.

For `CrestCylinder`, original z0 runs from the ascending crest phase p-wc/2 to the next-period descending crest p+wc/2, with original start and axial translation. Its unique meshing solution is Z=0 because dVc/dZ=-eta*cos(beta) excludes zero. The cylinder begins on the same-period ascending transition and ends on the descending transition rotated by -2*pi/teeth. Independent original helical-point/worm-rotation/wheel-frame tests check both joins and the source phase.

### Regularity and original-source injectivity

For the edge, dP/dt=(Zf/rw)*R_psi[-Z,0,C], while dP_y/dt=0 and dP_y/dbeta=Rc*cos(beta)>0. Hence the whole-chart cross-product norm is at least |Zf|*C*Rc*cos(beta)/rw. For the cylinder it is at least |q|*wc*Rc*cos(beta)/rw, q=Rc*cos(beta)-(rp+rw). Both lower bounds are computed outward from original data. This avoids cancellation of large dependent interval jet terms; saved-NURBS regularity remains an independent actual-control check.

`RootEmbedding` describes the original chart only. Y strictly fixes beta. At fixed beta an edge has rho^2=q^2+t^2*Zf^2, strictly increasing with t on [0,1]; the cylinder has a monotone arc of positive span wc/rw, proved shorter than 2*pi. These establish injectivity of each original source chart. They do not establish saved-grid/global wheel embedding, visibility against other cutter parts or finite cutter coverage.

### Approximation and ownership checks

The same univariate sixth Taylor algebra supplies f^(6)/6! for t/beta; independent affine-trigonometric sixth-derivative formulas check every root family. Original kinematic transforms, first jets, the whole-domain normal-velocity product factor, source invariants and malformed-domain refusals are tested for both hands and multiple starts. Root ownership replay takes the original source exposed by the owner, rather than an inner cached source; a phase-invariant contact proof cannot authorize poles from a different phase.

At default beta [-0.15,0.15] and requested world tolerance 1e-4, each family for either hand fits in one degree-five tensor Surface. The largest continuous original-source error upper is 4.981918783064071e-5 (ascending/right); the generated root cylinder has error <=1.184e-8. Edited poles, weights, source phase and insufficient proof budgets are refused or produce an explicitly inadequate error bound. Lossless packing replay now compares all saved knot/control/weight bits, including signed zero; a signed-zero knot change is not relabelled as byte-preserving packing.

Whole-wheel stock-tip boundaries, finite axial stock clipping, tooth placement, actual cross-family interface assembly and the owned closed B-rep remain open. NURBS-0088 remains planned. The complete source/candidate boundary proof for the eventual body must use the actual stitched/placed controls, not these separate family receipts.


## Finite physical-Y side charts, 2026-10-16

The angular strip's variable end heights are replaced by explicit `AxialDomain { t, y }` inputs. Physical Y remains the wheel-axis coordinate in model units; the public API does not rename it beta. `AxialRootChart`, `AxialStockIntersection`, `AxialTipChart` and their separate qualified grids retain the actual original source roles. These are the six side-face families of one tooth period, not a closed wheel B-rep.

### Certified inverse coordinates

The scalar interval/first-jet/sixth-Taylor algebras now implement positive square root and principal asin. Value asin uses monotone endpoint bounds, sqrt and the certified atan identities, switching to pi/2-atan(sqrt(1-x^2)/x) near positive one and using odd symmetry for negative arguments. This covers both reductions without relying on libm as proof. Derivatives require strictly positive radicands and asin arguments strictly inside (-1,1). Square-root coefficients solve 2*y0*yn=xn-sum(yk*y(n-k)); inverse-sine coefficients integrate x'/sqrt(1-x^2) through order six. Independent binomial/inverse series and near-endpoint value tests pass.

Admission now tightens trigonometric ranges by a complete 16-cell angular cover. Each cell retains the original certified Taylor/Lipschitz enclosure; its hull avoids refusing an admissible wide physical-Y slab merely because a single midpoint cosine bound is too coarse. No requested geometry or tolerance is changed.

For root sources beta=asin(Y/Rc). Original contact and source injectivity transfer through this monotone map. The original cross-norm lower bound is divided outward by an upper bound of Rc*cos(beta). The source expression keeps its exact-real identity P_y=Y explicit, so its t derivative and both sixth Y-coordinate coefficients are zero, while dP_y/dY=1.

### Working flanks and stock at fixed Y

For original profile radius r(U), let X=sqrt(r^2-Y^2), q=X-(rp+rw), K=k*X/r-h*Y/r^2 and Z=(rp-X)/K. The original squared wheel radius is q^2+Z^2. `axial_stock_intersection` proves strict U monotonicity over every rectangle of a complete original-U by physical-Y census (up to 4096 U cells and 16 Y cells), then checks inside-blank crest/outside-blank far-end signs. A unique original stock crossing exists for every Y; common root contraction preserves all of them. This uses sqrt/rational arithmetic directly, so radial admission does not repeatedly evaluate irrelevant wheel-frame angles.

The true retained chart is U=crest+(U_stock(Y)-crest)*t, beta=asin(Y/r(U)); no unrelated helical/spur profile is substituted. Implicit Taylor coefficients solve F_U*U_n+remainder_n=0 through six using original physical-Y F. The independently proved full-census derivative range may sharpen the root-box derivative interval. Source rank has lower bound min|F_U|*min|U_stock-crest|/(2*max(rho)): Y fixes Y, and strict radial progress fixes t, also proving original chart injectivity. Candidate rank remains independently checked from actual saved controls.

`position_bounds` intersects direct boxes with a centered mean-value enclosure when useful. The anchor is a freshly isolated original root at an exact stored Y midpoint; original first jets enclose derivatives over the complete requested t/Y rectangle. This preserves the implicit U(Y) correlation through a continuous theorem, not a sampled claim.

### Cylindrical stock tip

`axial_tip_chart(descending, ascending, proof_cells)` requires matching original Spec bits, radius, physical domain and actual flank roles. Complete Y coverage proves A_x*B_z-A_z*B_x<0 and A_x*B_x+A_z*B_z>0 for their original stock-end points. The source is the unique clockwise short stock-cylinder arc:

P_XZ=R*((1-t)A+tB)/|(1-t)A+tB|, P_y=Y.

Both ends satisfy the SAME original stock equation, so |A|=|B|=R exactly for the nominal source. The squared denominator is equivalently R^2+2*t*(1-t)*(A.B-R^2), with R^2/2<=norm^2<=R^2. The interval/Taylor analysis uses this proven original-source constraint, retaining all derivative coefficients while sharpening coefficient zero. It makes no circle claim about arbitrary rounded or saved endpoint poles. A separate exact unit-circle/Legendre-series check verifies sixth coefficients. Mean-value endpoint boxes further sharpen value intervals. This reduced the tested tip from 12 packed surfaces to three at the unchanged requested tolerance.

Strict angular progress bounds its source cross norm below by min|A cross B|/R; exact Y and the short arc prove source injectivity. Independent atan2-based circle-arc oracles, endpoints, source jets, changed radius/phase/roles, altered poles and plane controls are tested.

### Saved planes and counts

The fitter repeats one strictly progressing physical-Y control column across all t rows. End controls copy the requested Y plane locations exactly. Interior one-fifth controls are rounded, so candidate Y(v) need not be the exact identity; that rounding remains in the original-source control/error bound. The actual end boundary curves are nevertheless exactly coplanar in the represented NURBS geometry. Admission checks these canonical Y controls, not a caller planarity flag, and refuses even a tiny plane-control change. Packing remains bit preserving.

For default original Spec, Y=[-2,2], R_stock=21 and requested world tolerance 1e-4:

| Family | Quintic cells | Packed Surfaces | Continuous source error upper |
| --- | ---: | ---: | ---: |
| Descending crest transition | 4 | 1 | 8.869327476089024e-6 |
| Ascending crest transition | 4 | 1 | 9.771844750531934e-6 |
| Crest-cylinder root gap | 2 | 1 | 5.4652389029748185e-6 |
| Descending working flank | 32 | 3 | 5.617040773915694e-5 |
| Ascending working flank | 32 | 3 | 5.502907909696017e-5 |
| Stock tip | 16 | 3 | 5.49158357658283e-5 |

All families are independently packed. Their actual cross-family controls/bases have NOT yet been synchronized or sewn. The next stage must qualify the actual common/stiched/placed controls, complete all periods, build the two planar caps and bind the complete closed model to original source error. Global cutter visibility, whole-wheel geometric solid qualification and viewer/export integration remain separate. NURBS-0088 stays planned.

## Finite physical wheel candidate and planar ends

The Rust chain is `axial_side_network` → `worm::wheel_caps` →
`worm::wheel_body_candidate`, with the convenience entry point
`brep_core::worm::finite_wheel_with_certificates(Spec, WheelBuildOptions)`.
The options declare physical-Y breaks, all six complete t grids, stock radius,
side tolerance, patch budget and cap proof budgets. Defaults describe the
module-1, 40-tooth wheel, Y=[-2,2], stock radius21 and side tolerance1e-4.
Other specifications must supply compatible dimensions and sufficient grids;
there is no silent resolution or tolerance relaxation.

All original source families are retained for every period: descending crest
root, descending working envelope, cylindrical tip, ascending working envelope,
ascending crest root and crest-cylinder gap. Nominal period angles use certified
-2*pi*j/N. Actual world boundary controls are shared after placement and then
admitted against their original sources. Source derivatives, actual saved rank,
complete source coverage and world-space fit error are checked. This is an
original ZA-generated wheel, not a spur/helical replacement.

`end_contours` copies the actual packed end rows. It reverses ascending-family
poles, weights and negative-reversed knots exactly. A proper frame change
(x,y,z)→(x,-z,y) makes the physical ends Z=-2 and Z=2. General planar trim
admission proves each complete 240-curve default contour simple and positively
oriented, then proves every world edge/pcurve/support agreement. Curved fan caps
and sampled winding are not admission evidence.

The B-rep uses complete packed multi-span borders, shared edges with exact
control/knot/weight correspondence, opposite effective traversal, connected
shell topology and cyclic vertex links. The default census is722 faces,
1680 edges and960 vertices. The lower planar face use is reversed. The candidate
stores the whole-model watertight qualification; an unresolved boundary check
refuses construction. `wall_error_upper` reports only the maximum original side
error. `solid_geometry_certified` remains false and
`ideal_finite_body_error_upper` remains None: global saved-surface intersections,
complete cutter visibility, full ideal finite-body error and production/runtime
qualification remain separate.

Limits are explicit: source teeth12..256, at most4096 patches per family and
16384 network patches, B-rep at most1024 faces, general cap at most256 contour
curves, and bounded trim/agreement work. Thus not every otherwise valid source
count/resolution fits this body builder; unsupported or unresolved inputs fail.
No counts are promoted solely from aliases or side charts. Qualification logs
for the current finite-body work are under
`docs/qualification/nurbs-worm-wheel-caps-2026-10-16/`.
