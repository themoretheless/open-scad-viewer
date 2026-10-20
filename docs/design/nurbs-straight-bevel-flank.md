# Spherical involute and straight-bevel working flank

Rust owner: `nurbs_core::involute::qualified_straight_bevel_flank`.
Source: [straight-bevel involute-cone equation](https://www.mdpi.com/2076-3417/14/24/11919/html), section 2.1.
NURBS-0086 remains **planned**: this is its working-flank foundation, not a complete gear.

For base cone semi-angle d, s=sin(d), c=cos(d), roll t, w=s*t:

```
q(t) = [s*cos(w)*cos(t)+sin(w)*sin(t),
        s*cos(w)*sin(t)-sin(w)*cos(t), c*cos(w)]
S(u,v) = ((1-v)*R_inner+v*R_outer)*q(t_start+(t_end-t_start)*u)
```

The source unwinds a great-circle segment from the base cone. It lies on the
unit sphere and has a stationary base-point cusp at t=0. No regularity claim
is made at that cusp. This is a spherical involute, not a scaled planar tooth.

`approximate_spherical_involute` returns cubic endpoint-Hermite spans and an
explicit real-arithmetic fourth-derivative remainder. XY product-to-sum
harmonics have frequencies 1-s and 1+s; Z has frequency s. The Euclidean
coordinate remainder is M*h^4/384. This approximate constructor does not
include rounding in its estimate.

`certify_spherical_involute` independently reconstructs source endpoint jets
using outward Taylor trigonometric intervals, compares every saved Bernstein
pole, verifies the original cubic C0 knot/weight layout, and adds the outward
Hermite remainder. The exact real values of binary64 source inputs are the
reference. Saved controls, knots and parameter derivation rounding are covered;
rounding of subsequent point evaluation is separate.

`straight_bevel_flank` rules an outer spherical involute towards the cone apex.
`certify_straight_bevel_flank` extracts the actual outer surface row, requalifies
it, and bounds every inner-row pole against exact radial scaling. Convexity
extends this bound across the whole surface. Different source radii, angles,
inner poles, weights and knot ownership cannot reuse a previous receipt.

`qualified_straight_bevel_flank` owns the actual surface, source, cone distances,
tolerance and independently recomputed certificate. Its fields are private;
`reverify` rebuilds the continuous bound against the original source. Construction
refuses if the bound exceeds the caller's unchanged tolerance.

Limits: base cone in (0,pi/2), source roll in [0,16] for evaluation/approximation,
roll <=8 for inclusive certification, 85 cubic curve spans, 32 surface U controls,
finite positive cone distances with inner < outer. A fine full-width flank may
exceed the surface control limit and is explicitly refused. Multi-patch flank
assembly, nominal gear/pitch parameter derivation, left/right tooth placement,
root/tip transitions, bore/caps, original full-profile embedding, owned B-rep
faces and complete gear/meshing qualification remain to be implemented.

Tests use independent harmonic source positions, sphere/tangent/speed identities,
whole radial-surface samples, continuous certificate replay, perturbed original
poles/radii/angles/weights, invalid inputs, tolerance and source ownership.
Sampling is a regression oracle, not the continuous error certificate.

## Original-parameter patch assembly

`qualified_straight_bevel_patches` removes the single-surface control limit for
whole working flanks: it constructs one original cubic profile (up to 85 spans),
then copies groups of at most ten Bernstein spans into separate radial surfaces.
Every patch retains its original global U domain. There is no re-fit, knot
normalization, roll-interval reconstruction or separate local source formula.
Adjacent patches own exactly equal radial rows and weights, proving C0 across
the complete radial seam. G1/G2, regularity and sewn B-rep faces remain separate.

`certify_straight_bevel_patches` validates actual patch knot layouts and shared
rows, reconstructs the original outer chain, and applies the original-source
rounding-inclusive certificate once to that complete chain. It proves complete
ordered coverage of [0,1] and bounds all inner-row deviations continuously.
Missing, reordered, overlapping, differently weighted or disconnected patches
are refused. The opaque owner replays this proof and enforces the unchanged
caller tolerance. Up to nine patches cover the current 85-span curve limit;
the caller's smaller explicit patch budget is respected.

The single-surface constructor keeps its original 32-control limit. Whole-gear
nominal parameter derivation, tooth placement, root/tip transitions, profile
embedding and B-rep assembly are still pending. Patch admission alone does not
promote NURBS-0086 to native-ready.

## Nominal gear parameters and whole-flank source binding

`BevelGearSpec` defines module, tooth count, pitch/pressure angles (radians),
cone-distance face width, addendum, dedendum and pitch-arc backlash. The sharp
unshifted cone construction uses R=m*z/(2*sin(delta)), base cone
asin(sin(delta)*cos(alpha)), tip/root cones delta +/- atan(height/R), and
roll at a cone angle beta equal to acos(cos(beta)/cos(base))/sin(base).
Where the root cone lies below the base, the working involute starts at roll 0;
a root transition is still required. Tooth step, pitch roll, pitch involute
azimuth and pitch half-thickness are derived for subsequent tooth placement.
[Reference cone-distance and tooth-height conventions](https://www.sdp-si.com/D815/D815-Technical-Section.pdf).
This does not implement Gleason profile shifting, cutter envelopes or fillets.

`derive_bevel_gear_parameters` uses outward arithmetic, certified Taylor trig,
Machin pi, the existing bounded atan series with reciprocal reduction and
monotone sine bracketing for inverse sine. Overlapping interval comparisons
stop refinement without narrowing the proved bracket. Unresolved root/base
ordering and axis/equator/apex limits are explicit refusals.

`qualified_nominal_bevel_flank` constructs the complete patch set from central
stored parameters and propagates the full nominal parameter boxes into its
world-space error. For the unit spherical involute, ||q||=1,
||dq/droll||<=1 and ||dq/d(base angle)||<=1+|roll|. Therefore the endpoint radius,
roll and angle errors add a continuous inclusive nominal-source bound to the
actual saved-net certificate. The owner saves the original spec and rederives
it during verification. The caller's spatial tolerance remains unchanged.
This closes nominal working-flank derivation; whole-tooth placement, root/tip
connections, profile embedding and complete B-rep gear admission remain open.

## Complete paired working-flank inventory

`qualified_bevel_teeth` constructs both working flanks for every nominal tooth.
For tooth-center phase C=j*2*pi/z, pitch half-thickness H and pitch involute
azimuth Ip, the left flank is rotated by C-(H+Ip); the right flank is reflected
in Y and rotated by C+(H+Ip). At the pitch cone this gives the intended phases
C-H and C+H, including backlash. The base nominal source and original global
patch parameters are preserved.

`certify_bevel_teeth` replays the original nominal source, requires exactly one
ordered left/right pair for every tooth, checks each actual original basis,
weight and pole, and bounds placement against outward nominal phase intervals.
Bernstein convexity extends the pole difference continuously; ideal rotations
and reflections preserve the original source error. Adding these bounds gives
whole-inventory world error including phase derivation and saved-net rounding.
Every placed patch join must retain an exactly common radial row.

Ideal tip/root half-thickness is derived from endpoint azimuth bounds in the
first unwinding quadrant. The source azimuth increases monotonically there.
A nonpositive tip, unresolved quadrant or overlap of neighboring nominal tooth
sectors is refused. This is separation of the *ideal working-flank sectors*,
not proof of actual closed-profile embedding or an admitted B-rep solid.

The opaque owner saves the complete paired inventory, spec, source, receipt and
tolerance; revalidation reconstructs all proofs. Caller budgets cap patches per
flank and total surfaces (hard maximum 8192). Root and tip connections, root
fillets, cap/bore topology and complete gear qualification remain open.

## Closed outer spherical outline

`qualified_bevel_outline` closes the actual saved paired working flanks with
quadratic rational transitions. Each tooth follows the left flank from root to
tip, a tip parallel, the right flank in reverse, and a root parallel towards
the next tooth. When the root cone lies below the base cone, spherical meridian
arcs join the base endpoints to the root cone. These are sharp theoretical
transitions, not generated cutter envelopes or manufactured root fillets.

Original flank curves are copied from actual outer radial rows without fitting.
Right-side reversal negates the reversed knots exactly and reverses the original
controls/weights. Arc endpoints own the original shared endpoints exactly;
the last root parallel owns the first tooth's saved root point. Every join,
including closure, is verified through clamped original endpoint ownership.
The resulting contour is generally nonplanar and approximates a spherical
outer section; it must not be passed to a planar-profile verifier as planar.

`certify_bevel_outline` reconstructs the full original source-role inventory,
requires unchanged working-flank curves, verifies canonical arc endpoints and
checks actual transition poles/weights against independently derived outward
nominal rational-circle boxes. If dp bounds every pole difference, dw bounds
every weight difference, M bounds ideal pole norms and wmin is the minimum
saved positive weight, rational Bernstein convexity gives dp+2*M*dw/wmin.
This includes circle derivation, saved-control/weight rounding and endpoint
canonicalization without relying on samples or libm accuracy as proof.
The complete profile bound is the maximum of working-flank and transition
bounds. The private owner replays the original spec and enforces the caller's
unchanged spatial tolerance and curve/surface budgets.

Tests cover both below-base and above-base roots, exact full-chain closure,
independent nominal rational circle parameterization of every transition,
spherical radius, changed source curves, modified transition poles/endpoints,
missing inventory and curve limits. Actual complete-profile embedding,
cap/bore support surfaces, trimmed B-rep assembly and owned solid admission
remain unqualified. Closed source-bound contour qualification alone does not
promote NURBS-0086.

## Original 3D profile embedding through exact projection

B-rep API: `brep_core::gear::certify_bevel_profile`. The source-bound outer
outline is regrouped into four components per tooth above the base cone, or
six below it. Whole working flanks are reconstructed by copying the complete
original cubic Bernstein controls and global knots from their patches;
reversal and patch domains are retained. No re-fit or approximate conversion
is used. The original 3D grouped curves remain owned.

Each original grouped curve is projected to XY by copying its first two
control coordinates and retaining all weights, knots and degrees. The general
four/six-component modular gear-profile producer now accepts original source
wires independently of the planar involute generator. Its existing verifier
still requires complete local two-group exact certificates, strict rational
far-hull separators, exact winding and origin-free hulls.

`verify_bevel_profile_projection` reconstructs grouping/projection from the
original owned 3D source and requires identical projected source bits and
complete matching tooth roles before checking any modular receipt. A simple
projection proves the original 3D contour has no additional self-intersections.
The private owner replays original accuracy and topology certificates. This
is original contour embedding, not regular cap/wall surfaces, bore topology,
sewn B-rep or solid admission. Existing proof budgets and their explicit
refusals remain unchanged.

## Closed native body candidate with original-profile ownership

`brep_core::gear::bevel_body_candidate` constructs homothetic radial walls and
an explicit ruled fan cap at each cone-distance end. Each fan connects the saved
profile to its corresponding axis point [0,0,R]. These are neither planar nor
spherical caps. They define the declared finite-body cap reference for this
native constructor. The current constructor produces a blank without a bore;
manufactured fillets, bore subtraction and cutter-envelope geometry are separate.

Original outer profile curves are retained as actual body edges. Inner curves
use the stored radial ratio. Outward nominal ratio intervals bound every scaled
pole against exact original-spec homothety; a conservative coordinate L1 bound
adds this error to the original complete profile bound. Fan interpolation adds
only the bound on its nominal axis point. This continuously bounds all authored
walls and fan caps against the declared original-spec reference, including
stored radial rounding. Future point-evaluation rounding remains separate.

Each profile curve owns three faces: one radial wall and two cap fans. Radial
and cap-spoke edges are shared, and cap axis boundaries use explicit degenerate
pole edges. Pole collapse is checked from the complete original surface control
net. The body must pass combinatorial/UV validation and whole-interval edge/lift
agreement with exact and interval work budgets. An insufficient budget refuses
admission rather than changing geometry or tolerance. The 1024-face kernel limit
is enforced before topology allocation.

`bevel_with_verified_profile` first checks cheap source/body feasibility, then
produces the full exact source-profile embedding certificate. The independently
verified 3D source curves must match the actual outer body edges exactly. Its
private `ProfileVerifiedBevelGear` owner exposes the model, source and receipts,
nominal world error and continuous shared-edge closure. Like the existing four
native planar-involute gear constructors, it explicitly returns
`solid_geometry_certified() == false`: global cap/wall embedding, complete solid
admission, manufacturing envelopes and export/viewer integration are separate.
This preserves the catalog's native-constructor criterion rather than issuing
an unproved global solid certificate.

Candidate tests cover 6- and 20-tooth closed models, pole corruption, insufficient
tolerance and all boundary uses. An independent volume regression compares the
native integral to V=(1-lambda^3)*R*A_XY/3, using a separately sampled projected
source area. The volume regression is a numerical oracle, not a solid-validity
certificate. The combined default native constructor has its own integration
test; consult the qualification receipt for completed versus running gates.
