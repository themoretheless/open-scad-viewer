# Native library parity work ledger

> **Актуальный остаток на 2 ноября 2026:** [консолидированный план](plasticity-remaining-2026-11-02.md). Он имеет приоритет над устаревшими статусами в исторических записях ниже. Полный библиотечный паритет не завершён.

User objective: "все реализовано у меня". Scope is native modeling libraries,
not UI command names. This objective is **active after user-requested resume**, not completed by adding a
same-named API, an open sheet, sampled evidence or a bounded implementation.
No proprietary runtime/code is copied or linked. Keep the frozen clean library.

Состояние после явного возобновления 2026-11-02: цель **активна**.
Сводка результатов, точная граница «откуда что воссоздано», последние 27 тестов
и полный остаток: [plasticity-transfer-status-2026-11-02.md](plasticity-transfer-status-2026-11-02.md).
Сводка была сохранена во время предыдущей паузы; последующее явное изменение
статуса пользователем возобновило работу. Полный scope не сокращён.

## Порядок оставшейся реализации

Свежая привязка Full Round к установленной beta и подтверждённой цепочке
вызовов Parasolid: [reference evidence](plasticity-full-round-reference.md).
Выполнение Plasticity на сравнительных телах пока не наблюдалось; общий
паритет Full Round остаётся открытым.

Первоочередная привязка после напоминания пользователя: каждый пункт должен
иметь конкретный референс в нативной библиотеке Plasticity/plasticity-beta,
свой API и проверку эквивалентного поведения. Собственные тесты ядра не
доказывают паритет. Непроверенные соответствия остаются явно неизвестными.

1. Пересечения: физическое владение замкнутыми ветвями, общие world/UV концы
   mixed-axis traces, автоматические closed BRep trims; затем полный census
   скрытых ветвей, касаний, периодических областей и полюсов.
2. Объём и надёжность: material/outward/cavity roles, общий intersecting-NURBS
   Boolean, global embedding/self-intersection, near-gap healing и closed-solid
   admission. Локальные UV-секторы сами по себе не дают готового Body.
3. Freeform offset/replace: расширение опор за исходные charts, self-trimming,
   произвольные trim/periodic/pole области, изменение топологии при collapse.
4. Fillet/chamfer/Full Round: некруговые пространственные spines, variable-radius
   networks, пересечения нескольких blends, higher-valence corners и reblend
   на общих криволинейных опорах.
5. Quad и constraints: nonprojectable/non-disk reconstruction, CAD-патчи около
   extraordinary vertices, сохранение формы/авторских ограничений при edits;
   общий совместный поиск UV/shape/weights/basis и nonlinear equilibrium.
6. Pattern и история: перенос внутренних полостей, новые seams между опорами,
   остальные семейства операций, dependency/branch merges, ModelGraph integration
   и миграция сохранённой параметрической истории.
7. Обмен и итоговая проверка: сохранение исходных геометрических связей при
   roundtrip, расширенная проверка конвертеров, общий актуальный regression/
   architecture gate и интеграция библиотечных результатов в основной pipeline.

Это порядок работ, а не утверждение об отсутствии всех операций в каждом
семействе. Поддерживаемые частные случаи уже реализованы и перечислены ниже.
Полный исходный scope не сокращается до текущего этапа.

## Current implementation slices

| Workstream | Implemented native slice | Remaining before general parity |
| --- | --- | --- |
| Moving-boundary replacement | `feature_ops::retrim::replace_region`: infer adjacent supports and removed sectors from original incidence; original-root curved network cutting, preserved supports, admitted closed cap | Support extension beyond original charts; all contact branches, complex holes/regions, global embedding admission |
| Closed curved-support fillet/chamfer | `profile_blend`: closed rational-profile rounds/chamfers; `edge_transition::insert_radius_patch`: four-face original topology plus whole radius/radial-normal envelope; `round_straight_edge`: automatic straight rounds including curved ends; `round_circular_edge`: automatic outer/inner circular-spine rounds on original plane/concentric-wall supports; `chamfer_straight_edge`: asymmetric straight chamfers; `chamfer_circular_edge`: asymmetric outer/inner circular chamfers with complete closest-distance bounds on saved rails | General noncircular spines/support pairs beyond the admitted circular family, intersecting multi-edge blends, variable-radius body networks and higher-valence corners |
| Quad topology editing | `quad_surface::editing::reconcile`: join complete original boundary cycles; `delete_strip`: delete a connected quad strip/ring with explicit original-to-result maps and full manifold validation; `delete_vertex` / `delete_vertices`: disk-star deletion and exact original-coordinate projected quad reconstruction, retained internal controls via admitted quad splits, immutable maps and replay | Nonprojectable/non-disk regions, reconstruction requiring additional controls; finite CAD patch coverage of extraordinary neighborhoods; constrained shape preservation |
| Full Round / reblend | Full Round on rigidly placed cuboids and proven XY extrusions with retained nonrectangular/rational rear profiles and through-holes behind an isolated replacement strip; grouped coplanar continuous side/end chains on XY extrusions with original selection ownership; recognized planar-support constant-radius reblend | General curved support groups and face networks, strip interference resolution and radius/shape changes throughout a blend network; reference runtime comparison |
| Freeform offset / hollow / thicken | Retained normal-offset expressions and native proposal/regularity owners; `surface_offset::certify_candidate`: whole-original-domain rounding-inclusive normal displacement; `shell_thickness::planar_network`: closed coplanar rational networks with original seams/holes, T witnesses and projective clocks; `freeform_network`: automatic/authored nonplanar caps, original-UV continuous offset admission and shared closed sewing; `miter_network`: intersected planar bilinear rational support networks at creases and multi-support vertices, whole-cap signed support-distance bounds; `mapped_network`: independently based/trimmed caps with explicit affine original UV windows, continuous offset-jet residual bounds and native aligned ruled walls; r54 `quadrilateral_network`: authored bilinear original-UV cap regions and shared closed-network admission (31 affected native tests passed) | Automatic general curved-support branch/window discovery and extension, arbitrary trimmed/periodic/pole source propagation, self-trimming, topology changes at collapse and globally embedded closed-solid shell admission |
| Feature pattern | Original connected face-region volumes, named affine occurrences and retained Boolean lineage; up to 16 rim cycles with disjoint/nested caps on original support charts or certified planar groups; distinct charts require actual trimmed-image separation | Inner-cavity transfer, closure requiring new seams across different supports, geometric active edge-subrange normalization and arbitrary native Boolean pairs |
| Shared edge refinement | `edge_partition`: common stored 3D segments, independently split original face pcurves, shared cut vertices, original-geometry/lift checks, exact partial authored relation ownership, stable Split lineage, periodic supporting-face seam charts, closed periodic/ordinary 3D edges over one original cycle, free singleton-wire seam relocation, full authored seam clocks, exact partial-clock retention through a two-edge cycle, repeated shared multi-edge cycle seam editing with explicit traversal directions, admitted cycle compaction with connected exact affine authored-clock unions, original-bound affine world-clock normalization with independent UV clocks, full-lap exact-planar transverse homothetic quotient faces with owned seams/cone poles, native-constructor cone patch editing with exact dual/three-ray projections, strictly internal trim holes and immutable replay | Material-removing active subranges, general/seam-crossing-hole full-period quotient faces, unproved exact closed-UV decomposition, compaction of disconnected/nonaffine authored-clock unions, moving connected seam vertices, history beyond Split/seam/compact/reclock and general solid embedding |
| Saved refinement history | `edge_partition::history::History`: owned root, ordered native steps, expected actual models/IDs/exact relation maps, strict caller-budget import replay, atomic append, semantic parameter regeneration/root rebasing and prefix branches/undo; `history::sequence::Sequence`: mixed Split/seam/compact/reclock commands, strict `/2` replay and `/1` migration | Other operation families, general dependency-graph edits/branch merges, existing product ModelGraph integration and arbitrary schema migration; plain Model encoding has no history envelope |
| General constrained surface solver | Fixed-basis/weight hard point/jet constraints; complete original mapped-boundary G2; `constrained_surface::network` whole rational G0/G1/G2 curve networks, world-tensor Newton curvature constraints, boundary two-jet locks and immutable replay; `constrained_surface::rational` bounded positive-weight homogeneous proposals with full original-geometry admission; `constrained_surface::unparameterized` infers candidate/support UV curves and refines their bases before original G0/G1/G2 admission; `constrained_surface::adaptive` elevates surface degrees/inserts local knots, including original boundary replay; `constrained_surface::variational`: coupled reference-metric membrane/bending energy, pressure/traction/point loads, hard equations and original G2 locks; `network::loading`: original whole G0/G1/G2 curve networks plus loads, final actual-metric equilibrium and independent replay; `adaptive::loading`: fixed/positive-weight degree/knot adaptation with ORIGINAL physical rest and original G2 boundaries; `rational::loading`: admitted physical merit search with positive boxes/gauge/masks; `unparameterized::loading` and adaptive inferred-positive paths: combined UV/weight/basis construction and full original replay | Global inverse/basis-branch discovery and optimization, authored-control constraint transfer across basis edits, arbitrary projective gauges, nonlinear material/follower forces, joint UV/shape/weight stationary optimum and broad nonlinear feasibility |

XNURBS/Bodyshop/converter whole-library parity is not established by native
symbol inventories; keep their observed entry points and unknown behavior
separate. Existing primitive, NURBS, Boolean, exchange and healing modules must
not be listed as wholly absent merely because one general workflow is missing.

## Newly authored contracts

### `feature_ops::retrim::replace_region`

Input: one closed source body, distinct connected original region faces,
replacement chart, positive bounded work. No neighboring face list, removal UV
seeds or caller-positive root/closure receipts are accepted. Original region
incidence discovers the neighbors and chooses exactly one removed sector per
support. The old rim must survive unchanged through the private partition.

The face-cut owner retains shared original roots and rechecks actual stored
cuts. The region replacement owner admits trims, chart injectivity and complete
continuous closure. Final roots and neighboring source identities are mapped to
the final compacted model. Every retained support carrier remains byte-identical.
Failures publish no partially cut model and leave the source unchanged.

The owner still requires one certifiable original-root fiber per adjacent face.
Root-boundary coverage is not a full hidden surface-intersection branch census.
The returned closure/ownership audit is not a global embedding certificate.

### `feature_ops::profile_blend`

`round_corner` solves the two ORIGINAL rational planar offset-curve equations
with analytic derivatives and bounded descent. It trims the supports and
authors a rational circular bridge with endpoints taken from the actual retained
trim coefficients. Retained curves need fresh continuous bounds against the
original restrictions; no tolerance welding substitutes for these checks.

The original extrusion owner rebuilds all sides and both cap loops as a closed
body. The actual authored blend side needs a `moving_radius` certificate;
both complete longitudinal joins need original-pcurve G1 and regularity proof.
Full stored boundary agreement and the existing solid incidence audit are
mandatory. Rounded construction formulas do not authorize their own results.

`chamfer_corner` uses certified inverse arc-length queries and fresh measured
original head/tail setback acceptance. It authors a straight profile bridge and
the corresponding closed side/cap topology. G1 is intentionally not required
for the chamfer's sharp joins. These APIs admit convex junctions of retained
XY prisms, including curved rational supports, not arbitrary 3D solid edges.

### `feature_ops::edge_transition::insert_radius_patch`

Input is a named closed body, one original sharp edge, an authored transition
surface, four CCW original UV contour curves and center/radius laws. Geometry
is supplied explicitly; no caller certificate or removed-sector seed is trusted.
The two original supports and two distinct valence-three end faces are inferred
from edge/vertex incidence. Periodic edges and higher-valence ends are refused.

Each actual patch boundary is lifted and pulled back onto its original face.
Four corner contacts split the original shared support/end edges, including
both independently located face-use parameters. Stored corner coordinates
come from those shared splits; fresh complete patch lift checks follow.
All four original faces are cut privately. Sharp-edge/vertex incidence selects
the discarded strips. Region replacement then retains the other original
carriers, removes the original sharp edge and admits the actual closed trims.
The result owns source identities, final shared corner roots and fresh actual
support/transition G1 reports, a whole-carrier center-distance radius-law AND
radial-normal envelope certificate, and a solid incidence/sewing audit. Failure
publishes no model.

This admits a non-prismatic curved-end example and its arbitrary rigid placement
and transposed patch chart. It is not an automatic general fillet generator
or a global embedding/self-intersection proof.
The topology path may refuse original face networks that its cut owner cannot
certify. It does not extend original support charts or create corner networks.

`curve_pullback` now proposes rational UV curves on affine projected graph
charts while preserving the original curve clock, knots and weights. Full
original lift admission remains mandatory after chart bounding. G1 normal
bounds preserve original homogeneous coordinate correlation using an interval
shear before differentiation; sampled frames only propose the shear and cannot
authorize a normal. Opposite orientation and curved normal ranges remain covered.

### `feature_ops::edge_transition::round_straight_edge`

The caller supplies only the original body, selected edge, radius and finite
limits. Original incidence discovers both supports and both end faces. Signed
support-plane proposals and the straight edge propose a circular cross section
and an axial carrier. An original end-face vertex UV seeds bounded Newton
cap/ray intersections. Rational collocation with circle-weight powers proposes
both compact world curves and skin pcurves (degrees 3, 5 or 7). Every accepted
candidate needs a complete original skin lift and complete original end-face
pullback. Fit samples, convergence and cage plane residuals authorize nothing.
The same four-face owner then splits actual original shared edges, cuts the
original faces, removes the sharp edge and admits actual G1, radius and closure.
Failed attempts consume the shared proof-cell allowance; no partial model is
published. Original support/end carriers and identities remain available.

`region_replacement::cap_region_with_pcurves` preserves these supplied original
UV clocks instead of numerically reconstructing them on the cylinder. Proposals
must cover the incidence-derived rim exactly once in each actual forward edge
clock. Shared UV endpoint edits receive a new whole lift check, and the existing
original trim/closure admission still follows. Missing, duplicate, extra and
incorrect interior curves refuse; matching endpoints alone never suffice.

Verified geometry includes a non-prismatic quadratic end with nonconstant
contact height, three radii, an arbitrary rigid placement and a nonright support
angle. An independent volume formula checks the nonconstant-height cut.
This constructor does not solve curved spines/nonplanar support pairs, extend
original charts, handle concave or higher-valence corner networks, discover all
hidden contact branches or certify global embedding. End proposals outside its
finite rational family remain an explicit refusal, not sampled acceptance.

### `feature_ops::edge_transition::round_circular_edge`

Input is the original named body, one original edge, radius and finite limits.
Original edge/vertex incidence discovers the two supports and two ends. One
support must propose a plane; an original shell-oriented wall normal proposes
outer versus inner contact. A three-point circular frame proposes a rational
toroidal carrier and center path directly from the ORIGINAL edge coefficients,
retaining its knot basis/clock. No axis, neighboring face list, rails, patch,
UV curves or removal sector is supplied by the caller. Recognition samples
never certify circularity, contact or a result body.

The existing four-face owner must freshly prove every generated contour lift
on the original carriers, original shared roots and both face-use parameters,
original face cuts/removal incidence, closed saved trims and actual support G1.
Whole-carrier radius AND radial-normal sphere-envelope agreement are mandatory
on the ACTUAL saved patch. Outer and inner wall rounds, an oblique 60-degree
sector and independently integrated removed-volume laws are verified. Invalid,
collapsing or unresolved frames/radii and exhausted work publish no partial body.
General noncircular spines/support pairs, periodic or higher-valence end networks,
general end-contour discovery and global embedding remain open. The separate circular chamfer constructor below provides distance-controlled
sharp transitions for this admitted family.

### `feature_ops::edge_transition::chamfer_circular_edge`

Input is the original named body, one circular edge, two ordered distances
`[planar radial setback, wall axial setback]` and finite limits. The circular
family frame/contact proposals share the original-incidence constructor used by
rounds. A rational ruled carrier bridges the two proposed rails. Whole original
contour lifts, both original face-use roots, original face cuts/removal incidence,
closed saved trims/naming and actual solid incidence/sewing audit remain mandatory.
No caller-authored rail, source support list or trimmed-face certificate is trusted.
This sharp chamfer makes no G1 or sphere-envelope claim.

Every ACTUAL saved rail receives a complete nearest-distance interval relative
to the ENTIRE ORIGINAL selected edge image. An independent full matched-curve
law supplies the upper bound: every saved point has a paired original point.
This alone is not nearest-distance authority. On the planar side, original and
actual point-to-center radius certificates plus reverse triangle inequality give
a lower bound for EVERY possible point pair. The difference of authored radius
constants is enclosed outward, including rounding in `R +/- setback`. On the
wall side, positive-rational original/actual projected control hulls, divided
by a freshly enclosed normal length, give another universal lower bound. Each
full resulting interval must lie within model tolerance of the requested setback.

Immutable certificates retain actual/original curve snapshots, matched clock
orientation, radial or plane lower-bound evidence and saved edge index. Geometry
recognition and endpoint orientation tests only propose candidates. All algebraic
checks share the topology owner's original finite work allowance. Verified cases
include unequal setbacks on outer and inner sector walls, independent original
edge parameter domains, oblique placement and independently integrated triangular
removed-volume formulas. Elliptical stock, overlarge offsets, invalid inputs and
exhausted work publish no partial body. Noncircular spines, general end contour
search, partial original-arc correspondence and higher-valence networks remain open.

### `moving_radius::curves::qualify`

Certifies a constant full matched distance between two ORIGINAL rational curves,
with independently normalized natural domains and optional opposite traversal.
Original curve definitions are retained unchanged. No rounded knot remap, trim
or reconstructed Curve replaces either original. Original source knot fractions
only propose split locations. At every shared clock cell, ALL incident original
span combinations participate, including outward halos around rounded split
fractions. Their original Bernstein nets are affinely composed with outward
endpoint intervals; a neighboring chart's polynomial extension is never clipped
or selected heuristically. If its extension has an unproved denominator outside
its actual subset, subdivide instead of dropping that original branch.

Correlated homogeneous distance residuals bound the complete cell. Point
witnesses can refute but never admit. Immutable results replay the complete law
on the retained originals. Degree, curve controls, cells and polynomial work are
bounded; unknown coverage/exhaustion yields no certificate. This is explicitly
a MATCHED distance, not nearest-point/Hausdorff authority in isolation; the
circular chamfer owner supplies the additional universal lower bound above.
Regressions cover rational circles on reversed/nonunit clocks, independently
remapped multispan originals, zero distance, hidden quartic deviations, immutable
source ownership and exhausted allowances.

### Correlated original radial/normal and seam evidence

`moving_envelope` now partitions all ORIGINAL surface/center knots, then bounds
`N × D` using outward Bernstein arithmetic BEFORE interval hulls, where
`N=(W X_u-X W_u) × (W X_v-X W_v)` and `D=X W_c-C W`. Fresh original jet
regularity, positive homogeneous denominator lower bounds and the owned radius
relation supply the angular denominator. Every cell and incident knot side must
pass; singular poles and exhausted cells/spans/polynomial work refuse. A point
witness can disprove but never admit. Radius-law positive weights are mandatory
for its control-hull minimum used by the conservative distance/error bounds.

`surface_g1_pcurves` adds shared-clock original homogeneous normal polynomials
on original single-span charts/pcurves. Degree/budget checks limit this optional
route; other layouts retain general interval subdivision. Their cross-product
hull bounds angle, a signed dot-product hull fixes one coherent orientation,
and fresh nonzero normal hulls prove regularity. Complete original spatial
position ownership remains independently mandatory. No rounded normal curve,
primitive classification or endpoint matching is authority. `Report::work()`
includes original position predicates AND normal-polynomial work; network,
profile-blend, sweep and edge-transition callers share this combined allowance.

Regressions cover curved rational tori, independent nonunit clocks, oblique
placement, correctly shifted variable-radius characteristic circles, independent
surface/center knot insertion, both seam orientations and both traversal clocks,
and radius-correct tilted sections that are NOT an envelope. This is the stated
tolerance-based radius/radial-normal relation, not a separate derivative-error
bound on an authored variable-radius law, a union-of-balls boundary, global
embedding, wall thickness, or general closed-fillet network qualification.

### `feature_ops::edge_transition::chamfer_straight_edge`

Input is the original body, sharp edge and two distances, ordered by ascending
original support-face index. Each distance is perpendicular to the original
straight edge carrier inside its planar support. Signed original support frames
propose the two offset rails and ruled chamfer carrier. The same bounded
original end intersections and rational proposals admit curved end contacts.
The four-face topology owner trims the original supports and end faces, retains
their original carriers and removes the selected sharp edge atomically.

The common closed topology owner is separated from radius/G1 qualification.
The existing round API retains its inherent methods and mandatory G1/radius
checks. Chamfers instead require fresh setback bounds for every ACTUAL saved
contact curve. They do not inherit a fillet radius or tangent-plane receipt.

`SetbackCertificate` owns snapshots of the original edge, actual saved contact
curve and original endpoint-defined straight carrier. Orthogonal projection of
every original Euclidean control point is evaluated with outward interval
arithmetic. Positive rational basis functions preserve the complete projected
control hull on every original span. Its norm bounds the whole saved curve's
perpendicular distance. Original edge-to-line representation deviation must be
within one eighth of source tolerance and is included in the final distance
error. This is a straight-carrier setback, not an arc-length query or distance
to an arbitrary finite curved edge. No fitted frame or endpoint sample admits it.

Verified cases include unequal setbacks on a non-prismatic body with a
nonconstant-height quadratic end, independent removed-volume integration,
arbitrary rigid placement and nonright support angles. Invalid and excessive
setbacks, insufficient work and a correct-endpoint/wrong-interior rational
control curve are refused. Curved spines, nonplanar support pairs, support
extension, multi-edge/corner networks and global embedding remain open.

### `constrained_surface::network`

Original spatial curves, authored UV pcurves and optional original support
surface traces specify whole G0 or tangent-plane/G1 constraints. Each clock
has an explicit orientation and its own original parameter domain. Initial
knot/midpoint sites only propose fixed-weight/basis control coefficients.
Every actual result must pass full original curve/surface agreement, or full
original-support pcurve G1 plus position and regularity admission. Witnesses
and interval midpoints refine failed proposals within one shared bounded site,
iteration, interval-cell and exact-work allowance. Unresolved evidence, rank
ambiguity, incompatible crossings and exhaustion publish no Qualified object.

The numeric equality solve now uses pivoted twice-reorthogonalized QR after
Cholesky whitening of the anchor/thin-plate energy. It avoids the previous
normal-equation conditioning loss on rational guide networks. Actual geometric
constraint checks remain mandatory; numerical rank is not a certificate.
An independent analytic tensor-eigenmode solution checks the energy minimum.

`solve_preserving_g2` combines the complete curve network with original
boundary 0/1/2-jet coefficient locks and fresh complete boundary G2. The returned
owner stores the actual surface, immutable original guide/support snapshots,
proofs, sites, hard constraints and optional original boundary owner. `reverify`
rechecks every whole guide AND all requested original boundary seams under a
new shared allowance. A plain solved Report is not whole-curve authority.

Verified cases include a hidden between-site quartic deviation, a rational
circular pcurve, crossing rational guides with fixed nonuniform weights,
curved-support normal fields in all orientation combinations, nonunit domains
and combined four-boundary G2 plus an interior guide. This is fixed-basis/weight
known-UV fitting, not an unknown-UV or variable-weight nonlinear XNURBS solver.
Variable weights and inferred UVs now have separate rational/unparameterized
owners below. Bounded surface adaptation is provided separately below; physical
variational forces and global surface embedding/solid admission remain unfinished.

### `constrained_surface::network::solve_with_curvature`

Original support traces can now request geometric G2 throughout an interior
curve, mixed with the existing G0/G1 guides and optional original boundary locks.
The support's world-coordinate signed curvature tensor is evaluated only to
propose nonlinear equations: `n_ref · S_ij = S_i^T B_ref S_j`. This handles
independent tangent speeds/shears without copying source parametric second
jets or accepting a caller-supplied UV Jacobian. The initial solve establishes
position/tangent feasibility; subsequent bounded Newton steps linearize these
quadratic equations using scalar first/second-jet rows. The anchor/thin-plate
objective remains tied to the original seed. Fixed weights/basis are preserved.
Numerical site checks only steer Newton versus new sites; they never admit G2.

Every result needs fresh complete original 3D-curve position agreement and
whole original pcurve G2, with one coherent normal/tensor orientation branch
and regularity over the complete curve. The immutable owner retains G2 source
snapshots and can replay all G0/G1/G2 curves AND every requested original
boundary seam. Conflicting curvature fields, hidden between-site differences,
unknown proof and exhausted work return no Qualified patch.

A common-basis proof route correlates original homogeneous coefficient
DIFFERENCES before differentiation. It retains the common original rational
denominator and bounds normalized world normals/tensors through coupled jets.
It requires identical knots, degrees, weights and authored pcurve clocks,
positive weights and C2 continuity across every crossed original knot. No
rounded difference surface, snapped candidate or borrowed source receipt is
used. Other mappings retain the existing general original-chart inspector.

Verified cases include a G0/G1-compatible curve with incorrect transverse
curvature; unequal UV tangent scales; nonuniform rational weights; opposite
support orientation; all clock directions; combined four-boundary G2; a tiny
high-degree coefficient perturbation; and a quartic curvature discrepancy
hidden between three initially matching G2 sites. Altered rational weights
cannot inherit common-basis evidence. Nonlinear convergence is bounded, not
universal. This API retains authored UVs and weights; the separate owners below
add bounded weight/UV proposals; the adaptive owner adds surface basis edits.
Physical force functionals, broad branch handling and global embedding/solid
qualification remain unfinished.

### `constrained_surface::rational::solve`

Jointly proposes homogeneous Cartesian coefficients and positive rational
weights on the authored knot basis. A bounded working-set equality/box solve
uses the original homogeneous anchor and discrete thin-plate objective; an
original weight is held exactly to remove the projective scale gauge. Positive
weight bounds are mandatory. Point/control constraints and guide tangent-plane
rows are homogeneous equations. First/second scalar jets and world-curvature
equations use bounded Newton proposals; an infeasible linearization may fall
back to position/tangent construction, while final hard jets remain mandatory.
This is a construction strategy, not a global nonlinear energy certificate.

Each proposed weight vector is passed to the original fixed-weight network
solver, which must admit complete original G0/G1/G2 curves and all hard jets.
Optional original boundary two-jets pin all four homogeneous components of the
three boundary control layers; interior weights may vary. Actual saved geometry
must freshly pass every requested original boundary G2 proof. The immutable
result retains the seed, gauge, positive box, original network owners and replay.
Failed nested proof attempts consume their whole reserved shared allowance;
unknown proof, exhausted work or conflicting constraints publish no geometry.

Regressions cover a circle unattainable on the polynomial fixed-weight basis,
active weight bounds, rational-clock interior G2, a free central weight with all
four original G2 boundaries, scalar hard jets, nonlinear second jets, reversed
nonunit clocks and a nonunit projective gauge. This API takes authored UVs;
the separate unparameterized owner below infers them. Surface adaptation is
provided separately below. Physical force functionals, universal nonlinear
convergence, branch discovery and global embedding/solid admission remain unfinished.

### `constrained_surface::unparameterized::solve`

Input guides are original spatial curves plus optional original tangent/G2
support surfaces. No candidate or support UV curve is required. Bounded local
trust-region inversions use six starts (previous, chart center, four corners)
per numerical sample. Rank-revealing QR fits the original guide rational basis;
alternate attempts refine clamped cubic UV bases with original source breaks.
This can represent nonrational inverse clocks to the requested geometric
tolerance instead of forcing the spatial curve's basis onto its inverse.
The returned clock follows the complete original normalized curve traversal,
including reversal and nonunit source/surface domains.

Original support UVs are inferred/refined independently. Near-coincident inverse
proposals on a common authored basis may reuse a single candidate clock; this
is neither a chart-equivalence certificate nor inherited support evidence.
A sampled surface fit can seed the next inverse attempt, while final surface
energy remains anchored to the immutable ORIGINAL seed. Fixed or positive-weight
construction uses the existing native network/rational owners. All hard jets,
optional four-boundary G2, and complete original G0/G1/G2 guides must freshly
pass on the actual final surface. No local inverse convergence, sampled fit,
UV hull clipping, or borrowed support report authorizes a result.

The immutable qualified result owns original curves/supports and inferred
pcurves through its network owner; replay rechecks every full original trace
and boundary seam. All nested proof calls share finite cells/exact-work limits;
failed calls consume their entire reserved allowance. Inversions, samples,
control counts, local iterations and outer attempts have separate finite caps.
Any unproved trace, incompatible actual hard constraint or exhausted allowance
returns no qualified geometry. The authored surface basis remains fixed.

Fixed Cartesian control constraints are now restored EXACTLY before all final
point/jet/normal checks, so a derivative report computed on pre-restoration
coefficients cannot authorize the saved result. Regressions include a displaced
curve on a nonlinear chart, oblique placement, reversed/nonunit clocks, original
curved-support transverse G2, an independent transposed/opposite support chart,
positive weights with hard derivatives, all four boundary G2 seams, a square-root
inverse with refined candidate AND support UV bases, and a between-site quartic
excursion on locked geometry. General global inverse uniqueness/branch search,
physical variational forces and global embedding/solid qualification remain
unfinished; the adaptive owner below adds bounded surface basis edits.

### `constrained_surface::adaptive::{solve, solve_unparameterized}`

Attempts original full-network admission before changing the authored surface
basis. When the basis is insufficient, bounded proposals elevate degrees in
active trace directions, insert mapped original curve breaks with their needed
multiplicity, then split residual-scored active spans. Hard geometric point/jet
parameters keep their original surface coordinates. Inferred UVs, original
support G2 and bounded positive-weight construction can be combined with the
same adaptation. Original normalized clocks, orientation and nonunit surface
parameter domains survive every edit. A sampled residual is only a proposal
score; it never admits a patch or chooses a globally optimal basis.

Each edit is applied to the immutable seed's current representation, not to an
already deformed fitted patch. The final owner retains the ORIGINAL seed, the
refined fitting anchor and explicit degree/knot edit history. Homogeneous edit
algorithms preserve geometry in real arithmetic; this grants no binary64 error
certificate, original-seam identity or global physical-energy equivalence.
Original indexed control constraints are not silently reassigned to new poles:
requests needing refinement with such constraints refuse. Positive-weight
adaptation currently supports the original corner gauge `[0,0]` only.

Optional original G2 boundary references are refined for construction, but an
additional full ORIGINAL boundary proof is mandatory. A spatial isocurve only
proposes a common carrier: the native pcurve inspector independently proves its
complete lift on BOTH the saved patch and the immutable original reference,
with half the position tolerance per side and an outward triangle bound. Normal,
world-curvature tensor, regularity and coherent orientation are then admitted
on both original surfaces. Replay repeats original seam checks as well as all
inner original guide/refined-boundary checks. All layers share finite cell and
exact-work allowances; failed inner calls consume their entire reservation.

Verified cases include a hidden quartic needing degree growth, C2 and C1 local
piece boundaries needing different knot multiplicities, reversed independent
nonunit clocks, transverse world G2 needing a new basis direction, inferred UVs
with positive weights, four ORIGINAL boundary G2 seams across degree 1/3-to-6
adaptation including nonuniform rational weights, and rejection of a rounded basis edit's false zero-error original
boundary identity. Malformed guides, incompatible crossings, insufficient
controls/steps and ambiguous old control indices publish no geometry.

The finite search is not a universal feasibility solver or a global basis/branch
optimizer. General authored-control transfer, arbitrary gauges, physical forces,
all inverse branches and global embedding/closed solid qualification remain open.

### Quad cage edits

Boundary reconciliation uses explicit complete boundary-cycle correspondence
and a caller-supplied control-pair displacement allowance. Result orientation,
edge manifoldness and all vertex fans are checked. No nearest-point pairing or
silent orientation reversal is performed.

Strip deletion requires a single connected nonbranching strip/ring and explicit
opposite collapse edges per original quad. Vertex components may merge only
pairs. Every retained face must remain a quad, and the whole resulting cage
must validate. Result maps retain original vertex/face identities by index.
Both edits can change the old limit surface. A control displacement report is
not a bound on old-to-new surface error and does not certify global embedding.

## Verification commands

Current combined regression result (r24): **262 tests passed in 50 integration suites**, plus **2 existing affine-chart owner unit tests** (264 fresh distinct tests). Focused trim regressions are included once. Historical selected units remain separate: **25 operations (r21), 10 subdivision (r13), 7 G2 and 16 moving_ (r12)**; these 58 unit tests were not rerun for r24. The older setback unit regression remains separate historical evidence.
Commands, bounded scope, source hashes and outstanding architecture diagnostics:
[native evidence](../qualification/native-library-parity-2026-10-31/results.json).

```sh
cargo test --manifest-path crates/Cargo.toml -p brep-core --test retrim_replacement --test curved_profile_blend --test edge_transition
cargo test --manifest-path crates/Cargo.toml -p nurbs-core --test curve_pullback_graph --test surface_g1_pcurves --test curve_network_constraints --test curve_network_g2
cargo test --manifest-path crates/Cargo.toml -p subdivision-core --test quad_editing --test quad_surface
python3 crates/independent-modeling/verify_clean.py
```

The original architecture and API ownership gates remain mandatory. Do not
relax baselines or assign unrelated concurrent modules merely to make them pass.

Next work: extend automatic construction beyond circular plane/wall contacts,
extend chamfers to noncircular spines, then owned higher-valence end/corner transitions. Multi-branch discovery and support extension must remain explicit
refusals until their original-geometry evidence is actually implemented.

### Quad control deletion and reconstruction

`QuadSurface::delete_vertex` / `delete_vertices` retain the original source and
remove exactly the selected controls from a connected disk of incident faces.
A bounded interval dynamic program proposes quad tilings of the remaining even
boundary. Exact original-coordinate projected orientation/direction/area
predicates independently admit boundary simplicity, diagonals and saved fill.
No rounded projection inherits original geometric authority.

Every other control remains bit-identical. Actual saved source face/edge maps,
replacement group ownership, manifold/fan validation and original Euler,
component and boundary-loop counts are checked; immutable replay repeats the
original region and actual saved fill checks. Original unselected interior
controls now remain in the reconstructed disk: strict original projected
containment proposes quad splits, and each saved active disk receives the full
independent check before publication. No pending original control may hide
along a new chord. Boundary parity, disconnected or non-disk regions,
unresolved insertion and exhausted budgets refuse without changing the source. Regression cases include valences 3/5/6/8,
connected interior selections, paired boundary deletions, an oblique closed
cube, multiple retained internal controls, outside/coincident internal controls
that must refuse, and thin original geometry erased by rounded projection.

This proves a projected control-polygon fill and retained combinatorial
topology. It does not preserve the old limit surface or establish global limit
embedding, volume, G2 shape, arbitrary extraordinary CAD coverage or universal
quad reconstruction. Native no-codec use includes `cad-predicates` with default
features disabled.

### Reference-metric variational shell forces

Owner: `surfaces::constrained_surface::variational`; the registered surfaces
parent owns this public child. Fixed original rational basis/weights and
immutable original geometry define the reference metric, area, tangent frame
and Christoffel connection. Displacement membrane strain is the symmetric
tangential gradient. Bending is the linearized second-form variation
`n dot (U_ij - Gamma_ij^k U_k)`. Isotropic plane-stress material couples Cartesian
components with Poisson terms. Both membrane and bending stiffness may be
chosen independently. A positive Cartesian control-displacement anchor removes
rigid-mode ambiguity. This path requires zero legacy index-space fairness.

Every original nonempty knot span participates in bounded tensor Gauss
quadrature. The trial space must be C0 for membrane energy and C1 when bending
is nonzero; unresolved concentrated crease energy is refused. Dead pressure follows ORIGINAL normals, uniform traction is per
ORIGINAL area, and original-UV point forces use the actual rational basis.
Dense Cholesky plus rank-revealing whitened hard-equation correction proposes
an equilibrium. Actual exact controls are restored before checking every hard
point/jet/tangent constraint and the projected free-gradient residual. The
immutable result retains source/model/constraints, actual geometry, energy/work
diagnostics and fresh assembly/equilibrium replay. Optional three-layer locks
require fresh complete original G2/regularity proofs on the saved boundaries.

Independent regressions cover analytic biquadratic bending and membrane
solutions, nonuniform-rational point loads, infinitesimal rigid modes on a curved
rational reference, oblique placement and independent parameter clocks, loaded
original G2 boundaries, incompatible constraints, singular metrics and exhausted
quadrature/matrix work. Quadrature energy and stationarity remain numerical
diagnostics; there is no continuous integration-error certificate, whole-chart
regularity or embedding admission, finite-deformation elasticity, follower
pressure, nonlinear material, adaptive physical network solver or solid claim.

### Loaded original curve networks

`constrained_surface::network::loading::solve` shares the existing registered
surfaces/network owner. Original spatial curves, independent candidate/support
pcurves and original G1/G2 supports retain complete-curve admission; physical
proposals cannot replace it with collocation or energy receipts. The original
seed owns the fixed rational reference shell metric and loads across every
iteration. Optional original G2 boundary locks may refer to a DIFFERENT saved
reference with compatible basis/boundary weights. Boundary admission checks
the saved physical result without solving again or changing any coefficient.

After whole-curve admission, G2 tangent equations are regenerated on the actual
final geometry, and fresh hard-equation/energy/nullspace-gradient admission is
mandatory. A previous Newton metric cannot authorize equilibrium. Unresolved
final equilibrium triggers bounded relinearization; exhausted iterations or
shared assembly/cell/exact-work allowances refuse. Assembly terms are shared
across every proposal and final admission; failed final attempts consume the
known original-metric assembly cost.

Replay derives the current tangent equations again from immutable ORIGINAL
curve/support snapshots, reassembles physical energy, and checks actual hard
constraints and equilibrium. Whole original curve/locked-boundary proofs use
one shared geometric allowance. Quadrature diagnostics stay numerical, and
stationarity applies to the resolved finite-dimensional final tangent system;
continuous energy error, global nonlinear constrained minima, inferred UV or
adaptive/positive-weight physical fitting, nonlinear materials/follower loads
and global embedding/solid admission remain open.

Regressions include a hidden quartic requiring additional sites; simultaneous
loaded original G1/G2 fields; actual-metric curved G2 equilibrium beyond early
geometric tolerance; independent physical rest/original G2 boundary sources;
shared assembly-budget exhaustion; immutable input replay; and nonuniform
rational weights with reversed independent natural parameter clocks.

### Loaded basis adaptation on original physical rest

`adaptive::loading::solve` belongs to the existing registered surfaces/constrained
owner. Fixed-weight degree/knot proposals retain one immutable ORIGINAL shell
reference. The refined trial/regularizer anchor is separate; its rounded basis
or changed coordinates never inherit physical rest identity. Membrane, bending,
normal pressure and original area/connection use the original reference. Trial
rational jets provide unknown coefficients. Affine strain offsets and load-work
constants explicitly retain the ORIGINAL reference when anchor and rest differ.
Tensor quadrature includes the union of original and trial nonempty knot spans
on identical explicitly authored UV domains. No chart map is inferred.

Each basis attempt uses complete original curve G0/G1/G2 admission, actual final
loaded equilibrium and optional refined-boundary construction, followed by
independent complete ORIGINAL boundary G2 admission. Failed attempts consume
their reserved geometric/exact/assembly allowances; successful attempts consume
actual owned work. One finite assembly budget covers basis attempts as well as
all nested Newton/proposal/final admissions. Construction and replay bind the
actual energy owner to the unchanged original reference and actual trial anchor.
Authored control indices are never silently reinterpreted after basis edits.

Regressions independently integrate affine membrane/bending energy and original
load work on a DIFFERENT trial basis, require both original/trial knot partitions,
force degree growth and local knot insertion, and retain four ORIGINAL G2 seams
with rational source weights across degree 3-to-6 growth. Invalid UV domains,
resource exhaustion, conflicting data and unsupported control-index transfer
refuse without publishing geometry. The control anchor remains an explicitly
basis-dependent numerical regularizer; no basis-independent global energy
optimum, continuous integration-error certificate, inferred-UV/positive-weight
physical fitting, nonlinear/follower material, global embedding or solid claim
is made. The general library parity goal remains active.

### Loaded unknown-UV and adaptive inverse fitting

`unparameterized::loading::solve` and `solve_with_reference` infer candidate AND
original-support pcurves while retaining the original physical shell reference.
Every accepted inverse round needs the complete original G0/G1/G2 network plus
fresh final-metric equilibrium; sampled inverse/surface proposals confer no
authority. Failed rounds charge their reserved physical and geometric work.
One total assembly allowance covers all UV rounds and nested loaded Newton
solves/final admissions, with immutable physical reference/model and trial
anchor binding repeated during construction and replay.

`adaptive::loading::solve_unparameterized` also elevates degrees/inserts knots,
retaining ORIGINAL physical rest across ALL inverse/basis rounds. Actual owned
work includes unsuccessful inverse reservations, not merely the final network
call. Refined boundary construction still requires complete ORIGINAL boundary
G2 admission before return. Authored control indices are never reinterpreted.

Regressions cover nonlinear material charts, independent transposed/opposite
support clocks, actual transverse G2 under load, adaptive unknown-UV degree
growth, four original G2 boundaries, exact shared budget thresholds, immutable
source/model replay and a square-root inverse that needs further UV basis
refinement without moving locked controls. Equilibrium is for the final inferred
pcurves. No nearest/unique/global inverse, joint UV/shape stationary optimum,
positive-weight physical optimization, continuous energy certificate, nonlinear
material/follower forces or global embedding/solid admission is claimed.

### Positive-weight physical energy and admitted merit search

`rational::loading::solve` / `solve_with_reference` inherit the registered
surfaces/constrained owner. A bounded homogeneous stage discovers feasible
positive weights if the authored weights cannot satisfy original curves.
Every such trial is then refitted under the ORIGINAL physical shell objective,
using the unchanged original Cartesian control anchor and immutable physical
reference. No deformed homogeneous proposal becomes a new physical rest or
Cartesian regularizer anchor. Complete original G0/G1/G2, actual final-metric
equilibrium, optional original boundary G2, the positive box and exact gauge
are mandatory. Original boundary layers and unselected weight indices remain
bit-identical; explicit masks cannot include the gauge or boundary pins.

A bounded coordinate search in log-positive weights admits every trial before
comparing the same original physical potential plus an explicit original-weight
log penalty. Only strict merit improvements are retained. Reports distinguish
trials, independently admitted trials, numerically unresolved trials, accepted
improvements, completed sweeps, final step and exhausted resources/trial count.
A count of weights at bounds does not assert active KKT multipliers. No joint
weight stationarity is asserted, including when the requested step resolution
is reached. An interrupted search returns the best qualified geometry with an
explicit unfinished status. Failed calls consume reserved work; successful
trials consume actual geometric/exact/assembly work from the same totals.
Construction ends with ownership checks, without an uncharged duplicate proof.
Explicit replay freshly verifies original geometry/equilibrium and recomputes
the saved merit from the new energy diagnostics.

Independent regressions include an analytically solved one-weight point-force
objective, a circle unavailable to authored fixed weights, original G2 support
fields, four preserved original boundary layers with a free interior weight,
invalid masks/gauge/boxes/work and interrupted-search immutable input replay.
The fixed authored UV/basis path is implemented; inferred/adaptive positive-
weight physical fitting, joint UV/shape/weight optimality, continuous energy
certificates, nonlinear material/follower forces and global embedding/solid
admission remain open. General native library parity remains active.

### Combined positive weights, inferred UVs and adaptive basis

`unparameterized::loading::solve_positive_weights` (and original-reference
variant), `adaptive::loading::solve_positive_weights`, and
`adaptive::loading::solve_unparameterized_positive_weights` compose the existing
registered surfaces owners. Every inverse/basis attempt retains the full
positive-weight physical owner, including immutable reference/model, exact gauge,
weight mask/box, numerical merit trace and unfinished-search diagnostics.
Successful work includes the entire weight search and all earlier failed inverse
reservations; it is never reduced to the final fixed-weight network call.

Physical rest remains the ORIGINAL seed across basis changes. Cartesian anchors
are refined original representations; their weights may then be varied, so
loaded ownership checks bind the current rational owner's ORIGINAL trial anchor
to the saved adaptive anchor, rather than demanding unchanged final weights.
Explicit replay invokes the positive-weight owner to recheck mask/gauge/box and
recompute merit from fresh physical energy, together with complete original
G0/G1/G2 and ORIGINAL adaptive boundary seams. Fixed-weight replay remains
unchanged.

An explicit original weight-index mask cannot be silently reindexed after
degree/knot edits. Such edits require automatic unpinned selection; noncorner
gauges remain unsupported through basis edits. Regression cases combine
unknown UVs with admitted merit improvements; recover an original circle through
both degree growth and nonpolynomial weights; simultaneously infer UVs, elevate
degree and change weights for a rational quartic; preserve four ORIGINAL G2
boundaries; retain immutable source/model replay; and refuse unsupported index/
gauge transfer and exhausted shared work. The returned equilibrium is for final
inferred pcurves and resolved trial weights. No joint UV/shape/weight stationarity,
global branch/optimum discovery, basis-independent regularizer, continuous
integration-error certificate, nonlinear material/follower forces or global
embedding/solid admission is claimed. General library parity remains active.


### Original face-region feature patterns

`brep_core::feature_ops::region_pattern::{extract, pattern}` owns the original
source, selected face region, closing carrier and named affine occurrences.
Selected surfaces, original 3D edge curves, weights, knots and retained pcurves
are copied directly with their topology IDs; no depth-based reconstruction or
polygonal replacement is used. Additive and subtractive material roles are
explicit. Extraction accepts a connected region in an outer shell without
inner cavities and up to 16 simple rim cycles forming a nesting forest of
disjoint cap components, holes and material islands within proposed carrier
groups. Complete original support wires on matching retained charts reuse their
surfaces/pcurves; otherwise coplanar supports may supply derived planar caps.
Distinct groups require positive 3D separation of their actual trimmed caps. Interval bounds on every original
support pole admit planarity. The cap is separately identified as derived,
never presented as an unchanged original carrier.

Actual saved charts require regularity and injectivity, full original-edge /
pcurve lift agreement and valid trim regions. The rigorous signed-area interval
need only resolve winding sign; convergence of its numerical area approximation
is not an admission premise. Area work, chart proofs, boundary lifts and trim
checks consume shared bounded proof allowances across extraction, transformed
occurrences and final output. Numerical mass is an orientation diagnostic, not
a certified volume enclosure or global embedding proof. The returned solid
audit retains its explicit self-intersection-completeness flag.

One to eight uniquely named occurrences use the existing native Join/Cut owner.
Unsupported Boolean pairs refuse atomically. Instance identity derives from
separate pattern key, occurrence key and original entity ID, retaining identity
through placement edits, reordered occurrences and reordered source face storage.
Actual native Boolean lineage determines surviving descendants; consumed IDs do
not acquire fabricated survivors. Immutable replay reconstructs the original
region and repeats transformations, Boolean operations and final qualification.

Regressions cover an ordinary five-face boss on split coplanar supports,
rational circular boss/pocket Join and Cut, a retained bicubic nonextruded top
with independently known volume, independent nonunit 3D/UV clocks, source and
occurrence reorderings, consumed output becoming empty, invalid selections,
vertex-tolerance transfer, resource exhaustion, duplicate keys and singular transforms. Inner-cavity transfer, geometric active edge-subrange normalization, unsupported
closure requiring new cross-support seams, general Boolean branches and product feature-DAG integration remain
open. Local chart/trim checks do not prove global solid embedding. General
native library parity remains active.


### Multi-rim feature caps and orthogonal Boolean boundary coalescing

Original rim traversal now covers every boundary edge exactly once, separates
up to 16 cycles and rejects pinched/open/repeated boundaries. A single retained
support carrier can supply multiple complete original wires with their own
unchanged natural UV clocks. Otherwise the existing original-pole interval
planarity proof admits a common derived plane. Whole-curve simplicity and
pairwise separation plus conservative winding queries now establish a nesting
forest; the earlier r21 single-outer area-dominance proposal is superseded.
Even-depth cycles produce separate material caps; immediate odd-depth children
are their holes. Each actual saved cap independently passes the full trim-region
audit. Unresolved/intersecting loops or incompatible winding cannot be published.
Orientation, separation and winding work for every cycle share construction
cell, pair and domain allowances with subsequent qualification.

Each derived rim receives an identity based on sorted original edge IDs,
independent of vertex/edge/loop/face storage order. Its provenance names exactly
the original support wires participating in that rim. The cap face and enclosing
shell/body retain their original selected-region dependency. Existing single-rim
closure identity is preserved. Regression cases include two-hole rectangular
boss Join/Cut with preserved material voids, rational annular bosses, original
multiloop support reuse under independent nonunit 3D/UV clocks, all source
storage reorderings, scarce work and an unsupported boss left in edge-only
contact after removing its foundation.

The two-hole Cut exposed unnecessary orthogonal Boolean fragmentation. The
registered native operations owner now partitions exposed integer-grid cells
into disjoint rectangles on each oriented support plane before BRep authorship.
Every rectangle consists solely of present exposed cells, and every such cell
is consumed once. World corners use existing grid coordinates; holes are never
filled and no geometry is fitted. Existing shared-edge normalization sews the
resulting rectangle partitions. The 256-face and coordinate-grid limits remain
unchanged. This is a representation reduction for the existing orthogonal CSG
family, not broader curved Boolean or global embedding authority.

Independent regressions check volume, surface area and centroid of four blind
pockets under every cyclic axis permutation; exact six-face outer/inner box
boundaries with opposite cavity orientation; cavity refill; and retained complete
boundary agreement/solid incidence. General closure requiring new
interior seams across different carriers, inner-cavity feature transfer, geometric active edge-subrange
normalization and arbitrary native Boolean pairs remain open.


### Disjoint and nested material cap components

`FeatureRegion::cap_faces()` returns every closure face in stable original-rim
identity order; `cap_face()` retains compatibility by returning the first.
`region_pattern::cap_components` remains a private child of the existing BRep
feature owner. All original rim curves first require exact joins, whole-curve
simplicity and pairwise strict separation. Conservative winding queries use an
interval enclosure of the actual natural-domain endpoint, rather than assuming
it equals an authored control pole. Every endpoint query consumes the shared
domain budget. Unresolved winding or inconsistent/non-laminar containment
refuses; absence of a proof never means that two loops are disjoint.

Depth and immediate-parent checks establish one nesting forest. Each even-depth
cycle authors a cap component; its immediate odd-depth children are holes.
Original loop orientations must alternate across every parent/child relation.
The original outward signed-area enclosure only determines the required chart
orientation, without an area-order heuristic. All saved cap regions still need
independent trim admission, actual chart regularity/injectivity, complete original
edge lifts and solid incidence checks. This establishes disjoint material regions
on the shared cap chart, not global solid embedding or a certified volume.

Every component/rim identity derives from sorted original rim IDs, separately
from storage indices. Face dependencies name the component's original adjacent
support faces; loop dependencies name its original participating support wires.
Immutable replay now compares the complete saved cap list, constructor geometry,
source maps and original extraction, then repeats final qualification. Signed
area, simplicity, cross-loop separation, nesting queries, retained charts,
original edge lifts and final trim proofs share construction/qualification cells,
pairs and domain work. Exhaustion publishes no partial result.

Five new public-API regressions cover equal-area disjoint bridge feet with
Join/Cut and independently known volumes; a vented ring/central-column bridge
with an island nested inside another cap's hole; two rational circular bridge
feet without faceting; source face/edge storage permutations preserving component
order/occurrence IDs; and independently exhausted cell/pair/domain budgets.
Each group requires a common retained support chart or interval-qualified
coplanar support carriers. Independent/nonplanar groups now require the r23
actual trimmed-cap separation check below. Inner-cavity feature transfer,
closure requiring new cross-support interior seams, geometric active edge-subrange normalization, unsupported
unclamped/periodic exact joins and general curved Boolean branch/contact handling
remain open. Global library parity remains active.


### Independent and nonplanar closure charts

`region_pattern::cap_carrier` proposes shared original charts (including exactly
matching original surface definitions on distinct support faces) or qualified
planar carriers. If no common carrier is proved for all original rim cycles,
a deterministic bounded grouping proposes separate charts. Source rim identity
orders the proposals and final cap faces. An unresolved carrier probe is never
interpreted as proof that the groups are disjoint. Each group independently
receives complete original-curve simplicity/separation, conservative winding,
nesting, hole-role and orientation admission.

`region_pattern::cap_separation` then requires strict positive world-space
separation between every pair of saved material caps from different groups.
A positive rational pole-hull axis gap is sufficient for complete carrier
separation. Overlapping hulls remain unresolved and invoke the existing native
`trimmed_surface_distance` owner on the ACTUAL surfaces and UV trim domains.
Only a finite, positive conservative lower bound admits the pair. Nearest-
distance accuracy convergence is unnecessary for that separation conclusion;
a zero/unresolved lower bound cannot admit the caps. Caps within one chart
retain the nesting/disjointness proof and final chart injectivity check.

Every carrier probe is charged, including rejected grouping attempts. Plane
pole checks, proposed edge-pole projections, whole-carrier hull checks,
cross-chart pair checks and trimmed-distance geometric/domain cells consume
the same construction/qualification allowance. At most 16 cycles imply at most
137 carrier probes; each distance search receives at most 4096 cells from the
remaining shared allowance. Source geometry stays unchanged on refusal.
Original nonplanar support surfaces and their original pcurves are reused,
without fitting a planar surrogate or inventing interior seams.

Six public-API regressions cover distinct parallel foundation levels and
perpendicular floor/wall contacts with Join and independently known volumes;
original bicubic end surfaces under affine scaling; two broad intersecting
complete carrier images whose actual trimmed caps are disjoint; individually
regular/injective graph caps whose images intersect and must refuse; and
cell/pair/domain exhaustion plus storage-order identity stability. The broad
carrier test has known feature volume `16 - 1/2304`, so filling whole carriers
cannot pass by matching a mesh or point witness.

This extends closure construction on multiple original/planar charts. It does
not author a new multi-chart surface across one rim requiring additional seams,
transfer original inner cavities or normalize geometric active edge subranges, or prove global
solid embedding. Cross-cap separation does not certify every cap-to-retained-
face interior contact. General curved Boolean contacts/branches and full native
library parity remain open; original audit completeness flags remain explicit.


### Original-domain trim correspondence transfer

The existing `curve_surface_agreement` owner now exports
`verify_parameter_correspondence` and `verify_parameter_clocks`.
The first consumes literal original-domain scalar nodes, with the 3D edge's
orientation applied exactly once through outward interval reflection. It does
not replace authored values by rounded unit fractions. The existing normalized
correspondence API retains its previous semantics. The second proves transport
between two clocks on the SAME original curve: outward endpoint clock drift
and whole-original-span rational derivative bounds supply a conservative spatial
error bound. Identical clocks are exact identities. Curve validation's C0
interior-join requirement is mandatory; disconnected spans cannot borrow a zero
derivative bound from their separate pieces. Unresolved transport stays explicit.

BRep `boundary_agreement` now sends authored trim parameter values directly to
the original-domain owner and checks periodic endpoint-lift integer range.
Feature cap construction no longer rejects a rim solely for having `CoedgeTrim`.
It verifies original authored uses and their complete source boundary first,
preserves every selected original coedge/trim in the immutable source/tool and
proposes cap clocks in the actual saved cap representation. No interval is
silently sorted to follow a reversed flag. Original author-domain curve clocks
must freshly prove transport to the new nominal curve clock, and the actual cap
pcurve/surface must freshly match the ORIGINAL authored 3D clock over the full
requested relation. These are tolerance-qualified conversions; the original
scalar values/definitions stay unchanged in the retained source, rather than
being replaced by rounded reverse-knot proxies. New periodic lifts belong to
the actual cap chart and are checked for representability.

A trim correspondence subrange cannot establish whole face-boundary coverage.
For every stored trimmed coedge, feature qualification additionally proves the
complete original 3D-curve/pcurve lift independent of the partial relation.
This blocks a sampled-valid piecewise boundary with a hidden mismatch outside
the declared trim interval. Cap clocks are re-proved on actual moved curves and
surfaces after every affine placement, since scaling can amplify nominal clock
drift. The regression exposed an existing affine coordinate-cap repair that
replaced independent pcurve knots by 3D-edge knots. That repair now preserves the
original pcurve clock when rebuilding its UV control points, so literal trim
parameters remain attached to their authored representation. Original-use checks,
full lifts, parameter conversions and occurrence replay all consume the existing
shared proof-cell allowance.

Regressions cover nonunit/descending original intervals, rational quarter-circle
support boundaries, selected-only metadata on derived caps, invalid domains and
lifts, exhausted work, a hidden mismatch outside a valid partial relation,
nondyadic authored values and mirrored/scaled instances. Native tests separately
cover original physical-domain relations in both directions, wrong/partial
relations, budget limits and reflected rational clocks versus discontinuous or
steeply varying original spans. Existing normalized-correspondence and BRep
continuous-validation regressions remain separate fresh checks.

`CoedgeTrim` here denotes the model's authored correspondence relation; it is
not reinterpreted as a newly shortened topological edge. Geometric active-edge
subrange normalization, new vertices/seams, inner-cavity transfer, unsupported
unclamped/periodic exact-loop joins, arbitrary Boolean contacts/branches and
global embedding remain open. This work does not alter the frozen clean crate
or establish full native library parity. Boolean split/merge scalar-clock
attribution on newly authored result edges remains a separate open workflow;
immutable feature and occurrence snapshots retain their original dependencies.

### Shared BRep edge partition (r25)

`feature_ops::edge_partition::{split, split_with_uv_tolerance}` now creates actual
stored curve segments, shared cut vertices and two matching sets of face-local
coedges. One original 3D partition is reused by both incident uses; independent
pcurve clocks are partitioned in their own original domains. Native partition
certificates establish tolerance agreement with the original curves and exact
candidate endpoint joins; they do not assert exact floating-point restriction
identity. Whole saved lifts, local edge/vertex endpoint tolerances, complete trim
regions, chart qualification and solid incidence remain mandatory.

The immutable result retains its source, cuts, allowances and operation key for
full reconstruction. Child edge IDs derive from parent identity and operation
roles; actual Split lineage, modified wire ownership and generated cut vertices
are recorded. Face/body identities persist. Parent local tolerances propagate;
consumed sewing markers are removed without inventing certificates for children.
Full-domain authored trim metadata is remapped in its original direction;
partial relation fragments and periodic lift fragmentation explicitly refuse.

An additional sufficient cavity proof recognizes six exact axis-aligned surface
rectangles with full literal UV boundary coverage and outward orientation,
including subdivided boundary sides. Original positive-weight inner surface/edge
pole hulls must have strict outward-rounded clearance inside this proved box.
An outer AABB alone grants no containment authority. Regressions include inner
and outer cavity edge refinement, rational repeated splits, independent clocks,
stable names, exhausted work, source preservation and rejection of a cavity
inside the bounds but outside the material of a concave outer shell.

This is boundary refinement without material removal. Keeping only an active
geometric subrange, relation-fragment ownership, periodic seams, arbitrary curved
Boolean contacts, general fillet/chamfer corner networks, global solid embedding
and whole-library parity remain open. Frozen clean files remain unchanged.

### Partial authored relation ownership across edge refinement (r26)

The edge-partition result now exposes immutable `AuthoredRelation` records.
Each retains the original source face/wire/coedge and literal `CoedgeTrim` plus
`RelationFragment` records attached to actual child edges. Exact `BigRational`
arithmetic takes the preimage of every child world interval under the original
oriented affine clock. Fragments must form a nonempty, ordered, gap-free,
overlap-free partition of exactly [0,1] in the ORIGINAL authored relation.
Reversed traversal reflects about the original domain, then converts into each
child's independent local oriented clock. Children outside the requested relation
receive no authored trim metadata; partial relations are never expanded into
whole-child requests.

Stored binary64 child clock endpoints are proposals. Their UV clocks may be
converted from the authored parameterization; original scalar values, independent
pcurve definition and original relation remain unchanged in the immutable source
and ownership record. Every fragment proves the ORIGINAL world curve's requested
clock against the ACTUAL saved child pcurve/surface, as well as the new local
curve/pcurve relation. Exact original clock endpoints are enclosed around their
rounded proposals. The existing native correspondence owner supplies a
conditional original-curve derivative bound for that proven scalar error;
outward subtraction reserves this allowance before the remaining lift proof.
The same clock allowance combines with the actual original-to-child curve
partition bound to prove the original requested world clock against the stored
child 3D curve independently. No inferred endpoint match, sampled speed, rounded
scalar identity or increased 3D tolerance substitutes for these proofs. Complete saved boundary admission
still runs independently of partial requests, with one shared finite cell budget.

Regression coverage includes nonunit and descending clocks, asymmetric reversed
child ownership, unrequested children, rational circle arcs, exact reflected
endpoints that are not representable as binary64, repeated partial refinement,
invalid authored correspondence and exhausted work. The fractional-domain
regression exposed a diagnostic `curve_point` endpoint evaluation that could
round outside its active interval; literal normalized endpoints now select the
literal original domain ends. The same regression failed before that correction.

The result wrapper owns original relation history; exporting only its standalone
`Model` does not export those immutable source records. Whole product feature-DAG
integration remains open. Periodic seam/lift fragmentation, material-removing
active edge subranges and general solid embedding remain open. These additions
do not complete the full native library parity objective or alter clean files.

### Periodic face charts and authored lift fragments (r27)

Shared-edge refinement now admits periodic supporting surfaces when each complete
face UV enclosure fits within strictly less than one exact period per periodic
axis. Positive-weight ORIGINAL pcurve pole hulls enclose all authored face loops.
Exact rational period reduction proposes one original rectangle, or two ordered
opposite-shore intervals. Outward endpoint enclosures must remain disjoint across
the omitted lap. Exact original surface seam identity and one common native
contraction on the complete glued rectangle prove injectivity of the entire
admitted face chart, including a face that crosses the actual seam. This does
not assert injectivity of a full periodic carrier. Full original span regularity,
complete saved lifts, actual trim winding and solid-incidence checks remain.

This admission is explicit in the edge-refinement qualification path. The older
region-pattern qualification still requires its prior whole-carrier proof;
periodic chart support is not silently granted to every feature constructor.
UV-pole checks, original seam checks and native contraction spans share the
existing finite proof allowance. Full laps, unbounded sheets, unresolved seams
and exhausted work refuse atomically.

Partial authored relations keep exact original rational fragment coverage and
fresh original-to-child world/UV proofs. Child lift integers are recomputed from
the ACTUAL saved pcurve at each transferred endpoint. One BRep boundary owner
now computes and validates these integers with exact rational division on the
literal evaluated UV scalar and original chart ends. Edge refinement and cap
clock transfer reuse it. Rounded binary64 quotient/floor is insufficient: the
regression with a point immediately below the upper shore previously assigns
the next sheet. Invalid original lifts and unrepresentable integers still refuse.

The existing native periodic-trim owner provides explicit nominal
`wrapped_surface_parameters`; ordinary strict NURBS evaluation is unchanged.
BRep sampled validation uses this method for periodic carriers. Numerical Green
quadrature translates conditioned original knot grids to the actual lifted face
UV enclosure and anchors its U primitive there. Its sampling evaluates periodic
surface coordinates explicitly. Work is charged for the translated grid, with a
finite lap cap; results remain numerical convergence diagnostics, not certified
volume or embedding evidence.

Regressions include a rational cylinder whose shared seam uses different positive
and negative sheets, a single original box face that actually crosses a periodic
carrier seam, exact circle/box volumes, partial relation replay, incorrect
original lift integers, bounded-work refusal, and periodic storage accepted by
rounded validation but refused by the exact seam proof. Native sampling checks
both periodic axes and signed shifts while retaining strict evaluation errors.

Periodic THREE-DIMENSIONAL closed edge partition, full-period quotient faces,
material-removing active subranges, standalone history persistence, general blend
networks and global solid embedding remain open. Original periodic supporting
surfaces are retained; they are not replaced by newly cut proxy carriers. Frozen
clean files remain unchanged and the full native parity goal stays active.

### Closed periodic 3D edge refinement (r28)

The original curve-partition owner now admits periodic originals over one literal
original active period. A single clamped proposal is refined before extracting
all actual segments; every segment retains original physical knot coordinates.
Whole original-function bounds qualify each actual stored piece. Exact cyclic
storage gluing and actual bit-identical candidate closing endpoints are reported
separately from tolerance agreement. If independent clamping rounds the two seam
endpoints differently, the common proposal explicitly shares one candidate
endpoint before qualification; the original curve and stored vertex remain
unchanged. The modified candidate still needs the complete original-curve proof.
No endpoint proposal, constant curve or declared periodic flag grants topology.

A common-cell homogeneous Bernstein residual now supplements the prior interval
jet/anchor bound: restrict BOTH original definitions outward, express them around
one original coordinate origin, and bound A_xyz B_w - B_xyz A_w over a positive
Bernstein product denominator. Independent positive weight gauges cancel. The
minimum of two independently conservative bounds can admit real knot-refinement
roundoff without subtracting two broad derivative hulls of the same function.
Native tests cover rational and uniform cubic cyclic storage, degree changes,
independent weight gauges, modified interior poles, exact original coverage,
work exhaustion and storage that fails exact seam closure.

For topology, tolerance preservation alone remains insufficient. The existing
face-domain owner prepares single-curve loops and literal multi-span UV carriers
only for proof. Original p-fold knots identify exact Bernstein spans; raw source
pole/weight scalars are copied directly without a homogeneous roundtrip. An
actual-definition predicate rechecks exact original-function identity on each
complete span. Other layouts require the existing bounded exact restriction
predicate. Native simplicity/winding then audits the complete prepared loop;
the actual source model, coedges and pcurves stay unchanged. Structural checks
and exact predicate work share the existing trim domain allowance. This also
admits an actual two-coedge result with one long curved segment after only one
cut; the implementation does not secretly add quarter-edge topology to pass.

Proof-backed sewing now handles a genuine periodic edge with one shared vertex.
It requires complete rational authority, exact actual cyclic C0 closure and a
whole-span nonzero tangent proof (a separate fixed allowance of 4096 cells per
closed sewing check). Equal quantized endpoints alone remain insufficient.
Unproved/constant/2D definitions refuse. The generic endpoint-only sew-key API
remains strict. Closed correspondence replay reconstructs the actual curve,
checks its literal clock and rebinds endpoint/context evidence, so mutated
public correspondence fields cannot borrow a stale endpoint receipt. Closed
sewing proves incidence and nondegeneracy, not curve simplicity or embedding.

The complete BRep regression uses two original injective polynomial graph faces
with one shared rational periodic circle edge and one shared seam vertex. It
checks actual source-to-segment geometry, closed incidence, retained surface and
face identities, exact UV region ownership, independent analytic volume pi,
source immutability and replay. Additional cases cover reversed cyclic storage,
nonunit independent partial clocks, repeated child refinement, one-cut/two-curve
results and exhausted original closed-UV proof work. Native geometry additionally
admits uniform cubic cyclic originals without claiming exact restriction identity.

Full-period quotient FACE charts, arbitrary exact closed-UV decomposition,
cyclic seam relocation, material-removing active subranges, persistent exported
original-history ownership, general blend networks and global solid embedding
remain open. Exact source UV identity must succeed; failed identity is not
replaced by a tolerance-only topology claim. The frozen clean version stays
unchanged, and whole-library parity remains active.

### Persisted original refinement history (r29)

With the existing `codec` feature, `edge_partition::history::History` owns one
original root and an ordered chain of native `Split` results. The versioned
`brep-edge-history/1` JSON packet stores that complete root, literal operation
requests, expected actual model/identity snapshots and exact authored-relation
maps. Rational interval fractions are canonical strings; loading compares them
to freshly reconstructed exact fractions rather than parsing them as trusted
proof data. Runtime numerical bounds and success flags are recomputed, not
imported as authority.

`History::split` performs a new native edit. `History::append` can attach an
already computed native result: require its complete source to match the actual
current tip, reconstruct under the remaining caller allowance, and compare the
entire actual model and original ownership maps. The caller's result and prior
history remain unchanged on failure. Original root relations therefore survive
multiple edits, save/load, and continued modeling after reload, including a
closed periodic source edge whose root face has a single authored pcurve.

Import uses strict duplicate-key JSON decoding and a versioned field whitelist.
Canonical root identity tables are mandatory; neither import nor creation
silently applies legacy identity repair. Every step checks the original parent
edge ID, rebuilds the actual native operation, and compares complete saved model
and map data with literal floating-point bit equality (including negative zero).
A saved shape, cut, parent, scalar relation or fractional ownership record cannot
borrow an unrelated reconstruction's green result. This is consistency/replay
validation, not a signature authenticating the sender; a genuinely reauthored
valid root and fully reverified chain remain valid inputs.

Caller policy bounds the whole packet: up to 32 steps, 32 MiB canonical JSON,
128 nesting levels and 2 million values. Cells/pairs/domain proof work accumulate
across the chain. Stored per-step limits must fit the remaining caller allowance;
the packet cannot raise it. Existing native numerical mass allowances remain per
qualification and closed sewing retains its independent fixed cap. Exact byte
measurement precedes encoded-text allocation. Append validates packet size before
publishing a new tip. A root that would change under codec repair is refused
before export, so the writer cannot knowingly produce an unloadable history.

The API is explicit and leaves ordinary `Model` wire encoding compatible:

```rust
use brep_core::feature_ops::edge_partition::history::{Budget, History};
let budget = Budget::default();
let mut history = History::new(split.source(), budget)?;
history.append(&split, budget)?;
let packet = history.to_json(budget)?;
let restored = History::from_json(&packet, budget)?;
```

Ten native integration regressions cover every intermediate model and original
relation map, rational partial clocks, closed periodic roots, continued editing
after reload, attaching existing native results, exact byte limits with Unicode
and escaped keys, negative zero, empty history, wrong/reordered parents,
modified models/cuts/fractions, unknown fields/duplicate keys, missing identity
tables, stricter quotas and atomic failed appends. The closed-periodic geometry
fixture is shared with its original four geometry tests; their scope is unchanged.

This packet owns edge-refinement history starting at the supplied root. It does
not invent operations before that root, replace the existing product ModelGraph,
provide general branching/timeline edits, migrate incompatible schemas or cover
other modeling operation families. Plain standalone `Model` encoding still stores
only its model data. Full-period quotient faces, seam relocation, material-removing
subranges, general blend networks and the full native parity objective remain open.
The frozen clean crate is unchanged.

### Native refinement regeneration and prefix branches (r30)

The existing edge-history owner exposes semantic `Request` data: persistent
parent edge ID, literal physical cuts, operation key and UV allowance. Requests
are inputs, not receipts. `History::from_requests` constructs every actual step
under the caller's remaining aggregate allowance and resolves selections only
against persistent IDs of ACTUAL current edges. Numeric storage positions and
retired lineage nodes do not substitute for a missing selected parent.

`with_cuts` rebuilds a changed ancestor and all dependent operations; `replace_cuts`
publishes the new chain only after complete success. Original keys retain the
existing constructor's stable child-role identities, while geometry, source
snapshots, exact authored relation fractions and current proof data are all
recomputed. Cuts retain their literal actual-edge parameter coordinates. A new
ancestor range that excludes a downstream cut, or a removed selected descendant,
refuses without silently choosing a nearby edge or leaving a partial edit.

`rebased` performs the same explicit fresh construction on a supplied canonical
root. Placement and storage permutation can retain semantic selections when the
parent IDs persist. This is distinct from importing a saved expectation packet:
regeneration permits new actual output models after an explicit source/request
edit; import still requires its saved models/maps to match complete reconstruction.
Fresh regeneration can also establish a valid chain under lower resource caps,
then write a packet that loads under those new caps. It cannot borrow old limits
or success flags to skip current qualification.

`prefix` returns an independent native branch at an exact prefix; `truncate`
atomically replaces the current chain with a rebuilt prefix. The original future
can remain in the original History. Branch merges, general dependency-graph edits
and other modeling operation families are separate work; this does not replace
the existing product ModelGraph.

```rust
let edited = history.with_cuts(0, &[0.25, 0.75], budget)?;
let mut branch = edited.prefix(1, budget)?;
// Subsequent branch.split(...) calls use the actual qualified branch tip.
branch.truncate(0, budget)?;
```

Seven integration regressions cover real downstream topology/ownership changes,
constant child-role IDs, unchanged originals, save/load after regeneration,
out-of-domain and missing-parent atomic failures, root placement plus edge-vector
permutation, prefix branches/undo, lower-budget requalification, malformed semantic
requests, and a changed rational periodic ancestor followed by a requalified
selected child. The periodic seam vertex and literal original root stay retained.
The prior ten history transport tests remain separate regressions.

Native source/trim/lift/solid-incidence gates remain mandatory. This adds refinement
parameter editing and independent prefix branches, not arbitrary feature-DAG
merging, root-ID healing, cyclic seam relocation, material-removing subranges,
full-period quotient-face admission, general blend networks or whole-library
parity. The frozen clean crate remains unchanged.

### r31 — ordinary closed NURBS edges (2026-10-31)

Closed edge refinement/sewing no longer requires the storage flag `periodic`.
The existing native `curve_partition_agreement::verify_source_closure` owner
provides a sufficient exact original C0 test for ordinary clamped curves and
ordinary unclamped curves whose natural endpoint knot multiplicities are at
least the degree. It selects the actual active endpoint poles, ignoring inactive
exterior poles. Positive endpoint weights may differ. Near-equal points and
unproved endpoint blends refuse; periodic definitions retain their previous
cyclic-storage proof. This is an endpoint theorem, not a simplicity certificate.

The existing closed sewing gate additionally requires complete actual rational
curve authority, three-dimensional data and its bounded whole-span nonzero
tangent proof. Reconstructed sewing records recheck that authority. Coincident
endpoint-only keys still refuse. Actual BRep edge partition retains both face
uses, the seam vertex, source identities, independent authored partial clocks
and fresh original-to-child geometry/UV admission. No endpoint welding or
periodic reinterpretation is introduced.

Seven new integration regressions cover clamped and ordinary unclamped rational
closed-edge partitions, inactive exterior poles, distinct positive endpoint
weights, near-closure refusal, reversed independent partial clocks, repeated
child edits, numerical volume and codec/history parameter regeneration. The
clean comparison crate remains unchanged. Full-period quotient faces, cyclic
seam relocation, general endpoint-blend closure, arbitrary blend networks,
material-removing subranges and full library parity remain open.

### r32 — actual free closed-edge seam relocation (2026-10-31)

`feature_ops::edge_partition::seam::relocate` and
`relocate_with_uv_tolerance` now stage an actual closed-model seam edit.
The original edge must have exact source C0 closure, exactly two singleton face
wire uses and a seam vertex unused by any other edge. A connected junction is
not a movable parameter origin. Periodic and ordinary closed original curves
are supported. The existing native section-phase candidate builder supplies a
proposal; the saved output is an explicitly closed clamped NURBS curve. Clamping
and endpoint coupling remain proposals, followed by fresh whole-original bounds.

Both actual face pcurves receive their own phase rotation using their actual
independent domains and reversed use direction. Full authored affine relations
retain their direction and obtain fresh actual endpoint lift integers. Partial
relations refuse rather than being dropped: crossing a newly chosen seam may
require two affine pieces, which the single `CoedgeTrim` cannot represent.
An original-to-actual phase certificate is required independently for the world
curve and both UV curves. Actual boundary lifts, face domains, injectivity,
sewing, local endpoint tolerances and closed-solid incidence are rechecked.
The result retains faces, shells and bodies, provides modified edge/vertex/loop
identities with actual persist lineage, transfers local tolerances and removes
the consumed sewing marker. Source and result access are immutable; `reverify`
reconstructs the operation from the original source and literal request.

The existing `periodic_seam::verify_curve` owner now compares original and actual
rational Bernstein definitions on common exact-rational normalized phase cells.
Both original knot partitions and the original wrap point enter that partition.
The rational affine source/candidate clocks use their actual binary64 domain
ends, including a rounded new end that differs from an exact translated period.
Blossom endpoints are enclosed outward independently; the shared homogeneous
residual bound supports independent positive weight gauges and translated
coordinate origins. No rounded source-knot translation or display sampling
supplies a certificate. The shared curve-distance blossom owner now exposes
this private endpoint-enclosure primitive; existing restriction calls retain
identical scalar-endpoint behavior. Rational subdivision is bounded by caller
cells and depth 64. The prior independent enclosure path remains available when
the initial common partition does not fit the caller allowance.

Six BRep regressions cover ordinary unclamped active cycles, actual vertex movement, retained face/body ownership,
post-seam shared-edge subdivision, original immutability, repeated clamped seam
edits, reversed world traversal with independent/directed full authored clocks,
invalid requests, exhausted proofs and partial-relation refusal. Three new native
regressions cover rational/nonunit clocks, near-end wrap, independent weight
gauges, a large common coordinate origin, and rejection after pole/knot-clock
mutation. The existing five periodic candidate tests now verify the stronger
small-tolerance bound (seven cells at 1e-12) and retain low-budget refusal. The four section-phase consumer regressions also
retain atomic unresolved behavior at an unproved 1e-30 tolerance, while the
previous five-cell 1e-12 example now has accepted continuous evidence.

Partial seam relations, seam changes at connected vertices, feature-history
transport for this operation family, full-period quotient-face admission,
material-removing subranges and general blend networks remain open. This does
not establish global solid embedding or full native library parity.

Current r32 qualification: 335 fresh integration tests in 64 suites passed;
no unit suites rerun. Catalog audit passed; frozen clean baseline has 11 unchanged
files. Global architecture gate still reports ten unrelated oversized files;
API mapping still reports twelve unrelated unassigned public roots. Neither
gate nor the native-parity goal is marked complete. Exact commands and logs
are retained in `docs/qualification/native-library-parity-2026-10-31/results.json`.

### r33 — partial authored seam relations with retained clock breaks (2026-10-31)

The existing seam relocation owner now accepts partial authored relations.
When either face use has a partial relation, the native shared-edge partition
constructor cuts at the requested new seam. Both actual wires rotate to start
at the new shared vertex. The old seam remains an intermediate vertex, so the
saved boundary is an actual two-edge cycle with original physical clocks.
No single affine relation is forced across a cyclic wrap and no source metadata
is erased. The full-relation/no-metadata path retains r32's single closed curve.

`Relocation::cycle` returns actual edges in forward world traversal from the
new seam, `seam_vertex` identifies its actual vertex, and `relations` exposes
constructor-owned exact original relation coverage. Each retained fragment also
lives on its actual saved coedge as ordinary `CoedgeTrim`, so standalone Model
codec transport and subsequent native edge edits preserve it. This extends the
existing topology/codec representation by actual clock-break incidence, rather
than introducing a parallel metadata store invisible to other constructors.
Source requests crossing the new seam receive exact rational fragments covering
[0,1]; noncrossing requests retain one whole fragment on the appropriate edge.
Reversed world uses, independent face clocks and authored interval directions
are retained. Requesting the original seam is an admitted immutable no-op for
partial relations, after complete source qualification.

Existing Split lineage and local tolerances are retained. Each cyclically
reordered loop receives its own persist identity linked to the actual partitioned
loop. A final full actual-model qualification rechecks lifts, winding, charts,
sewing and solid incidence. The partition's complete prior work is deducted
before this final qualification. `reverify` reconstructs model, cycle, seam
vertex and exact authored ownership together. Failures publish no partial model.

Five new regressions cover exact half/half relation ownership across the new
seam, actual wire starts and retained old seam, reversed independent directed
partial clocks, Model codec transport followed by another native edge split,
a noncrossing relation/original-seam no-op, and source-relation/budget failure.
The previous six seam regressions now also require successful partial-clock
retention instead of the old deliberate refusal.

This is a two-edge representation for partial clocks, not a claim that they
have been compacted into one closed NURBS definition with a first-class wrapped
affine metadata object. Generic existing multi-edge cycle seam editing,
connected seam vertices, mixed-operation history, full-period quotient faces,
material-removing subranges, general blend networks and global solid embedding
remain separate work. Full native-library parity remains active.

Current r33 evidence: 340 distinct integration checks in 65 suites passed.
After the matrix, intermediate Split provenance retention was tightened and
the eleven affected seam checks rerun with a generated-vertex role regression;
these replace the same matrix cases and are counted once. The existing ten
architecture violations and twelve unassigned API roots remain unchanged;
catalog passes and the frozen clean baseline retains all 11 files.

### r34 — repeatable actual multi-edge cycle seam editing (2026-10-31)

The same `seam::relocate` entry now accepts a nonclosed selected edge when its
two incident actual wires contain the same complete cycle of 2..256 distinct
edges. Both directions of the selected edge domain endpoints are admitted;
interior requests subdivide the actual selected edge through the existing
original-geometry/partial-clock constructor. Endpoint requests reuse the literal
existing vertex without changing any curve, vertex position or edge/vertex ID.
Both actual face wires cyclically rotate to begin at the requested vertex.
An unrelated pair of incident face loops is not guessed to be a shared cycle.

This permits further seam edits on the r33 partial-clock result and on cycles
produced by repeated native edge subdivisions. Existing clock-break vertices,
other edges, original face/body IDs and per-coedge authored fragments remain
retained. Modified loop identities link to their actual intermediate source;
intermediate generated cut-vertex provenance survives. Complete source and final
model qualifications share the caller budget, with the selected-edge Split's
full prior work deducted before final qualification. Model codec transport
requires no new schema or hidden relation storage.

`Relocation::traversal` exposes `CycleUse { edge, reversed }` in a closed cyclic
order oriented with the selected source edge. This distinguishes independent
stored edge directions from actual cycle traversal. The existing `cycle` is its
edge-index projection, `seam_vertex` is the actual current origin, and `edge`
continues to identify the selected SOURCE edge. Both singleton and partial-clock
paths now expose the same traversal contract. `reverify` freshly discovers the
original cycle, reconstructs the operation and compares geometry/model, actual
cycle, directed traversal, seam vertex and exact original relation maps.

Five new regressions cover another interior seam on a retained partial-clock
cycle, exact source relation fragments on saved coedges, both endpoint requests
with literal edge/vertex geometry and persistent IDs, independently reversed
world-edge definitions, explicit closed mixed-direction traversal, codec-loaded
cycles with repeated edits and a literal no-op at the current origin, unrelated
face-cycle refusal, invalid scalars and exhausted work. The prior eleven seam
regressions remain in scope.

This edits actual shared cycles without compacting their original clock breaks
into a single curve. Arbitrary cycle compaction, movement of connected junctions,
mixed-operation feature history, full-period quotient faces, material-removing
subranges, general blend networks and global solid embedding remain open. The
whole-library goal remains active and the clean comparison crate unchanged.

Current r34 qualification: 345 fresh distinct integration checks in 66 suites
passed, including all sixteen seam/cycle regressions. No unit suites rerun.
Catalog audit passes; clean comparison retains all 11 unchanged files. The
existing ten global architecture diagnostics and twelve unassigned API roots
remain outside this slice and the goal is not marked complete. Commands, hashes
and logs are recorded in the qualification results.

### r35 — rational curve concatenation and actual cycle compaction (2026-10-31)

The existing native `curve_partition_agreement::concatenate` owner now constructs
an admitted open or closed rational curve from oriented original curves. Each
input retains its literal physical source range and gets one unit actual output
interval. Sources may have different degrees, positive weight gauges, parameter
ranges and traversal directions. Clamping, elevation to a common degree,
parameter normalization, homogeneous weight matching and endpoint coupling are
explicit numerical proposals, not exact source restrictions. Every whole actual
output section must be freshly admitted against its unchanged original under
one aggregate cell allowance. Output controls remain bounded to 256, input count
to 64, and degree to the existing curve limit. The immutable result exposes its
actual curve and segment maps and reconstructs all admission in `reverify`.
Closed output additionally couples its first/last stored poles; it does not
claim G1/G2, regularity, simplicity or solid topology.

`curve_partition_agreement::mapped::verify` supplies complete original affine
clock comparison on exact rational phase cells, including descending clocks
and matching dimensions with different degrees. Both actual knot partitions
enter the common subdivision. Original/candidate parameter endpoints are
independently enclosed outward for the shared homogeneous blossom/residual
bound. Unsupported tolerance/work remains an unaccepted report, not a guessed
intersection or a proof of excess. Rational subdivision retains the depth-64
limit. The prior periodic-phase owner now calls the same private wrapped-clock
specialization, preserving its original normalized wrapped correspondence,
independent enclosure fallback, and existing public report contract. The
existing twelve periodic/section regressions remain in the matrix.

The BRep `seam::compact::compact` / `compact_with_uv_tolerance` constructor uses
these native curves to replace a complete shared cycle by one actual closed
world edge, one retained seam vertex and independently concatenated face UV
curves. Stored direction flags preserve the complete actual boundary lift even
with mixed world-edge directions. Both original face carriers, face/shell/body
identities and unrelated components remain retained. Intermediate junctions
connected to other edges refuse; compaction cannot erase their incidence.
Authored coedge clock metadata also refuses compaction rather than being erased;
those cycles remain editable with the existing clock-preserving cycle API.

All source edge AND cycle-vertex local tolerances contribute to the new merged
edge's precision. At each p-fold integer output junction, its exact active
endpoint pole additionally must meet the original vertex point tolerance. This
check includes removed vertices; source point precision cannot be loosened just
by removing a clock-break entity. Actual final edge endpoints retain their
vertex tolerance check. Each such junction check charges the shared work budget.
Source/Split/rotation work, all world/UV concatenation proofs and final saved
model qualification share the caller allowances.

Consumed edges have actual Merge lineage; retired clock-break vertices have
Deleted changes. Loop modifications link to actual intermediate identities.
Edge/vertex indices are compacted with all surviving coedges/edges remapped;
unrelated geometry and authored metadata remain literal. Local tolerances and
consumed sewing markers are transferred/removed consistently. `Compaction`
retains its immutable source, canonical source traversal, actual segment maps
and complete reconstruction in `reverify`.

Four native regressions cover mixed degrees/reversal, rational closed arcs with
independent knots and weight gauges, both descending affine clocks, rejection
of changed poles/wrong clocks, incompatible joins/closure and exhausted work.
Four BRep regressions cover four edges to one closed edge/vertex, original face
and body identity, Merge/Deleted history, strict removed-vertex tolerance
inheritance, numerical volume, subsequent edge partition, mixed directions,
codec transport, authored-clock refusal, failure atomicity and preservation of
a separate body with authored partial coedges through index compaction.

Compaction of authored-clock cycles still needs a first-class piecewise relation
representation (the existing explicit cycle remains valid). General feature
history, full-period quotient faces, material-removing subranges, general blend
networks and global solid embedding remain open. Full native parity is not
claimed; the frozen clean comparison remains unchanged.

Current r35 evidence: 353 distinct integration checks in 68 suites passed.
After the main matrix build, four affected compaction checks were rerun with
strict original vertex-tolerance inheritance and direct junction guards; these
replace the same matrix cases and are counted once. No unit suites rerun.
Catalog passes, frozen clean baseline retains all 11 files, and the same ten
architecture violations/twelve unassigned public roots remain outside this
slice. Full parity remains active. Exact commands/hashes/logs are recorded in
the qualification results.

### r36 — full-lap extrusion quotient faces in actual edge editing (2026-10-31)

Existing edge-refinement qualification now admits complete one-period faces for
an exact axis-aligned rational extrusion with a simple original closed profile.
The native `surface_injectivity::certify_periodic_extrusion` entry lives under
the existing seam-glued injectivity owner. It proves injectivity on the cylinder
quotient, not injectivity of the closed UV rectangle whose shores coincide.
No standalone alias/public NURBS root or baseline relaxation was introduced.

The source surface must have exactly one periodic axis, an independent clamped
linear other axis, equal literal weights across the extrusion rows, and one
world coordinate constant on each source row with distinct endpoint values.
The other two coordinates are literal copies across those rows. Consequently
the axial coordinate is strictly monotone, while its complementary projection
is the actual unchanged original profile. This profile is extracted by literal
coordinate/weight selection, requires exact original cyclic storage closure,
and is decomposed only into exact original Bernstein spans. Complete native
Jordan simplicity then excludes all other same-height profile identifications.
Source seam identity remains mandatory. Transposed U/V storage and permuted
world coordinates are supported. The method does not substitute a sampled or
rounded fitted profile for the original tensor surface.

The BRep full-lap path additionally requires exactly one exact-rational period
of original UV pole-hull width, an unperforated rectangle inside the other
natural interval, and complete original side coverage without gaps/overlaps.
Every actual pcurve is clamped, lies on one literal rectangle side and stays
inside its own endpoint range. Opposite periodic shores must use the same
actual world-edge IDs once each with opposite directions. Thus seam identity
is owned by actual BRep incidence, not inferred from coincident points. Actual
world/UV lift tolerances remain independently mandatory; this does not claim
bit-identical opposite-shore UV subdivisions. Ordinary full loop simplicity,
winding, source/final regularity, sewing, vertex/edge ownership and solid
incidence checks remain in place.

Profile Jordan pairs now consume the same operation-wide pair allowance as
face trim pairs. Structural/profile cells consume the existing shared cell
allowance, including both source and output qualification. The public
Verification field documents that admitted charts can be quotient charts;
it is not a global solid embedding certificate. Standard non-edge feature
qualification has not been switched to this new edge-editing path.

Four native regressions cover a full rational extrusion, transposed storage,
permuted world axes, double-cover rejection, altered tensor weights, exact seam
failure and cell/pair exhaustion. Four BRep regressions use a real three-face
closed cylinder (two rational closed rim edges and one shared seam edge used
twice on the same face): rim and seam subdivisions, complete saved face identity,
numerical volume, independent UV lifts by two periods, transposed/oriented face
storage, failure atomicity and wrong/multiple-period refusal. Both seam uses
are actually subdivided, not replaced by a sampled proxy or an extra cap.

Oblique/general periodic carriers, nonliteral profile decompositions, doubly
periodic surfaces, perforated quotient faces, general feature-pattern admission,
compaction of disconnected/nonaffine authored-clock unions, material-removing subranges and general
blend networks remain open. This implements a proved full-periodic face family;
whole-library parity and global solid embedding remain unclaimed. The clean
comparison crate is unchanged.

Current r36 qualification: 361 fresh distinct integration checks in 70 suites
passed, including the eight new native/BRep full-period quotient regressions.
No unit suites rerun. Catalog audit passes, clean comparison retains all 11
files, and the same ten architecture diagnostics/twelve unassigned public
roots remain outside this slice. Full native parity remains active. Commands,
source hashes and logs are retained in the qualification results.

### r37 — exact source-frame oblique extrusion quotients (2026-10-31)

The existing periodic extrusion proof no longer requires world-axis alignment.
A private source-frame child selects literal noncollinear source controls P,A,B
and computes N=(A-P) cross (B-P) in exact rational arithmetic on original binary64
scalars. Every original base pole must satisfy N.(pole-P)=0 exactly. All source
row displacements must equal the original D=Q-P exactly and N.D must be nonzero.
Thus equal surface points have equal transverse height, hence equal extrusion
clock; original profile Jordan injectivity then gives the same periodic point.
No normalized floating normal, fitted plane, transformed proxy surface or
near-equality supplies this theorem. A next-float/nonplanar source refuses.

The native reporter exposes literal `source_frame` controls and the successful
`projection_axes`. Each omitted coordinate must have a nonzero exact normal
component, so its complementary raw-coordinate profile projection is one-to-one
on the source plane. Candidate projections share one accumulated pair/cell
budget; a failed projection cannot reset allowances. `axial_coordinate` remains
present for the prior single-coordinate height cases; an oblique plane uses its
source-frame linear height instead. The original exact seam, full original
Bernstein decomposition and complete Jordan checks remain mandatory.

Row weights may also differ by one exact constant positive gauge. Original
cross-products verify the same ratio for every row without rounding division.
The extruded surface then has clock h(t)=r*t/(1-t+r*t), whose derivative is
strictly positive for r>0. Its original base profile and translation family are
unchanged. Nonconstant row ratios still refuse. This expands the r36 literal
weight-equality restriction without borrowing a sampled correspondence.

Five new native regressions cover oblique translation, an exact oblique profile
plane under a nonsingular affine source transform, constant positive row gauge,
near-planarity/nonconstant-shift rejection, tangent extrusion and work refusal.
Three BRep regressions use a real closed transformed cylinder: shared seam and
closed-rim refinements, original face/body geometry ownership, negative two-lap
UV lifts, positive numerical material volume, and a weighted extrusion with its
actual rational seam clock. The original four quotient tests now share the same
unchanged source factory with these tests, keeping their previous proof scope.
The actual rectangular paired-shore ownership and final model gates are not
weakened or replaced by the source-frame theorem.

Exactly nonplanar/approximately transformed source tensors, non-extrusion
periodic carriers, nonliteral profile decomposition, doubly periodic surfaces,
perforated quotient faces, general feature-pattern integration, authored-clock
cycle compaction, material-removing subranges, general blend networks and global
solid embedding remain open. Full native-library parity remains active and the
clean comparison is unchanged.

Current r37 qualification: 369 fresh distinct integration checks in 72 suites
passed, including eight new oblique/row-gauge regressions and all prior direct
quotient cases. No unit suites rerun. Catalog passes, clean comparison retains
11 unchanged files; the same ten architecture diagnostics and twelve unassigned
public roots remain outside this slice. Full native parity remains active.

### r38 — strictly internal trim holes on full-period quotient faces (2026-10-31)

The full-period carrier path no longer rejects every face containing a hole.
Its literal outer rectangle still requires complete exact side coverage and
opposite uses of the same actual seam edges. The existing global periodic
extrusion quotient theorem remains unchanged. Removing strictly contained
regions from an injective quotient chart does not invalidate that injectivity.
All original hole curves must independently pass the subsequent whole trim
region proof: simplicity, clockwise orientation, strict containment and mutual
separation. Neither a periodic flag nor a rectangular hole bounding box replaces
these checks. Hole counts, pairs, cells and classification work retain their
existing aggregate allowances; failures publish no partial refinement.

A full-lap carrier now takes its enclosure from the original OUTER rectangle.
Hole pole hulls do not define the outer material rectangle; they may over-enclose
their actual rational curves. Actual hole inclusion is decided by the mandatory
original-curve trim audit. The prior strict-subperiod chart path still uses its
complete all-loop enclosure and its existing contraction proof. No native
Surface theorem or public API alias was added or weakened in this extension.

Five integration regressions use a closed cylinder partitioned into a full-lap
face with an internal trim hole and the actual original curved patch occupying
that hole. This verifies a real trimmed-face partition, not an inferred physical
through-cut. Actual shared hole-edge refinement updates both incident faces;
outer seam refinement retains the hole and patch literally. Original face IDs,
partial authored hole clocks and exact relation fragments remain retained.
Another case applies an exact oblique transform and shifts outer/hole UV clocks
by two full periods. The source remains unchanged and the numerical oriented
volume remains 2*pi. Native full-curve lifts and trim ownership are requalified.

Negative cases isolate the rejection mechanisms: a hole touching the outer
periodic shore reaches the native trim winding/simplicity refusal; a reversed
hole with opposed shell-edge directions reaches the original UV orientation
validation (hole loops must be CW). Pair exhaustion remains atomic. These
checks ensure removing the blanket hole ban does not remove original validity
requirements. No actual through-hole creation or new cavity geometry is claimed.

Holes crossing the chosen periodic seam, non-extrusion/nonplanar carriers,
nonliteral profile decomposition, doubly periodic surfaces, general feature
pattern integration, authored-clock cycle compaction, material-removing active
subranges, general blend networks and global solid embedding remain open. Full
native parity remains active and the clean comparison remains unchanged.

Current r38 evidence: 374 distinct integration checks in 73 suites passed.
The five new cases were rerun after strengthening only the negative input data
and assertions to identify trim-contact and original hole-orientation guards;
they replace the same matrix cases and are counted once. No unit suites rerun.
Catalog passes and the clean baseline retains all 11 files. The same ten
architecture diagnostics and twelve unassigned roots remain outside this slice;
full native-library parity remains active. Exact commands/hashes/logs retained.

### r39 — tapered full-period carriers, cone poles and native constructor cones (2026-10-31)

The existing exact source-frame theorem now admits nonnegative homothetic rows:
C_other(i)-Q = lambda*(C_base(i)-P) for every literal original pole, with one
exact rational lambda. The base row is planar and N.(Q-P) remains nonzero.
A positive constant row-weight ratio supplies the same strictly monotone
rational extrusion clock psi. At fixed height, the in-plane scale is
(1-psi)+lambda*psi. Positive lambda preserves an injective regular profile;
lambda=0 collapses exactly the opposite boundary; negative lambda has an
interior scale collapse and refuses this quotient theorem.

Either original nonperiodic endpoint can be the cone pole. A collapsed first
row selects the other original row as the nondegenerate base. Native reports
expose `base_end`, outward `profile_scale`, `collapsed_end` and
`regular_interior`. The last flag has its own fresh whole-original projected
curve regularity proof; it is NOT inferred from profile Jordan injectivity.
The report's quotient identifies both periodic shores and any declared
collapsed boundary. All original controls, weights and knot domains remain
unchanged; no tolerance fitting or artificial cap supplies pole authority.

When a full-lap material rectangle includes that collapsed endpoint, its actual
boundary coedges must reference exact degenerate curves, one actual pole vertex
and the original source-frame apex. The existing whole UV winding, full lifts,
sewing, vertex/edge incidence, naming and numerical material-role checks remain
mandatory. A factored regularity theorem supplies rank away from the declared
pole: positive in-plane scale times nonzero profile derivative, with transverse
height derivative. A strict-subperiod common inverse-Jacobian contraction also
proves rank on its entire actual enclosure, even if an unused natural carrier
boundary collapses. Generic unsupported regularity still refuses. This changes
regularity authority to independently proved actual charts, not an unchecked
exception for a periodic or degenerate flag.

Ordinary nonperiodic cone patches from the public `brep_core::frustum` constructor
also enter the edge-refinement path. Their original ruled-pole certificate uses
a strictly positive source ray denominator and a strictly signed angular ratio
derivative. Together with the positive radial clock these prove both global
ray-quotient injectivity and rank away from the owned pole. A literal parameter
transpose only permutes source controls/knots; it is not a fitted replacement.
Actual pole-edge/vertex authority and complete UV pole-hull containment in the
source chart remain required. Ordinary regular nonpole faces keep their prior
regularity and injectivity gates. Standard non-edge feature admission remains
unchanged.

The basic Model pole validator now recognizes exact one-period lifted straight
pole pcurves when the original periodic surface seam identity is proved. It
also selects actual active endpoint rows for p-fold knots, rather than trusting
an inactive first/last stored row. Lower-multiplicity endpoints use exact original endpoint basis coefficients,
checked nonnegative and summing to one, and exact zero homogeneous residuals.
This admits weighted cancellation without sampling or fitting. The validator
moved into a private model-validation child. Three focused unit regressions
cover lifted range/seam/pole refusal, inactive-row false positives, and exact
weighted cancellation with rejection of a nonzero subnormal residual.

Four native regressions cover positive taper, either cone end, rational row
gauges, negative/interior collapse and inconsistent source homothety. Six BRep
regressions use real full-period frusta/cones, opposite parameter orientation,
actual rational seam clocks, an exact oblique transform with negative UV lifts,
original pole/face identity, positive numerical volume, undeclared pole refusal
and exhausted work. Two additional integration regressions use the public
native cone constructor, refining radial edges at either pole end and the rim
while retaining the actual zero-measure pole edge. These are actual closed BRep
operations, not only native Surface reports.

Nonhomothetic/general periodic carriers, unsupported pole charts and unproved
original ray orderings, nonliteral profile decomposition, seam-crossing
hole loops, doubly periodic surfaces, general feature-pattern integration,
authored-clock compaction, material-removing active subranges, general blend
networks and global solid embedding remain open. Full native-library parity is
not claimed and the frozen clean comparison remains unchanged.

Final r39 qualification: 386 integration tests in 76 suites and three focused
pole-validator units passed on the final sources (389 fresh checks). Historical
104 non-overlapping checks make the mixed evidence aggregate 493, not a fresh
whole-workspace run. Formatting and catalog audit pass; the clean baseline
remains 11 unchanged files. Architecture still reports ten oversized-file
violations (one concurrent source grew since r38); API mapping reports thirteen
unassigned concurrent public roots. Baselines were not relaxed. Full parity
remains active. Logs, exact commands and source hashes are retained in
`docs/qualification/native-library-parity-2026-10-31/results.json`.

## r40: source-owned ray frames for strongly sheared native cone patches

The existing ruled-pole quotient certificate retains its first endpoint-sum /
endpoint-difference projection. If its full polynomial proof fails, it may try
an exact dual projection of the two original endpoint rays, followed by three
independent original rays. These are fixed linear functionals on ORIGINAL
homogeneous ray numerators, not transformed or fitted replacement surfaces.
Two-ray Gram coefficients and three-ray inverse coefficients use exact rational
arithmetic; only final original projected coefficients receive outward binary64
enclosures. The whole original denominator must remain positive and the whole
ratio derivative numerator must have one strict sign. This gives distinct rays
and rank away from the declared pole, with the existing positive radial clock.

Every attempted projection consumes the same caller cell allowance. An
insufficient allowance cannot restart the proof with a fresh budget. A singular
source frame cannot supply a projection. Repeated rays, folded order, nonconstant
pole rows and unsupported source representations retain refusal behavior.
The public API and source geometry are unchanged. The algorithm moved into a
private child of the existing surface_quotient_injectivity owner; the large
report implementation shrank rather than adding another public NURBS root.

The new integration cases exercise an exact determinant-one strong shear of
public-constructor cones at either pole end, then split actual radial edges and
recheck complete BRep qualification, preserved pole/face identity, numerical
volume and source immutability. Native regressions also cover independent row
weights, nonunit domains, all four sheared quadrants, actual third-frame use,
shared budget exhaustion and negative folded/repeated/nonconstant-pole cases.
General pole charts beyond these freshly proved ray projections, nonlinear
carriers and global solid embedding remain open.

Final r40 qualification: seven integration checks (four native and three BRep),
fifteen existing quotient units and one source-shell pole regression passed on
the final implementation: 23 fresh checks. The initial three native cases in
the combined log are replaced, not double-counted, by the final four-case
native run. r39 full-matrix evidence remains historical. The mixed distinct
evidence aggregate is 514; this is not a fresh full-workspace run. Formatting
and catalog checks pass; clean comparison retains all 11 files. Current
architecture gate has 11 oversized-file violations, including a concurrent
Minkowski file crossing 800 lines; API mapping has 13 unassigned concurrent
roots. No baseline was relaxed; full parity remains active. Commands, source
hashes and logs are in the qualification results.


## r41: authored-clock preservation during actual cycle compaction

`seam::compact` no longer rejects all authored coedge clocks. The private clocks
child maps each ORIGINAL nominal world clock through its original reversed use,
the actual world concatenation segment, and the saved coedge direction. An
independent original UV clock maps through its actual UV concatenation segment.
Exact rational output images remain in `Compaction::relations()` together with
the original face/wire/coedge/edge address, original directed trim, proposed
stored trim and independently recomputed original-geometry bounds.

The existing single-relation Coedge representation admits a union only when
all exact output intervals are connected and every requested pair lies on the
same exact affine line. It checks both exact pre-storage images and actual
stored binary64 pairs. Thus rounding cannot fill an authored gap or flatten
nonaffine clocks. One partial request remains partial; adjacent partial requests
merge to exactly their union; complete compatible cycles retain complete
metadata. Request orientation remains explicit in the original relation maps.
Disconnected/nonaffine unions still require a compound coedge representation;
this implementation does not silently expand or discard their requests.

Original-to-actual world concatenation bounds plus a fresh actual world clock
drift bound handle rounded output parameters. Fresh whole original-pcurve versus
saved-pcurve comparison, original-world versus saved UV-lift comparison, and
actual saved world/UV correspondence are mandatory per request. All consume the
same operation work allowance. Exact aggregate endpoint/line checks bind those
actual request fragments to the single saved coedge relation. Source/final whole
boundary, trim region, sewing, junction precision and naming gates remain.
`Compaction::reverify` reconstructs and compares the relation maps as well as the
actual resulting model. No topology or codec schema changed.

Regression coverage includes independent nonunit UV clocks, directed requests,
mixed original world-edge directions, whole and partial unions, subsequent
native splits, actual Model codec roundtrip, nonrepresentable output clocks,
rounding-erased gap refusal and geometrically close nonaffine request refusal.
The original source remains immutable on both success and refusal. General
compound clock unions and mixed-operation history remain open.

Final r41 qualification: 21 integrations in four affected-consumer suites
passed on final sources: seven authored-compaction regressions, four general
compaction, five shared-cycle seam and five partial-clock seam cases. The
historical mixed evidence aggregate is 521, not a fresh full-source matrix.
Formatting/catalog pass and the clean comparison retains all 11 files. Current
architecture/API gates still report 11 oversized-file violations and 13
unassigned concurrent roots; the architecture baseline was not changed. Exact
commands, source hashes and logs are retained in the qualification results.
Full native library parity remains active.

## r42: mixed native edge-edit history and semantic reconstruction

The existing edge-partition history owner now exposes `history::sequence`:
`Command`, `Operation`, `Applied`, `Step` and `Sequence`. It composes existing
native Split, seam relocation and cycle compaction results without replacing
any operation's geometry, topology, naming or proof owner. Each command selects
the actual persistent parent edge ID. The retained Split-only History API and
its `/1` format remain supported.

The mixed `/2` packet stores canonical original geometry/identity, typed commands,
per-step remaining caps, actual model expectations and exact ownership maps.
Loading reconstructs every operation afresh on the preceding actual tip and
compares literal typed model/map values, including floating signed zero and
exact rational authored-clock image strings. No imported approval flag or
stored numerical report can authorize an operation. Strict field sets, duplicate
JSON rejection and existing bounded byte/node/depth limits remain mandatory.
Caller work caps apply across ALL operation families, and packet limits cannot
increase a caller's remaining allowance. As in the existing Budget contract,
numerical mass allowance is per qualification and closed sewing keeps its
separate native cap; the aggregate journal counters are cells/pairs/domain work.

Version `/1` migration delegates the only geometry replay to its existing owner,
then moves admitted actual steps and accumulated work into the typed mixed
sequence. It does not replay a second time with a renewed proof budget. Export
uses `/2`; old readers explicitly refuse that newer schema. Commands can be
regenerated on an explicit canonical root, replaced with complete downstream
reconstruction, or branched/truncated at a freshly reconstructed prefix. Missing
descendant IDs, invalid clocks, exhausted steps/work and oversized new packets
leave an existing sequence unchanged.

The mixed chain is a linear dependency history for three native edge operations.
General operation families, graph branch merges, automatic selection remapping
when a parent is deleted, ModelGraph/product integration and arbitrary document
schema migration remain open. General fillet networks, closed offset/hollow
and whole-library parity remain separate unfinished work.

Final r42 qualification: 23 integrations passed in three history suites
(ten legacy replay, seven legacy regeneration, six new mixed-operation cases).
After strengthening the actual-model corruption test, the six mixed cases
reran successfully and replace the same six cases rather than adding counts.
Cargo check, formatting and catalog audit pass; clean comparison remains 11
unchanged files. The mixed historical aggregate is 527, not a fresh whole
workspace/full-matrix result. Architecture/API still report 11 oversized-file
violations and 13 unassigned concurrent roots; baselines were not relaxed.
Commands, logs and source hashes are retained in the qualification results.
The whole native-library objective remains active.

## r43: original-bound affine world-edge clock normalization

The existing edge-partition owner exposes `reclock::reparameterize` and its
immutable `Reclock` result. A selected regular stored world edge receives a
finite increasing affine parameter interval. Source control points/weights,
vertex positions, original face carriers, UV curves and their independent
clocks remain unchanged. Knot values are only a proposal: exact rational source
clock images are rounded for storage, existing equal/distinct knot partitions
must survive, and the complete unchanged original world curve is freshly
compared with the actual saved curve under the requested affine map.

Every authored coedge request retains its directed original metadata and exact
rational output world-clock image. Actual stored endpoints receive a fresh
world-curve parameter drift bound; original whole-curve error plus that drift
must fit the local edge tolerance. Actual world versus unchanged UV-lift
correspondence is separately proved per request. UV intervals and periodic lift
integers remain literal original data. Whole source/final chart, boundary,
trim region, incidence/sewing and numerical material-role gates remain mandatory
under a single operation work allowance.

Clock edits retain original unclamped storage. Natural endpoints with at least
p-fold knots use their ACTUAL active endpoint poles, including inactive exterior
storage. Lower-multiplicity endpoints use the existing native outward homogeneous
interval enclosure at the actual endpoint. Both actual endpoints must fit the
original edge AND vertex tolerances; no vertex is snapped, no sampled value or
first/last inactive pole supplies authority. Invalid target clocks, changed knot
partitions, excessive rounded authored drift, degenerate edges and exhausted
work refuse without publishing a model. Native knot bounds remain in force.

Persistent edge lineage uses a stable parent/key-derived descendant and carries
local tolerances. Authored-use loops receive retained lineage. Other identities
and vertices survive. `Reclock::reverify` reconstructs the actual model and
original relation records. Model codec and later native splits preserve the
saved actual correspondence.

The mixed history Operation/Applied enums now include Reclock. The existing
strict `/2` command discriminator gains a `reclock` kind with exactly two domain
endpoints. Unknown kinds/fields still refuse; older `/2` readers lacking this
kind refuse it rather than guessing. Replay reconstructs the whole edge and its
exact original authored-clock images, compares actual output/model maps, and
uses the same remaining caller allowances. Editing a clock reconstructs later
literal-parameter requests with stable parent identities. Its shared Command
UV-tolerance field must remain finite and positive even though UV curves are
unchanged by this operation.

This is world-clock normalization, not material-removing active-subrange
normalization, arbitrary nonlinear reparameterization, compound coedge storage
or global solid embedding. General blend networks and multi-face offset/hollow
remain open; the whole-library objective is active.

Final r43 qualification: 15 integrations passed in three suites (six direct
world-clock, three new mixed-history clock cases, six retained mixed-history
cases). Source and codec replay, downstream split/regeneration and negative
precision/partition cases are all exercised. Formatting/catalog pass; clean
comparison retains all 11 files. Historical mixed evidence aggregate 536 is
not a fresh whole-workspace or full-matrix result. Architecture/API gates
still report 11 oversized-file violations and 13 unassigned concurrent roots;
baselines were not relaxed. Commands, logs and source hashes are retained in
the qualification results; full native-library parity remains active.

## r44: closed normal-thickness skins of original planar patch networks

`shell_thickness::planar_network::thicken` accepts one connected, coherently
oriented network of 1..32 full nonperiodic rational Bezier source charts.
The original controls must share one EXACT plane; fitted/sample planarity is
not authority. Exact corner control differences and positive original weights
fix each chart's normal orientation; final whole-chart regularity/injectivity
proof is mandatory. Original cap surfaces are retained literally, including
independent knot clocks, positive weights and transposed charts.

Original complete-boundary endpoints only index possible joins. Existing
normalized Bezier control/weight ownership or fresh exact curve/lift identities
decide internal seams, with one shared exact-work ledger. Source incidence must
be edge-manifold, coherent and connected. Only exposed original boundaries
receive ruled side walls. Original internal seams stay shared cap edges, not
internal material walls. Connected source rings naturally produce through-hole
walls and a single closed material shell. Vertex links, actual complete sewing
and whole final chart/trim/boundary/material-role/naming qualification remain
mandatory. New-mode whole-chart monotonicity fallback reuses the native global
original-chart certificate; prior feature qualification modes are unchanged.

A common original normal is computed with exact rational source differences.
Scaled normal components receive outward enclosures; sqrt values only propose
norm endpoints, whose squares are checked by exact rational inequalities.
Every ACTUAL saved translated pole is compared with the ideal signed normal
translation, including source-coordinate addition rounding. Positive unchanged
rational weights make these per-pole residual bounds valid over the COMPLETE
original carriers. No sampling grants thickness authority.

`normal_offset_error_upper` bounds each saved offset cap versus its original
normal-offset carrier. `cap_distance_bounds` also bounds closest distances in
both directions between the ORIGINAL cap union and actual offset cap union:
all originals are on the common plane, so plane separation gives the lower
bound even across different patches, and original same-UV correspondence gives
the upper bound. Positive cap separation is mandatory. This does not bound
distance to connecting side walls or grant global solid embedding.

The immutable result owns originals, orientations, exact exposed-boundary
addresses, actual Model and proof/normal-error evidence. `reverify` reconstructs
from unchanged originals and compares actual model/boundaries. General curved
source networks, noncoplanar support offset intersections, partial shared-edge
joins/T junctions, arbitrary trimmed source faces, pole/periodic decomposition,
self-trimming and general hollow/offset topology remain open. This is an added
native modeling path, not completion of general freeform offset/hollow parity.

Final r44 qualification: 15 integrations in three affected-consumer suites
passed (eight planar-network thickness, four feature-ops, three freeform). After
hardening the new norm enclosure with exact squared-endpoint checks, all eight
affected tests reran successfully and replace the same eight cases. Historical
mixed evidence aggregate 544 is not a fresh full-matrix or whole-workspace
result. Formatting/catalog pass; clean comparison remains 11 unchanged files.
Architecture/API still report 11 oversized-file violations and 13 unassigned
concurrent roots; baselines were not relaxed. Commands, source hashes and logs
are retained in the qualification results. Full library parity remains active.

## r45: original-carrier T-junction normalization for planar thickness networks

Planar network thickening now finds original patch corners inside straight
rational source boundaries, including unequal positive endpoint weights.
Exact rational collinearity and inverse rational-Bezier clocks supply original
cut images; endpoint positions only propose these edits. Stored cuts must stay
strictly ordered and distinct. Native original-curve partition checks qualify
all proposed pieces, and endpoint coupling to literal original T vertices needs
fresh COMPLETE original-curve agreement afterward. No source cap carrier is
split or refitted. `Thickness::cuts()` retains exact source clock, stored clock
and original witness-patch/corner addresses. Exposed Boundary records now also
retain their actual original source-curve parameter subrange.

UV segment proposals retain the EXACT original natural rectangle as an image:
constant side coordinates are copied literally, corner endpoints retained,
interior joins shared, and the varying coordinate must be strictly monotone
inside the original side. Every changed saved UV piece gets fresh original
agreement. Consequently the complete original/offset cap-union distance proof
from r44 remains valid; a rounded UV interpolation cannot remove carrier points.

A private prepared-boundary path in the existing rounded-patch assembler admits
variable coedge counts on the UNCHANGED original rectangular surfaces. Every
actual saved 3D/UV use needs a continuous whole-lift proof. Shared candidates
retain exact normalized-control or exact curve/lift routes; the prepared path
additionally requires a fresh actual tolerance lift when those exact routes do
not establish ownership. Positive rational straight uses may propose UV Möbius
weights derived from both original boundary clocks; only a fresh whole actual
lift admits them. Saved source/top cuts and literal original witness vertices
are reconstructed independently, including rounded offset-cap vertices.

The public full-patch `rounded_patch_network::assemble` keeps its prior exact
admission mode. Its three legacy regressions retain actual curved-network
sewing and rejection of missing caps, wrong complete curves, duplicate faces
and exhausted work. T-junction candidate matching does not grant endpoint-only
sewing or fill gaps. Source graph/manifold orientation, final vertex links,
whole actual chart/trim/lift/solid-role checks remain mandatory. Native world/UV
partitions and all prepared lifts consume the same operation cell allowance;
source-discovery and actual assembly share the exact-work ledger.

Supported new cases include one long side facing several shorter neighbors,
independent source clocks, nonbinary exact rational T roots, transposed charts,
negative thickness, oblique planes and open slits in a connected source sheet.
General curved partial-boundary root discovery, arbitrary trimmed source faces,
noncoplanar offset propagation, general self-trimming and global solid embedding
remain open. This does not complete the whole native-library objective.

Final r45 qualification: 16 integrations (eight final T cases, eight retained
planar-network cases) and three legacy exact-sewing units passed: 19 distinct
current checks. The final eighth T case extends/replaces the earlier seven-case
target rather than duplicating counts. Historical mixed evidence aggregate
555 is not a fresh full-matrix/whole-workspace run. Formatting/catalog pass;
clean baseline retains all 11 files. Architecture/API gates still report 11
oversized-file violations and 13 unassigned concurrent roots; no baseline was
relaxed. Logs, commands and source hashes remain in qualification results.
The whole native-library parity objective remains active.

## r46: bounded original curved-boundary T witnesses

The existing planar-network prepare owner now supports curved rational Bezier
boundary T candidates through its private curved child. Distinct original
corners outside the positive rational control hull are excluded with outward
bounds. For remaining candidates, the existing native complete-domain clearance
proof either separates them or retains the obligation. Native bounded nearest
point search proposes a parameter; a separate original homogeneous interval
enclosure at that literal parameter certifies the witness distance. Search,
clearance and witness cells come from the SAME remaining operation allowance.
No sampled hit or nearest-point seed supplies root/geometry authority.

`CutKind` separates `ExactStraight` rational inverse roots from `CurvedWitness`
parameters. For curved witnesses the rational parameter field represents the
literal proposed binary64 scalar, not an exact algebraic intersection root.
The original point-distance upper bound and native discovery cells are retained.
Straight bounds concern their exact rational parameter; stored-scalar rounding
is still qualified by the full child-curve admission. Curved witness ambiguity,
boundary-only witnesses, unresolved candidate clearance and colliding distinct
original vertices cannot silently merge selections or publish partial output.

Existing native world/UV partitions, post-coupling whole ORIGINAL curve bounds,
actual saved face lifts, original natural UV rectangle images and final closed
BRep gates remain mandatory. Every original cap carrier stays literal. The
normal-offset and bidirectional cap-union distance theorem therefore remains
unchanged. Source/offset reconstruction, Model codec and repeated reverify retain
actual curved seam incidence rather than replacing the seam by its chord.

Verified families include polynomial and positive rational curved T boundaries,
midpoint/quarter and multiple ordered witnesses, independent nonunit charts,
transposed neighbors, both thickness directions and oblique source planes.
An original corner inside the control hull but separated from the entire curve
must NOT produce a T join. Native search budget exhaustion leaves originals
unchanged. Global root uniqueness, every nonlinear boundary clock branch,
arbitrary trimmed/periodic/pole sources, noncoplanar offset propagation and
global embedded-solid validity remain open; whole native library parity is not
claimed by this added construction path.

Final r46 qualification: 23 integrations passed across three suites (seven
final curved-T cases, eight retained straight-T cases, eight retained thickness
cases). The final seven-case target replaces its prior five, adding two cases
without duplicate counts. Formatting/catalog pass; clean comparison retains
all 11 files. The mixed historical aggregate 562 is not a fresh full-matrix or
whole-workspace result. Architecture/API still report 11 oversized-file
violations and 13 unassigned concurrent roots; baselines were not relaxed.
Commands, logs and source hashes are retained in qualification results.
The entire native-library parity objective remains active.

## r47: independent projective clocks on higher-degree rational curved seams

The existing rounded-patch owner now keeps clock proposals in a private clocks
child. For two degree-p full positive rational Bezier curves, fixed controls
and weights `alpha_i = gamma * beta_i * lambda^i` imply a common image under
`t -> lambda*t/(1-t+lambda*t)`. First adjacent weight ratios propose the positive
linear-fractional UV clock for ANY supported curve degree; no p-th root or
endpoint-only geometric approval is used. Opposite traversal uses the actual
reversed source weight order. Positive candidate UV weights preserve the
original natural side as an image; actual weights may be rounded.

Every proposed world/UV use still needs a fresh COMPLETE actual lift proof.
Coincident poles and matching endpoints do not suffice: nonprojective weight
changes that alter a curved image are refused. Exact proportional weight gauges
with identical poles retain the original exact route. A boundary-candidate scan
now also selects prepared proof for unpartitioned whole seams with different
clocks; it selects a route only, not boundary sharing. Original cap carriers,
independent knot clocks and authored weights remain literal. No surface fit,
control-point copying from a neighbor, or sampled match supplies authority.

Whole and partial curved boundaries with independent projective clocks therefore
receive actual rational face pcurves. These clocks survive Model codec and later
native edge splitting. The edge-refinement qualifier now also accepts the
EXISTING native full-original-chart monotonicity certificate after its original
injectivity criterion is unresolved, under the same remaining work allowance.
This is the same global certificate used to admit the saved cap constructor;
regularity, actual boundary/trim/sewing, naming and numerical material-role
gates remain mandatory. Periodic/pole quotient proof owners retain their prior
requirements. The public full-patch exact assembler keeps its original mode.

Verified cases include quadratic clocks in both directions, cubic nonunit and
transposed charts with rounded 1/3 UV weights, partial curved T neighbors,
nonpolynomial rational base seams, later native splits, rejection of altered
nonprojective curved images, and unchanged exact uniform gauges. This is not
arbitrary nonlinear boundary-clock inference, every degree/gauge configuration,
noncoplanar offset propagation or global solid embedding. Full library parity
remains active.

Final r47 qualification: 222 integration checks across 43 selected BRep
feature/refinement suites plus three legacy exact-sewing units passed on final
behavior (225 current checks). Six new projective cases are included once in
that matrix. The native NURBS/subdivision/cad-predicate matrix remains historical;
the mixed evidence aggregate 568 is not a fresh whole-workspace result.
Formatting/catalog pass, and all 11 clean comparison files remain unchanged.
Architecture/API gates still report 11 oversized-file violations and 13
unassigned concurrent roots; no baseline was relaxed. Commands, source hashes,
logs and the pre-fix downstream Split counterexample are retained in the
qualification results. Full native-library parity remains active.

## r48: continuous normal-offset candidate admission and a nonplanar patch skin

`nurbs_core::surface_offset::certify_candidate` covers the complete original UV
rectangle with outward interval bounds of the original unit normal and a
same-positive-rational-basis displacement carrier. Literal pole subtraction
roundoff is enclosed before the positive-basis convex-hull argument. No sampled
normal is an acceptance authority. Within requires every rectangle to pass;
Exceeds requires an interval-separated residual component; budget, depth and
unresolved normals return Unresolved without a claimed whole-domain upper bound.
Breadth-first subdivision avoids starving interior defect discovery at a
threshold boundary. Author domains, weights, periodic flags and knots are kept.

`brep_core::shell_thickness::thicken_patch_with_cap` admits an authored same-basis
cap using this certificate before closing the full nonplanar Bezier source with
four ruled sides. Normal-error tolerance and normal-cell budget are independent
of BRep sewing tolerance and its regularity/agreement/exact-work budgets. The
saved original and cap, all six chart regularities, exact boundary ownership and
continuous saved edge/lift agreement are checked. Legacy sampled construction
remains available with its original assurance. The new successful stop is
ConstructedCertifiedNormalDisplacement; neither Exceeds nor Unresolved returns
a model. A numerical/shifted cap is merely a proposal until admitted.

The error bound is pointwise normal displacement at identical original UV, NOT
closest thickness, wall thickness, offset injectivity or global embedded-solid
validity. This does not yet implement coherent multi-patch nonplanar offset
propagation, crease support intersections, self-trimming, arbitrary trimmed or
periodic/pole topology. General blend/corner networks and full library parity
also remain open. The frozen clean baseline is unchanged.

Final r48 validation: 227 BRep checks across 45 integration suites and 10
NURBS checks passed; final focused skin checks also passed. Formatting and the
200-entry catalog passed; frozen clean baseline retains all 11 files. Global
architecture still reports the same 11 oversized foreign files; API map still
reports the same 13 unassigned foreign roots. No gate was relaxed. Commands,
logs and owned-source hashes are retained in the qualification results.

## r49: continuously admitted nonplanar cap networks, including T seams

`brep_core::shell_thickness::freeform_network::{thicken,thicken_with_caps}`
construct closed skins of 1..32 coherently oriented full Bezier source charts.
The authored-cap path continuously qualifies original-UV normal displacement
for every cap, with one shared normal-cell budget across the whole network.
The automatic path proposes caps through the existing native offset owner and
falls back to its same-basis candidate when the proposal changes basis. Sampled
recognition is never admission: the identical original-domain continuous gates
and actual saved-network checks apply to automatic and authored caps alike.
Normal-error tolerance must be positive and smaller than absolute thickness;
this is still a pointwise offset theorem, not a closest-thickness theorem.

Common source-boundary preparation, exact/continuous shared-seam discovery,
T subdivision, external ruled walls, closed sewing and saved-model
qualification now live in `shell_thickness/network.rs`. The existing planar
constructor uses the same extracted implementation without losing its stronger
coplanar closest-cap-distance theorem. Existing Boundary/Cut/Limits paths stay
available; freeform_network reexports these types. Nonplanar surfaces do not
inherit the planar closest-distance theorem.

The freeform result owns immutable original sources/caps, orientations, limits,
normal certificates, exposed boundaries and cuts. `reverify` reconstructs the
actual model from these original inputs and compares geometry and ownership.
Original interior seams survive and receive no internal walls. All final saved
charts, natural UV regions, boundary lifts, naming and material-role diagnostic
gates remain required. Normal-cell, sewing-cell and exact-work limits remain
separate; each is shared within its operation rather than reset per patch.

Regressions cover both thickness signs on a genuinely noncoplanar bilinear
network, reversed original parameter orientation, a nonplanar T junction,
rational cylindrical radial caps, automatic native candidate admission,
incompatible cap seams, disconnected inputs, aggregate normal budgets, sewing
and exact-work exhaustion, source/cap retention, codec and fresh reconstruction.

Still open: intersection/extension of offset supports at creases, healing
incompatible proposals, general self-trimming, arbitrary authored trim/pole or
periodic source topology, globally embedded-solid proof and general blend/corner
networks. This implements a real nonplanar-network path, not general offset or
whole Plasticity parity completion.

Final r49 qualification: all 41 affected BRep checks across seven integration
suites passed, including seven new network regressions. The broader 227-case
r48 matrix and NURBS checks remain historical, not a new whole-workspace claim.
Formatting, 200-entry catalog and clean-baseline verification passed. The same
11 foreign oversized-file diagnostics and 13 unrelated unassigned NURBS roots
remain; no gate was weakened. Exact commands, logs and hashes are retained.

## r50: intersected offset supports at planar creases and multi-support vertices

`shell_thickness::miter_network::thicken` closes connected networks of full
positive rational bilinear charts whose complete literal control nets lie on
exact original planes. Oriented plane unit-normal intervals and independent
rounded translation error are reused from the existing planar offset owner.
One original vertex receives one displacement proposed by the native RRQR
minimum-norm solve against ALL incident oriented support equations. Literal
shared output vertices preserve cap-edge incidence instead of bridging support
gaps with extra internal walls. Reversed parameter charts are accounted for.

A native solver proposes, never certifies, these intersections. Every saved cap
pole is independently bounded in signed distance to its original support plane:
its interval projection relative to the independently verified translated cap
is added to the latter's rounding-inclusive error. Identical positive original
weights make the complete cap residual a convex combination of these bounded
pole residuals. Consequently the theorem covers every actual cap point, without
sampling. It certifies support-plane distance; tangential miter displacement is
intentional and this does NOT inherit the same-UV normal-offset or planar
closest-bounded-cap theorems.

Native whole-cap normal bounds additionally prove retained source orientation.
A caller-supplied limit bounds actual L1 vertex displacement / absolute
thickness using outward enclosures and a lower bound of the allowed product.
Near-opposed supports, consumed or reversed caps, inconsistent intersections,
nonplanar source controls and exhausted shared work refuse the operation.
All original/source-cap/wall chart, boundary/lift, trim and closed-role gates
remain mandatory through the same common network constructor. The result owns
sources, orientations, parameters, caps and actual topology; fresh `reverify`
reconstructs and compares all saved geometry and boundary ownership.

Regressions include perpendicular/oblique supports, both signs, three incident
support planes with one common miter vertex, nonunit original UV domains,
positive rational weights, reversed chart orientation, original retention,
closed topology, diagnostic volumes, codec, full fresh replay, miter limits,
near-opposed normals, consumed/reversed caps and both shared-work allowances.

Still open: general curved-support intersections/extensions, partial creases
whose moving intersection needs new cap subdivision/trims, global collision
self-trimming, arbitrary trimmed/periodic/pole source topology and globally
embedded-solid admission. General blend/corner networks and whole-library
parity remain active requirements.

Final r50 qualification: 47 BRep checks across eight affected integration
suites passed on final sources, including six new miter regressions. Formatting,
200-entry catalog and clean baseline passed. The same 11 foreign architecture
diagnostics and 13 unrelated unassigned NURBS roots remain; gates unchanged.
Commands, output logs and owned source hashes are saved in qualification results.

## r51: changed-basis trimmed offset caps and curved-support intersection skins

`nurbs_core::surface_offset::certify_mapped_candidate` removes the identical
basis restriction for candidates with an explicit increasing affine rectangle
map to an original source UV window. Candidate degree, weights, knots and active
UV domains may differ. Outward interval mapping covers the literal affine
correspondence, including endpoint and slope rounding; mapped intervals stay
inside the original source domain. Source and candidate jets must be continuous
and nonperiodic. The certificate covers the COMPLETE candidate chart, not the
parts of the original source outside the selected window.

Existing original offset Jacobian enclosures and candidate Jacobians bound the
residual derivative. An interval-enclosed midpoint plus the full derivative
remainder tightens the direct image residual; only the intersection of these
continuous enclosures can accept a cell. Numerical point evaluations do not
supply proof. All cells, including unsuccessful normal enclosures, share the
caller budget. Within requires full coverage; Exceeds requires a separated
residual component; unresolved work/depth/normals never return a finite global
error bound. This reuses the existing surface_offset native owner; no new
public NURBS root or generic math ownership was introduced.

`shell_thickness::mapped_network::thicken_with_caps` admits independently based
full Bezier caps with retained immutable source-window maps, original sources
and actual saved caps. Every cap needs fresh mapped offset qualification under
one aggregate normal-cell allowance. `thicken_from_windows` proposes native
offsets and restricts them to explicit windows; source corner identities only
propose tolerance-bounded endpoint coupling. The changed FULL caps and saved
shared curves are then independently qualified. Window/root discovery itself
is separate and is not claimed automatic by this constructor.

Common network walls retain the former exact path for identical boundary bases.
Different degrees/weights use the existing native aligned ruled-surface owner;
full prepared original-boundary/lift proof admits the actual saved wall uses.
In particular, connecting walls may need rational projective UV clocks when
source and trimmed-cap endpoint weights differ. No source chart is replaced by
a tessellation or a fitted proxy, and interior seams receive no walls.

Closed regression geometry includes a cylinder/plane crease: the offset
cylinder is restricted at an intersection proposal; the plane cap uses its
literal shared endpoints. Both caps, including the changed rational weights
and boundary clocks, pass whole-domain mapped normal bounds, chart/trim/lift
and closed-role checks. Native proposals from the explicit contact windows
also pass the same actual-model gates. Other tests cover changed degree,
independent nonunit UV domains, proper subrectangles, hidden interior drift,
wrong maps, gapped caps, degenerate normals, shared-budget stops, codec and
fresh immutable reconstruction.

This does not prove closest thickness, offset branch completeness, support
extension beyond original domains, arbitrary moving/nonrectangular trims,
periodic/pole topology, general self-trimming, global solid embedding or full
library parity. General branch discovery and trimmed-surface propagation remain
active requirements, alongside general blend/corner networks.

Final r51 qualification: 51 BRep checks across nine affected integration suites
and 14 NURBS checks across three suites passed. Formatting, the 200-entry
catalog and all 11 clean baseline files passed. Global gates retain the same
11 foreign oversized-file diagnostics and 13 unrelated unassigned public NURBS
roots; no gate was relaxed. Commands, logs and source hashes are retained.


## Paused checkpoint: r52 local automatic offset-pair search

Development was paused explicitly by the user. The complete native-library
parity objective is unchanged and NOT achieved. The readable Russian checkpoint
is `docs/design/plasticity-parity-checkpoint-2026-10-31.md`.

`shell_thickness::intersected_pair::thicken` is implemented: it discovers one
complete original seam, runs the existing native offset predictor with mandatory
interval section-root evidence, proposes rectangular cap windows and invokes
the full mapped-cap/closed-model constructor. Replay repeats the automatic
search as well as geometry construction. Source identity/prepared-lift discovery
spends the same sewing/exact allowance as subsequent construction. A section
witness is NOT promoted to global branch completeness.

Four focused final-source integration checks passed: automatic cylinder/plane
construction without authored caps/roots/windows; nonunit clocks and reversed
orientation; missing/ambiguous seams and exhausted search/normal work; refusal
of support extension and invalid search work. Log: `intersected-pair-r52.log`.
The latest broader evidence remains r51 (51 BRep + 14 NURBS). It was NOT rerun
after r52. Gauge/projective discovery fallback and transposed-axis/varying-root
cases require dedicated additional verification on resume, followed by affected
consumer tests and current global gate checks. No new validation was invented
for the pause checkpoint; source hashes and completed logs are retained.


## r52 resumed qualification: complete requested checks and fix clock-domain panic

The user resumed work specifically to complete intersected_pair verification and
run the r52 tests. All planned checks now pass: uniform weight gauges, exact
2:1 and rounded 3:1 projective source clocks, transposed first-source axes,
nonunit/reversed charts, and refusal of a varying nonrectangular full seam even
when its middle section has an independent interval root witness. A bowed source
boundary with coincident endpoints is independently refused as a shared curve.
The 3:1 case specifically spends continuous source-lift work; its approximate
projective clock cannot inherit exact identity authority.

These tests exposed a panic in the common network constructor: original and
trimmed target boundaries were looked up using identical physical UV intervals.
An uncut side has the same topological side identity even when its cap domain
changes. Complete uncut sides now correspond by side, with all actual curve/lift
and closed-model checks retained. Missing prepared pieces return a refusal;
partitioned sides with changed clocks require an explicit cut map and refuse
rather than guessing. Existing compatible planar/T/projective consumers pass.

Final evidence: 59 BRep checks in ten affected suites and 14 NURBS checks in
three suites passed. The focused eight-case intersected_pair target was rerun
after adding the rounded 3:1 variant and passed; those eight tests are counted
once. Formatting, catalog and 11 frozen clean files passed. Current architecture
and API gates still report the same 11 oversized-file diagnostics and 13
unassigned public roots; no rule was relaxed. Logs and hashes are in results.json.

The previous paused-checkpoint verification TODOs are complete. Next development
remains general contact-branch continuation and nonrectangular moving cap trims,
with full shared-seam and global solid qualification. Full parity is not achieved.


## r53: architecture/API recovery (1 November 2026; behavioral qualification running)

All eleven observed architecture diagnostics are resolved through twelve
cohesive extractions: original boundary/IGES tests, source-chart admission,
projected seam curves, proposal least-squares solves, STEP point inversion,
nonlinear inverse features, binomial coefficients and Bernstein jet tensors.
Original public paths remain; algorithms and numeric work limits are retained.
All new production children independently fit the existing size limit, including
untracked paths. Baseline edits only lower eight existing pins and remove one
now-small file; no size/layer rule or new exception was introduced. There are
still 124 grandfathered oversized files; a green ratchet is not elimination of
all legacy architecture debt.

Thirteen previously unassigned NURBS roots are reviewed and registered with
matching domain facade exports. API map now passes for 242 public modules and
200 catalog entries. Ownership rationale: nurbs-domain-ownership-2026-11-01.md.

Qualification exposed two copies of a fixture assumption: Boolean subtraction
of cubes need not produce a vertex-only pair of faces after coalescing. Both
negative topology tests now explicitly construct an arc contact and a vertex
contact, assert those prerequisites, duplicate topology and still require
refusal. No geometric certificate rule was relaxed. Both corrected cases pass.

The combined selected BRep suite passed all 240 tests (zero failures or
ignored cases) under opt-level 2 with debug assertions and overflow checks
retained. The same queue also passed all 108 cad-predicates library tests,
three NURBS collar unit tests and 15 central-graph integration tests. Numerical
limits, cases and assertions are unchanged. The redundant unoptimized run
was intentionally stopped only after the complete optimized suite passed;
its partial output is not a successful full debug qualification.

The three NURBS collar integration targets also passed all eight tests and
ten BRep consumer targets passed all 59 tests. All six optimized command groups
completed successfully: 433 native tests, zero failures or ignored cases.
Evidence: r53-qualification-final.json. Session 86092 is terminal exit 0;
do not resume or duplicate it. Qualification covers the r53 source snapshot;
new r54 sources have separate pending checks. Architecture, API/catalog,
ten architecture-policy tests, child formatting and the 11-file frozen clean
baseline pass. Full geometry-library parity remains active and unproved.


## r54: quadrilateral original-UV offset comparison (1 November; qualified scope)

`surface_offset::certify_quadrilateral_candidate` compares complete candidate
charts against original normal offsets under a bilinear map into four authored
original UV corners. Four straight UV boundaries may be nonparallel. Literal
corners stay inside the original chart; a whole-domain interval Jacobian must
prove positive orientation. Bounds retain original offset-jet chain rules for
both cross-coordinate terms, midpoint residual enclosures and whole candidate
coverage. The rectangular API shares the comparison engine and retains its
original validation/acceptance thresholds.

New regressions cover nonunit domains, projective candidate gauge, complete
positive bounds, an interior bulge with all boundary controls retained,
folded/reversed/outside/nonfinite maps, and unresolved budget exhaustion.
Three integration targets (new quadrilateral, existing mapped and candidate)
are queued in session 33777, behind the r53 build-directory lock. They are not
yet qualified. Evidence: r54-quadrilateral-offset.json and r54-offset-map-tests.log.
Architecture/API and frozen clean checks pass. This is the native comparison
prerequisite for nonrectangular caps; their BRep construction and sewing are
still absent. General curved trim maps, branch discovery and embedding remain
open under the original objective.


### r54 BRep integration (qualified scope)

`shell_thickness::quadrilateral_network::thicken_with_caps` retains literal
full source patches, owns each cap's original UV corner map, shares normal
proof cells across the network, then uses the existing closed-network owner.
Only exposed sides get aligned ruled walls; internal cap seams and every saved
chart, lift, closed incidence and replay are freshly qualified. Bounds concern
only the mapped cap region; walls and omitted source parameters have no constant
normal-thickness claim. The constructor accepts authored regions/caps, without
automatic root or region discovery.

Three new BRep regressions require: a closed single quadrilateral skin with
literal source/replay, adjacent quadrilateral caps with one shared seam and
no internal wall, and refusal of folded maps, changed caps and exhausted normal
budget. The new target plus legacy mapped/intersected_pair targets are queued
in r54-brep-quadrilateral-tests.log. No test success is claimed yet. The r53
consumer queue and r54 NURBS queue remain active; source snapshots and scopes
must stay separate because r54 sources were edited after r53 compilation began.


r54 NURBS qualification: session 33777 completed exit 0. All 16 tests pass:
5 new quadrilateral, 4 legacy mapped and 7 legacy candidate. Zero failures or
ignored cases. Captured source hashes match current files. BRep session 14496
is compiling its dependency configuration; the three BRep targets remain
unqualified until their own successful terminal result.


r54 final qualification: BRep session 14496 completed exit 0; all 15 tests
passed (3 quadrilateral, 4 legacy mapped, 8 legacy intersected_pair), with no
failures or ignored cases. Together with the 16 NURBS cases this is 31 tests,
not a whole-workspace or full-library parity claim. Captured source hashes
match. Architecture/API/catalog and frozen 11-file clean baseline pass.
Both r54 handles are terminal; authoritative evidence is
r54-quadrilateral-offset.json. Previous queued/running text records the earlier
checkpoint only. The constructor now admits authored nonrectangular bilinear
UV regions into qualified closed skins; automatic region/cap proposal, curved
UV maps, all-branch discovery, support extension and global embedding remain open.


## r55: curved rational original-UV correspondence (qualified scope)

`surface_offset::certify_parametric_candidate` accepts an authored positive
rational chart `(source_u, source_v, 0)` on the literal candidate domain.
Curved UV boundaries, independent map/candidate degrees and weights are allowed.
The map's full positive-weight control hull stays inside the original source.
A midpoint inverse proposes a constant preconditioner P; only interval-positive
symmetric part of P*J over the WHOLE candidate rectangle authorizes the map.
Integrating this uniformly positive quadratic form along parameter segments
proves global UV injectivity. Positive interval det(P) also proves orientation.
Valid maps outside this sufficient condition may refuse; no midpoint or sample
supplies the proof. Existing continuous offset jets then enclose the complete
residual with both UV chain terms.

Four new regressions cover curved rational sides/independent candidate degree,
a fold away from a positive midpoint Jacobian, interior cap drift with unchanged
boundary controls, and budget/invalid correspondence refusal. They run with
three legacy offset targets in session 25147, r55-parametric-offset-tests.log.
No test success is claimed yet. Architecture/API and 11-file frozen clean
baseline pass. General curved-map BRep integration, automatic region/cap
proposal, all contact branches, extension and world embedding remain open.


### r55 BRep parametric integration (qualified scope)

`shell_thickness::parametric_network::thicken_with_maps` owns the authored
rational UV maps and literal sources/caps, shares normal-error cells across
patches, then uses the existing closed-network sewing/qualification owner.
Only exposed sides receive native aligned ruled walls. Independent full cap
seams, chart/lift/closed incidence and immutable replay remain mandatory.
Three new tests cover a curved rational map with nonunit candidate clocks,
a pair with a shared cap seam and no internal wall, and independent sewing
refusal despite a successful looser normal-offset bound (plus reversed maps
and exhausted normal work). New and three legacy BRep targets are queued in
session 7383, r55-brep-parametric-tests.log, behind NURBS session 25147.
Neither pending suite is counted as passed. Maps and caps are still authored:
automatic region/cap inference, branch completeness, support extension and
world embedding remain open under the unchanged full-parity objective.


r55 NURBS qualification completed exit 0: 20 tests passed (4 parametric,
5 quadrilateral, 4 legacy mapped, 7 legacy candidate), zero failed/ignored.
Captured hashes match. Complete-map acceptance and refusal of an off-center
fold, cap interior drift, exhausted work and invalid original correspondence
are confirmed. BRep session 7383 remains live; its 18 expected cases are not
counted as passed. No whole-library parity claim follows from this slice.


Next after r55 BRep qualification: use existing native Bernstein nodal bases
and tensor reconstruction to propose single-Bezier caps over the authored UV
map. Sampled normal-offset points define only an interpolating polynomial
proposal, not an original-function polynomial identity or an error certificate.
Every actual rounded control net needs fresh complete parametric-offset
admission. Degree attempts, evaluations, coefficient work and all proof/final
sewing work require explicit shared limits. Automatic cap proposals will still
not establish automatic region discovery or contact-branch completeness.
No further geometry source changes were made while the r55 BRep run compiled.


r55 final qualification: BRep session 7383 completed exit 0. All 18 BRep
cases pass (3 new parametric, 3 quadrilateral, 4 mapped, 8 intersected_pair).
Together with 20 NURBS cases this is 38 successful tests, zero failed/ignored.
Source hashes match. Architecture/API/catalog, formatting of all new modules
and the frozen 11-file clean baseline pass. Both r55 sessions are terminal;
r55-parametric-offset.json is the authoritative current status. Earlier
queued/running paragraphs are historical checkpoints. This qualifies authored
curved rational UV maps satisfying the global chart admission criterion and
actual closed-network sewing, not automatic maps/caps, all contact branches,
extension or global embedding. Full parity remains open.


## r56: automatic cap proposal on an authored UV chart (geometry qualified)

`surface_offset::propose_parametric_candidate` proposes single-Bezier caps
using degree attempts 1,2,4,8,16 (ending at the caller's max degree). Native
interval Bernstein reconstruction defines an interpolation polynomial from
rounded sampled target stations. This is a polynomial proposal, not a claim
that the original normal-offset expression is polynomial or sampled evidence
of its error. Only fresh `certify_parametric_candidate` on the literal rounded
net can publish a cap. Reports retain each rejected/unresolved degree and
aggregate work; exhaustion returns no admitted cap. Separate evaluation,
coefficient and shared residual-cell limits are bounded and charged across
all attempts. Later degrees receive reserved proof allowance.

Four regressions require degree refinement on a curved polynomial UV map,
a fresh admitted cap of a rational cylinder offset, refusal/no cap under each
independent work/degree limit, and invalid map/budget rejection. They run with
four prior offset targets in session 76347 (24 expected cases, none counted
until actual success), r56-offset-proposal-tests.log. Architecture/API and
11-file frozen clean checks pass. Common UV-map validation was factored for
preflight without changing its geometric admission criterion. BRep automatic
cap construction, automatic map/region discovery and the wider original scope
remain open; no full-parity completion claim is made.


### r56 automatic BRep caps (geometry qualified)

`parametric_network::thicken_from_maps` accepts original sources and authored
UV maps, without caller caps. It generates candidates through the native
proposal owner, then freshly rechecks every actual cap with the existing
parametric/closed-network constructor. All proposal attempts and final normal
checks share one residual-cell allowance; half is reserved for final admission.
Station/coefficients budgets are also shared across every patch. Sewing/chart
proofs keep their independent existing limits. Original inputs remain literal;
failed generation, insufficient work or unsewn caps publish no partial model.
Saved replay repeats generation as well as actual complete geometry proofs.

Four new BRep cases require nonunit curved UV cap refinement with replay,
adjacent automatic caps and no internal wall, a rational curved source with
an automatically proposed cap, and refusal for degree/evaluation/coefficient/
normal limits. They run with four legacy targets (22 expected tests) in
session 41106, r56-brep-automatic-tests.log, behind NURBS 76347. Nothing pending
is counted as passed. Automatic UV-map/region discovery, all contact branches,
support extension and world embedding remain open under the full objective.


r56 NURBS qualification: session 76347 completed exit 0, 24 tests passed
(4 proposal + 20 prior offset cases), zero failed/ignored. Degree refinement,
the rational curved-source cap and independent budget/degree refusal cases
pass. Captured source hashes match. BRep session 41106 is compiling; its
22 expected cases remain unqualified until their own terminal result.


Branch-completeness follow-up evidence (not implementation): existing
surface_offset::intersection_candidates preserves all possible/pending product
boxes. Future coverage admission can combine fresh continuous-path tubes with
whole-box exclusion: only boxes wholly inside proved tubes can assign roots
to those paths; every other unexcluded region stays explicit. Caller-positive
reports must not substitute for fresh original-source proofs. Natural-domain
boundary endpoints, tangencies/poles and drive-axis switches require separate
admission and remain open. This does not change the still-running r56 source.


r56 BRep initial run terminated exit 101: three cases passed (including the
curved source), adjacent automatic caps left unpaired boundaries, and later
legacy targets did not run. Original red log: r56-brep-automatic-tests.log.
The automatic constructor now uses the existing interval-bounded proposal
corner coupling from mapped caps. Actual changed caps still require fresh
normal and closed topology proofs. Added literal shared-endpoint assertions;
no geometry tolerance, budget or acceptance rule was relaxed.
Corrected session 8532 runs the same 22 cases in
r56-brep-automatic-corrected-tests.log. BRep package opt-level=1, dependencies
opt-level=2, debug assertions/overflow checks retained. Qualification pending.
NURBS 24-test qualification remains passed.

Architecture recheck is currently red on concurrent changes outside this
geometry slice: DirectModeler.vue (3136 > 3127) and mainSolidResult.ts
(595 > 594). Those files and baseline were not changed here. Keep the current
red gate explicit; formatting of this correction passes. Geometry test
qualification continues independently in its existing session.


r56 final geometry qualification: corrected BRep session 8532 completed exit 0,
all 22 cases passed (4 automatic, 8 intersected_pair, 4 mapped, 3 parametric,
3 quadrilateral). Together with 24 NURBS tests this is 46 successful cases,
zero failed/ignored. Original red 41106 log remains historical evidence; no
partial failed-run counts are added. Shared seam correction and literal endpoint
regression pass. BRep uses opt-level=1, dependencies opt-level=2, assertions and
overflow checks retained, identical numeric inputs/budgets. All r56 sessions
are terminal. Source hashes match; API/catalog/formatting and 11-file clean
baseline pass. Explicit size checking includes untracked r54-r56 sources.

Global architecture remains red ONLY on the two previously recorded concurrent
UI files (DirectModeler.vue and mainSolidResult.ts); no pins were increased and
those files were not modified here. This is not an all-green workspace claim.
Automatic caps on authored maps are qualified; automatic region/map discovery,
all contact branches, extension, global embedding and the wider full parity
ledger remain open. Current authoritative evidence: r56-offset-proposal.json.


## r57: full-natural-domain offset root coverage (qualified scope)

`surface_offset::coverage::certify` replays caller numerical path requests
against original sources, sharing visited path cells. Complete natural-domain
product boxes are either excluded by full normal-offset enclosures or assigned
wholly inside fresh continuous uniqueness tubes. Partial overlap is not
coverage. Budget/precision limits retain every unexcluded/unvisited region.
No caller-positive path report is accepted. Root-set coverage/absence is proved
only when no pending region remains. This does not count connected branch
components, infer requests/maps, handle trimmed material or authorize topology.
Natural-boundary/tangent/pole regions are not silently dropped.

Six new tests cover complete transverse roots, full pending-domain preservation,
an insufficient middle-only path, complete separation/absence, boundary and
nonisolated contacts, and shared path work with invalid later-seed rejection.
They run with five previous offset targets (30 expected cases) in session
54358, r57-offset-root-coverage-tests.log. NURBS package opt-level=1,
dependencies opt-level=2; assertions and overflow checks retained. Qualification
pending. API/format/clean checks pass; global architecture still separately
tracks the two concurrent UI overgrowths without relaxing their pins.


r57 final qualification: session 54358 completed exit 0. All 30 NURBS cases
pass (6 root-coverage + 24 prior offset cases), zero failed/ignored. Full natural
root coverage on the transverse example and absence on separated charts are
proved; middle-only paths, boundary/nonisolated contacts and work exhaustion
retain pending regions. Requests are freshly replayed, and invalid later
requests are not hidden by earlier work. Source hashes match; explicit source
size checking includes untracked files. API/format/frozen 11-file clean checks
pass. Global architecture remains red on the same two concurrent UI files.
This qualifies root-set coverage, not automatic path/UV discovery, connected
branch counts, chart switching, trimmed membership, extension or topology.
No active r57 session remains. Evidence: r57-offset-root-coverage.json.


## r58: automatic offset-path requests (qualified scope)

`surface_offset::coverage::discover` requires no caller seeds or positive
reports. Complete-domain possible boxes propose section roots. Numerical
failures remain diagnostics; full-natural then local-drive requests need fresh
continuous-path proof. Opposite seed boxes alternate to avoid ordering one
component ahead of every other one under a small budget. A final independent
full-natural coverage walk replays the selected requests. Missed contacts,
limited queries and unsupported boundary/tangent regions must remain pending.
Candidate-search and qualification boxes have separate bounded allowances;
predictor queries are bounded and path cells are shared across initial proof
and final replay with half reserved for replay. This does not count connected
components, build a chart-switch graph, infer UV regions or authorize BRep.

Six new regressions require automatic transverse coverage, zero-query absence,
two disconnected contact sets, refusal of completeness after only one is found,
nonisolated/boundary/query limits and invalid limits even under separation.
They run with six prior targets (36 expected tests) in session 99548,
r58-offset-path-discovery-tests.log. Qualification pending. API/format/11-file
clean checks pass; global architecture still records concurrent UI overgrowth
without pin changes. Root topology, UV construction and remaining full parity
scope stay open.


r58 final qualification: session 99548 completed exit 0. All 36 cases pass
(6 discovery + 30 prior offset cases), zero failed/ignored. Automatic transverse
and two-contact discovery receive complete fresh root coverage. Limiting the
same two-contact problem to one selected path leaves pending regions, as do
nonisolated/boundary/query-limit cases. Separation needs no positive numerical
receipt. Source hashes match; API/format/clean and explicit untracked-source
size checks pass. Global architecture remains red on the same two UI files.
No active r58 run remains. Evidence: r58-offset-path-discovery.json.
Root coverage is not connected branch counting, chart switching, UV region
construction, material trim admission or BRep authority. These and the original
full parity scope remain open.


## r59: certified parameter-root connectivity (qualified scope)

`coverage::graph::certify` freshly replays coverage/path requests. Connections
require a native common-root witness inside both original uniqueness tubes;
nearby samples/endpoints do not weld paths. Full product bounds or disjoint
parameter boxes prove no shared root; a section-only exclusion cannot remove
a complete intersection box. Unresolved whole path pairs remain explicit.
Shared connection steps cover segment-pair tests and subdivision work, with
a separate total section-query bound, so large path products are not silently
materialized outside the allowance.

Each proved path is compact and connected over a closed driving interval.
Finite groups joined by common-root witnesses are connected. Only full root
coverage and disjointness of all remaining groups produce component_count.
This counts components of the original four-parameter root set, not world
curves on noninjective charts, trimmed material or a BRep branch graph.

Seven new regressions cover certified endpoint joins, duplicate paths,
overlapping different-axis charts, two automatic disconnected contacts, a
missing middle arc, connection-budget ambiguity and absence/nonisolated/invalid
work. They run with seven previous targets (43 expected cases) in session
59900, r59-offset-path-graph-tests.log. Qualification pending. API/format/clean
checks pass; the global architecture gate still records the concurrent UI
files. World embedding, natural-boundary/tangent admissions, UV regions and
original full parity scope remain open.


r59 final qualification: session 59900 completed exit 0. All 43 cases pass
(7 graph + 36 prior offset), zero failed/ignored. Common-root endpoint joins,
duplicate-path merging and different-axis overlapping charts prove one
parameter component. Two automatic disjoint contacts prove two. A missing
middle arc or exhausted connection allowance produces no component_count,
even if root coverage alone succeeds. Absence proves zero, nonisolated
unresolved sets do not. Source hashes match; API/format/clean and explicit
untracked-source sizes pass. Global architecture remains red on the same UI
files; no baseline change. No active r59 run remains. Evidence:
r59-offset-path-graph.json. World curves on noninjective sources, natural
boundary/tangent/periodic transitions, UV construction/material trims, BRep
and original wider parity scope remain open.


## r60: automatic original-UV contact curves (qualified scope)

`offset_contact_pcurve::fit_branch` first freshly proves one unique continuous
contact band, obtains certified root sections, then constructs degree-refined
polynomial UV proposals. Both literal rounded curves need full original-tube
correspondence proofs before either publishes. Shared section-query,
coefficient and proof-cell budgets cover every degree and both sides. No
sample, endpoint or coincident world chart supplies parameter authority.

Initial nonlinear test failed DegreeLimit: a fixed broad tube left implicit
jet bounds loose even on tiny drive intervals. New `certify_refined` uses
bounded certified uniform contractions, retaining the last reproducibly proved
input if precision stops further contraction. Root anchors stay on the ORIGINAL
tube. Geometry tolerance (1e-7 in the nonlinear case) and all limits are retained.
Legacy `certify` selects zero band refinements and keeps one band query per
visited node; refined proof counters include all band calls. Wrong-UV regression
now has coincident original AND offset world points with the same orientation,
yet the original parameter proof still refuses it.

Corrected session 2021 passed 48 cases (5 new + 43 prior), zero failed/ignored.
Final session 8304 adds counter-bound assertions to that same set;
r60-offset-contact-curve-fit-final-tests.log is pending until terminal output.
The initial red log and corrected passing log remain separate. API/format/clean
checks pass; global architecture retains the concurrent UI violations. This is
UV curve correspondence, not world lift agreement, material membership,
closed UV regions, map injectivity or BRep authority. Full parity remains open.


r60 final qualification: session 8304 completed exit 0. All 48 cases pass
(5 original-UV fit + 43 prior offset), zero failed/ignored. New counter assertions
bound uniform-band queries and preserve legacy one-query-per-node behavior.
The nonlinear contact reaches the unchanged 1e-7 UV tolerance; independent
fresh refined proofs admit both curves. Wrong UV sheets refuse even when both
original and offset world samples coincide. Degree/root/coefficient/proof
limits publish no partial pair. Source hashes match; API/format/clean and
explicit new-source size checks pass. The initial red log remains historical
and no duplicate passing runs are added. Global architecture still records
only the same two UI files. No active r60 sessions. Evidence:
r60-offset-contact-curve-fit.json. Closed UV-region construction, material/world
boundary authority, BRep use and the original full parity scope remain open.


## r61: closed original-UV contact strips (qualified scope)

`surface_offset::regions::build_strip` constructs a full rational UV map between
a contact trace and its projection onto one literal natural source boundary.
Source axis roles are retained; the contact map side is opposite the chosen far
side. Literal original trace knots remain and reversal is explicit metadata.
Whole-map injectivity/orientation and UV hull containment admit the rectangle's
closed Jordan image; canonical boundary extraction and exact joins supply four
CCW original-UV curves. A sampled polygon or endpoint tolerance does not admit.

`fit_pair` freshly proves/fits both original contact traces before both strips
can publish. Private original inputs/maps/loops/clock metadata support complete
replay. Side choice remains explicit. This is an untrimmed original UV region,
not material membership, world injectivity, preserved source seams or BRep.
Six regressions cover rational curved loops/classification, clock reversal,
fold/collapse/outside refusal, nonlinear paired regions with natural side IDs,
nonunit clocks and partial-fit refusal. They run with nine previous targets
(54 expected cases) in session 29967, r61-offset-uv-region-tests.log.
Qualification pending; API/format/clean pass. Global architecture still records
the concurrent UI files without new exceptions. Full parity remains open.


r61 final qualification: session 29967 completed exit 0. All 54 cases pass
(6 UV-region + 48 prior offset), zero failed/ignored. Rational Jordan-loop
construction/classification, exact joins, explicit reversed clocks, source-axis
side indices, nonlinear contact pairs and independent nonunit charts pass.
Folds, collapsed/outside strips and incomplete pair proofs refuse. Source hashes
match; API/format/clean and explicit untracked-source sizes pass. Global
architecture still records the same two UI files without pin changes. No live
r61 runs remain. Evidence: r61-offset-uv-regions.json. Closed UV regions do not
supply world contact/lift agreement, existing material-domain membership,
original seam topology, source embedding or BRep authority. These and the
original full parity scope stay open.

Next actual contact binding needs a shared WORLD curve with full-interval
error against the original offset-contact root (implicit world tangent plus
certified root anchors), not merely close UV traces or independently fitted
cap edges. Any shared cap-edge proposal still needs fresh complete normal
bounds and actual saved BRep boundary/lift/closed-role qualification.


## r62: complete world-contact curve error (qualified scope)

`surface_offset::contact_curve::certify` bounds actual candidate(t) minus the
original offset-contact world root over the complete driving interval, in mm.
Original-tube certified root anchors plus implicit world tangents supply the
continuous residual bound. Matching endpoints, UV closeness or sampled points
cannot admit. `fit` constructs degree-refined polynomial world proposals and
publishes only after complete verification; query/coefficient/cell allowances
are shared across all attempts. Literal nonunit original clocks are retained.

Uniform-tube refinement now has one native owner in offset_contact_tangent;
UV proofs reuse the same mechanism and preserve their legacy zero-refinement
behavior/counters. World coincidence does NOT prove UV-sheet identity, source/
curve regularity, material membership, embedding or BRep. A continuous world-
agreement test with the wrong original UV sheet must still fail the separate
parameter certificate.

Six new regressions cover independent nonunit clocks, nonlinear world-root
curves, interior drift despite matching endpoints, each budget/degree stop,
world/UV authority separation, absent/nonisolated/wrong-clock inputs. They run
with ten prior targets (60 expected cases) in session 25696,
r62-world-contact-curve-tests.log. Qualification pending. API/format/clean pass;
global architecture still records the concurrent UI files. Shared cap-edge
binding and actual BRep authority remain open in the full parity objective.


r62 final qualification: session 25696 completed exit 0. All 60 cases pass
(6 world-contact + 54 prior offset), zero failed/ignored. Nonunit clocks,
nonlinear complete-world error at 1e-7, interior drift, every fit budget,
continuous world agreement without UV-sheet authority and absent/nonisolated/
wrong-clock inputs pass. Shared uniform-tube helper preserves existing UV
proofs and legacy counters. Source hashes match; API/catalog/format/clean and
explicit new-source sizes pass. Global architecture remains red on the same
concurrent UI files. No live r62 runs. Evidence: r62-world-contact-curve.json.
Position agreement does not authorize a regular/embedded world edge or a BRep.
Next: bind the SAME admitted world curve into both actual cap boundary nets,
then reprove complete original normal error, actual boundary/lift agreement
and closed material-role/model qualification. Full parity remains open.


### r63 — общая мировая контактная граница в фактическом BRep

shell_thickness::contact_pair::thicken строит две UV-области и caps по
исходной контактной полосе, обнаруживает полный исходный шов и привязывает
одну мировую кривую к обеим фактическим границам caps. Степени согласуются,
reversal часов учитывается. После изменения полюсов повторяются мировая
ошибка, полное mapped normal displacement и boundary/lift/closed-role gates.
Оригиналы сохранены, replay заново выполняет весь конструктор.

27 тестов прошли, 0 failed/ignored, exit 0: пять новых и 22 прежних.
Проверены плоскости, нелинейный цилиндр/деформированная стенка, transpose,
буквальное равенство полюсов/весов обеих границ общей кривой, отсутствие
внутренней стенки, полный replay, shared budgets и отказы неверной полосы/шва.
Допуски 1e-7 мм для шва и 0.005 мм normal на нелинейной паре сохранены.
Формат/API/каталог/11 чистых файлов/размеры source проходят. Общая архитектура
по-прежнему красная на двух сторонних UI-файлах; pins не повышались.
Доказательства: r63-contact-pair.json и r63-contact-pair-tests.log.

Полоса Band пока задаётся вызывающим кодом. Это локальный замкнутый skin,
а не полное автоматическое покрытие естественного offset, material sectors
или доказательство global embedding. Следующие задачи: автоматический выбор
полос/концов, полнота исходных ветвей, material topology и support extension.
Полный исходный parity scope остаётся открытым. Все r63-прогоны завершены.


### r64 — автоматические исходные пути в локальный BRep

contact_pair::discover больше не требует Band, seed-точек, driving-интервала,
UV-карт, caps или мировой кривой. Native discovery независимо перепроверяет
пути и полноту естественных исходных chart-доменов; объединённые трубки лишь
предлагают полосу для полного конструктора r63. Успешная локальная модель
не стирает pending и не получает полноту контакта автоматически. Полнота
допускается только при свежем full-natural coverage и включении всех owned
boxes в одну успешно сконструированную uniform band. Положительный полный
BRep-кейс этой проверки пока не квалифицирован; embedding из неё не следует.

Нелинейный тест сначала обнаружил ноль путей: широкие proposal-окрестности
не давали подтвердить сечение. Discovery теперь пробует до девяти меньших
окрестностей, каждый раз заново проверяя сечение и расходуя прежний общий
predictor_queries budget. Неудачные численные кандидаты не исключают области.
Проверенные меньшие padding сохраняются в запросе пути для независимого replay.
Исходный красный лог сохранён, допуски и положительные критерии не ослаблены.

50 тестов прошли, 0 failed/ignored: 30 BRep + 20 NURBS, включая четыре новых.
Проверены автоматические плоскости и нелинейная пара цилиндр/стенка без Band,
буквальная общая граница, fresh normal/world/closed gates, отсутствие контакта,
исчерпание budget, сохранение pending, replay и прежние topology/refusal cases.
Шов остаётся 1e-7 мм, normal на нелинейной паре 0.005 мм.

Бюджеты построения задаются на каждую попытку независимо; max_constructions
ограничивает все попытки, включая неудачные. Это явно не один общий allowance.
API/каталог/формат/11 чистых файлов/размеры новых source проходят. Общая
архитектура красная на двух сторонних UI-файлах, pins не повышались.
Итог: r64-automatic-contact-pair.json, r64-brep-tests.log, r64-nurbs-tests.log.
Все процессы r64 завершены. Открыты natural-boundary endpoints, полнота всех
ветвей, material-sector selection, support extension, global embedding и весь
остальной исходный parity scope. Следующий этап — конечные точки контакта
на естественных границах, без превращения локального успеха в global receipt.


### r65 — исходные граничные корни и полнота восьми естественных границ

surface_offset::boundary_contact::certify фиксирует буквальную естественную
границу выбранного исходного chart. При необходимости меняется порядок
источников, а UV witness восстанавливается в исходном порядке. Независимые
section contraction и полный inward-band/tangent доказывают изолированный
корень и, когда удалось, регулярную ветвь с внутренней стороны. Не вычисляется
продолжение поверхности за исходным support.

discover ищет все восемь границ без seed-точек. Затем отдельный полный обход
каждой границы исключает box исходными offset bounds либо целиком относит его
к проверенной unique-root tube. Пересечение tube не считается ownership.
Лимиты/неразрешённые области сохраняются; дубликаты witness не превращаются
в число компонент. Положительно проверено полное покрытие граничных корней
для плоскостей и нелинейной пары четверть цилиндра/деформированная стенка.

BRep AutomaticReport теперь содержит boundary_contacts с отдельными явными
лимитами поиска. Полный replay сравнивает корни, исходные трубки, результаты
full-face qualification и сохранённую модель. Полнота граничных корней не
выдаётся за полноту внутренних ветвей: прежние pending/false сохраняются.

57 тестов прошли, 0 failed/ignored, exit 0: 27 NURBS + 30 BRep, семь новых.
Проверены literal boundary, оба порядка источников, transpose и signed normal,
nonunit clocks, регулярный inward collar, автоматический нелинейный конец,
full boundary coverage/absence, лимиты, nonisolated/corner refusal и BRep replay.
Две первоначальные ручные положительные tube были слишком широкими для
contraction; уточнены только proposal-интервалы тестов. Критерии не ослаблялись;
автоматический нелинейный поиск прошёл с первого прогона. Красный лог сохранён.

Хеши source до/после совпали. Формат/API/каталог/11 чистых файлов/размеры
source проходят. Общий architecture gate остаётся красным на двух сторонних
UI-файлах; pins не повышались. Итог: r65-boundary-contact.json,
r65-nurbs-tests.log, r65-brep-tests.log. Все r65-процессы завершены.

Следующий этап — доказанная принадлежность концов внутренним ветвям,
обработка natural-boundary trims со сменой driving chart и полное покрытие
исходного контактного множества. Открыты material-sector selection, corner/edge
root events, support extension, global embedding и остальной исходный parity
scope. Полное покрытие границ само по себе не доказывает эти требования.


### r66 — принадлежность концов внутренним путям и автоматические мосты

attachment::certify заново проверяет исходный граничный корень, регулярную
inward-ветвь и все внутренние пути. Связь требует нового общего UV-корня
в пересечении обеих uniqueness tubes; близость в мире или концы численной
траектории не являются доказательством. Доказанная непрерывность полного
пути и inward-ветви связывает весь путь с буквальным граничным концом.

attachment::bridge::connect предлагает недостающую среднюю дугу из независимо
уточнённого inward-anchor к внутренней точке целевого пути. Пользователь
не задаёт anchor или мост. Публикуемая связь требует полного adaptive path/join
proof, отдельного общего корня с целевым путём и отдельного общего корня с
концом. Ошибки/лимиты остаются диагностикой. Proof::bridge_paths хранит исходное
владение, AutomaticReport::connect_endpoint связывает автоматически найденный
конец с автоматически найденными внутренними путями. Replay полностью повторяет
построение и положительные проверки.

Существующий original-root relation вынесен из coverage::graph в приватный
переиспользуемый helper без изменения критериев. Shared relation steps/sections
сохранены. У автоматических мостов initial replay имеет свой budget; каждая
ограниченная попытка имеет независимые явные proposal/path-link/endpoint-link
allowances. Неудачные попытки не исключаются из max_attempts.

64 теста прошли, 0 failed/ignored, exit 0: 34 NURBS + 30 BRep, семь новых.
Проверены оба порядка исходных UV, нелинейная пара цилиндр/стенка, недостающая
средняя дуга, лимиты, два независимых link proof, полное повторное построение
и автоматический BRep-вызов без ручных координат. Совпадающие мировые ветви
на разных исходных UV-листах не склеиваются даже попыткой автоматического моста.

Хеши source совпали. Формат/API/каталог/11 чистых файлов/размеры новых source
проходят. Общая архитектура по-прежнему красная на двух сторонних UI-файлах;
pins не повышались. Итог: r66-endpoint-attachment.json, r66-nurbs-tests.log,
r66-brep-tests.log. Все процессы завершены.

Это connected subsets исходного четырёхпараметрического root set. Disjoint
collar/path sets не считаются глобально несвязными компонентами; неизвестная
дуга может связать их вне проверенной области. Полнота interior roots, полные
контуры обрезки со сменой driving chart, согласованные piecewise UV/world lifts,
материал, support extension и global embedding ещё не доказаны. Следующий этап
— полные исходные contact trims и согласование clocks/lifts; весь остальной
исходный parity scope остаётся открытым.


### r67 — фактические C0 contact chains и прямые original-offset lifts

contact_curve::chain::build строит фактические мировые кривые и две UV-кривые
в исходном порядке источников. Поддерживаются смена владельца/оси исходного
clock, обратный обход и nonunit clocks. На стыках обе UV-пары и мировые полюса
буквально общие. Независимая uniform unique regular root branch содержит оба
исходных endpoint-корня и всю малую дугу между ними. Ошибка оценивается для
всей дуги, clock derivative и исходные enclosure доказывают порядок/ориентацию.
После изменения полюсов полные UV и world-root proofs повторяются.

contact_curve::lift::certify отдельно проверяет world(t)-offset_source(uv(t))
в мм на всём clock. Center interval и обе UV chain-rule составляющие с
исходными incident-knot jets ограничивают остаток. Положительный rational
control hull сначала доказывает нахождение фактической UV-кривой на исходном
chart; только после этого разрешено пересечь outward-overshoot enclosure с
доменом. C1 source knots покрываются непрерывностью normal field и всеми
incident производными. UV-ошибка не подменяет физическую lift-ошибку.
BRep AutomaticReport::fit_path строит такую цепочку по автоматически найденному
пути без ручных полюсов/контактных кривых; все положительные проверки повторяются.

Новый alias-sheet тест сначала не мог квалифицировать отдельный UV-участок
(~90 секунд). Добавлена точная coordinate-plane normal identity по буквальному
control net и строгому знаку якобиана; близкие к плоскости поверхности не
считаются плоскими, folded/singular boxes не получают normal authority.
Дополнительно исправлены proposals вложенных root-box refinements: почти
постоянная координата не замораживает остальные на пределе outward rounding.
Proposal остаётся вложенным в последний успешный input и каждый раз проходит
тот же строгий root proof. Тот же тест затем прошёл за 0.16 секунды при прежнем
1e-7 UV/mm. Красные логи сохранены, tolerance/positive gates не ослаблялись.

103 теста прошли, 0 failed/ignored, exit 0: 57 NURBS integration + 16 прежних
source unit + 30 BRep. Двенадцать новых случаев: actual shared poles, owner
switch/reversal/nonunit clocks, нелинейный clock switch, oversized gap/backtrack
refusal, independent limits, совпадающие мировые разные UV-листы, прямые lift
proofs, физический масштаб, interior bulge/support escape, C1 knots и exact-vs-
near-planar normals. Старые periodic/singular/rational/refinement gates сохранены.
Codec-only unit harness не собирался из-за существующих sweep-тестов с transport
reference без feature guard; unit-фильтр пройден с требуемыми default features.
Это не runtime-проверка transport/UI.

Хеши source совпали. Формат/API/каталог/11 чистых файлов/явные размеры source
проходят; новых exceptions нет. Общий architecture gate красный на прежних двух
сторонних UI-файлах. Итог: r67-contact-chain-lift.json и три r67 test logs.
Все процессы завершены. Limits явно независимы для каждого piece/fitter и
каждого из пяти final proof checks; это не один глобальный allowance.

Цепочка остаётся открытой C0-аппроксимацией. Замкнутые простые Jordan trim
loops, material-sector selection, полное естественное root coverage, G1/G2
стыки, support extension, global embedding и BRep sewing из неё не следуют.
Следующий этап — замкнутые исходные UV trim loops/Jordan qualification и
материальные сектора; весь остальной исходный parity scope остаётся открытым.


### r68 — замкнутые исходные UV Jordan loops с фактическими world lifts

Chain::close_with принимает предложенные исходные UV-кривые закрытия;
close_to_boundary предлагает rails до выбранной буквальной естественной
границы. Исходный contact chain полностью перестраивается. Положительный
control hull доказывает нахождение всего UV-контура на исходном chart;
exact endpoint joins и полный trim_simplicity audit доказывают Jordan simplicity.
Ни samples/polygon, ни близость концов не считаются закрытием или простотой.

contact_curve::lift::fit уточняет мировую полиномиальную proposal-кривую
в исходном UV clock. Общие world endpoint poles задаются буквально, но
принимаются только после полного interval lift proof. Actual world-loop closure
и direct normal-offset lifts каждой конечной кривой проверяются независимо
повторно. Region::classify использует исходный nonzero winding; boundary bands
остаются Unresolved. BRep AutomaticReport::close_path строит такой регион по
автоматически найденному пути без ручных полюсов или контактных кривых.

68 тестов прошли, 0 failed/ignored, exit 0: 38 NURBS + 30 BRep; семь новых.
Проверены owner switch и обе исходные UV-стороны, reverse/nonunit clocks,
нелинейный цилиндр/стенка, literal UV/world closure, полные lifts, winding
inside/outside/boundary, crossing/escape refusal, неверные forced endpoints,
все независимые бюджеты и полное повторное построение. Допуски 1e-7 UV/mm
сохранены. Первые шесть новых тестов и окончательная расширенная матрица
завершились успешно; промежуточные прогоны не суммируются с итогом.

Lift-fit делит evaluations/coefficient/proof budget между степенями. Каждая
кривая закрытия имеет свой явный fit allowance, каждый final lift — свой.
Simplicity pair/cell work общий для всего UV-контура. Начальная chain fit и
closure fresh rebuild имеют независимые stage allowances, это не один общий
budget. Нулевые rails исключаются только по буквальному равенству концов.

Хеши source совпали. Формат/API/каталог/11 чистых файлов/размеры новых source
проходят; exceptions/pins не повышались. Общая архитектура остаётся красной
на двух сторонних UI-файлах. Итог: r68-closed-contact-region.json,
r68-nurbs-tests.log, r68-brep-tests.log. Все процессы завершены.

Доказан один фактический UV Jordan loop на выбранном исходном chart и его
замкнутые мировые lifts. Выбор far boundary пока proposal вызывающего кода,
а не автоматический material sector. Отверстия/вложенность, global source/world
embedding, cap construction, полное естественное root coverage и solid/BRep
sewing отдельно не доказаны. Следующий этап — holes/nesting и материальные
сектора исходных UV-регионов, затем cap/trim sewing и global embedding.
Весь остальной первоначальный parity scope остаётся открытым.


### r69 — доказанная вложенность отверстий/островов и UV material parity

trim_region_audit::nesting доказывает каждый исходный Jordan loop, полное
разделение каждой пары кривых разных границ и containment интервальным winding
по точным clamped endpoints. Разделение делает winding постоянным на всей
связной границе; Jordan theorem даёт вложенность ограниченных interior regions.
Parent/depth/components публикуются только после проверки полного transitive
laminar forest. Несколько внешних UV-компонент и произвольная допустимая
глубина поддерживаются в явных loop/curve/work bounds.

Even/odd правило определяет отверстия и острова независимо от ориентации
входных loops. Canonical orientation выражается порядком/направлением coedges;
исходные UV-полюса, веса и knots не изменяются, reverse-кривая не округляется.
Касания/пересечения/дубликаты/лимиты остаются unproven; частичная parity не
становится Inside/Outside. Некорректный поздний input проверяется до budget exit.

Region::with_nested переносит sealed immutable outer Region в Compound.
Дополнительные границы должны находиться строго внутри исходного outer.
Forest и все конечные direct world lifts строятся заново; новые мировые loops
имеют буквальное замыкание. Полный replay также перестраивает исходный outer.
Это UV material selection, не вывод объёмных секторов материала.

76 тестов прошли, 0 failed/ignored, exit 0: 46 NURBS + 30 BRep, восемь новых.
Проверены unordered deep nesting, несколько UV-компонент, rational/nonunit
definitions, coedge orientation без изменения геометрии, crossing/touch/duplicate
refusal, shared budgets и validation поздних inputs. Составные области правильно
классифицируют outer/hole/island/deeper-hole и сохраняют boundary Unresolved.
Рациональное отверстие на nonlinear offset и отверстие в автоматически
полученном BRep-регионе прошли literal world closure, lifts при 1e-7 мм и
полное повторное построение. Промежуточные прогоны не суммируются с итогом.

Nesting pair/cell/domain allowance общий для всей квалификации; трудная пара
получает оставшийся общий budget с учётом actual visits, без искусственного
малого per-pair quota. Неподтверждённый resource failure расходует выделенный
allowance и не даёт положительного proof. Fit/final-lift budgets явно отдельны
для каждой дополнительной границы; размеры inputs ограничены до копирования.

Хеши source совпали. Формат новых/извлечённых модулей, API/каталог, 11 чистых
файлов и явные размеры source проходят. Старый trim_region_audit parent получил
только module declaration. Pins/exceptions не повышались. Общий architecture
gate остаётся красным на двух сторонних UI-файлах. Итог:
r69-nested-offset-regions.json, r69-nurbs-tests.log, r69-brep-tests.log.
Все процессы завершены.

Открыты объёмные material sectors, global source/world embedding, cap
regularity/construction и solid/BRep sewing. UV-компоненты не считаются
компонентами мирового тела. Следующий этап — объёмные роли и cap/trim
construction/sewing на допущенных compound UV regions; весь остальной
первоначальный parity scope остаётся открытым.


### r70 — фактический offset carrier и открытые BRep cap faces

Compound::cap строит фактическую NURBS-поверхность: сначала существующий
analytic/basis proposal, затем degree-refined proposal. Sampling/exact flags
предложения не дают допуска: заново доказываются полное normal displacement
от исходной поверхности, whole-chart injectivity и continuous agreement всех
неизменённых мировых rims с фактическим carrier. Authored candidate проходит
те же три независимых gate без подмены fallback-поверхностью.

contact_pair::cap строит настоящие BRep faces для even-depth UV-компонент и
их непосредственных holes. Каждая несвязная компонента получает отдельную
открытую shell; Bodies отсутствуют. Original world edges и raw UV definitions
сохраняются. Для обратных coedges создаётся фактическая reverse pcurve и
доказывается весь affine clock correspondence с оригиналом; после этого
заново доказываются вложенность всех actual oriented loops и agreement всех
actual coedges с фактической поверхностью. Full replay повторяет исходные
contact/root/loop/lift proofs, proposal/qualification и BRep assembly.

NURBS API остаётся внутри существующего владельца domains::offsets::surface_offset;
нового публичного корня не добавлено. Геометрия открытого листа не считается
замкнутым телом. Автоматический выбор объёмных material sectors, global world
embedding, согласование нескольких caps и sewing замкнутого solid остаются
открыты; полный первоначальный parity scope не сокращён.

83 теста прошли, 0 failed/ignored: 50 NURBS + 33 BRep, семь новых случаев.
Повторы sheet suite не суммируются. Проверены independent normal/rim/chart
refusals, basis и polynomial carriers, budgets, holes/islands, rational nonunit
orientation, actual FaceDomain classification и полный nonlinear discovery →
compound hole → actual carrier → BRep face → original replay. Первоначальный
красный тест disconnected shell исправлен отдельными open shells компонентов;
его лог сохранён в r70-initial-sheet-failure.log.

Source hashes совпали. 11 чистых файлов неизменны; scoped format, 242 API roots,
200 catalog entries и explicit new-source sizes проходят. Global architecture
остаётся красной на двух прежних UI-файлах, pins/exceptions не повышались.
Evidence: r70-actual-offset-caps.json и r70-*-tests.log. Все процессы завершены.
Следующий этап: объёмные material sectors, совместные cap boundaries/trim sewing
и global embedding перед выдачей замкнутого solid. Полный parity не достигнут.


### r71 — автоматическая сшивка полных границ actual cap sheets

contact_pair::cap::sewing принимает sealed Sheet объекты и ищет пары полных
рёбер между входами. Буквальное равенство концов только индексирует candidates;
полное mapped-clock comparison исходных мировых кривых доказывает
correspondence. Physical clocks могут различаться и идти в противоположных
направлениях. Частичные рёбра, непопавшие в индекс near-endpoint gaps и
interior-only contacts этим этапом не считаются найденными/сшитыми.

Canonical edge остаётся исходной кривой одного входа; исходные другие Sheet
и их CV/weights/knots сохраняются в Network. Для финальной модели все actual
pcurves заново сопоставляются с canonical edge на всей длине. Union касается
только буквально совпавших вершин допущенных рёбер. Инцидентность, связность,
vertex fans и согласование FaceUse по всем циклам проверяются заново.
Несвязные компоненты получают разные shells. Ambiguous mates, inconsistent
orientation и exhausted work отказывают без partial Network.

Даже closed edge incidence не создаёт Body: совпадающие indistinguishable
faces остаются отказом naming/topology gate. Глобальные world intersections,
объёмные material sectors и embedding не следуют из этой сшивки. UV/cap/root
входы replay целиком, затем заново строится Network и проверяется exact model.
Владелец API — существующий BRep contact_pair::cap, новых NURBS roots нет.

44 теста прошли: 40 BRep + 4 NURBS, семь новых случаев. Повторные финальные
10 sewing/sheet случаев считаются один раз. Проверены adjacent reversed seam,
две различные contact cap surfaces, independent physical clocks, near-gap
без snap, endpoint-equal/curve-different refusal, ambiguous mates, duplicate
face refusal и shared budgets. При общем correspondence allowance до 1e6
каждый native pair query получает не более своего лимита 100000; отдельный
финальный тест проверяет увеличенный общий allowance.

Изначальный clock fixture также дублировал indistinguishable faces и был
правильно отклонён stable topology IDs; положительный fixture исправлен для
проверки clocks на разных face regions. Naming gate не обходился, duplicate
refusal сохранён отдельным assertion. Source hashes совпали. Scoped format,
explicit source sizes, 242 API roots, 200 catalog entries и 11 clean files
проходят. Global architecture по-прежнему красная на двух прежних UI-файлах;
pins/exceptions не повышались. Evidence: r71-cap-network-sewing.json.
Все процессы завершены. Открыты partial-edge/subdivision trim sewing,
volumetric material sectors, global embedding и feature-history lineage сети.
Остальной первоначальный parity scope не сокращён.


### r72 — subdivision cap rims и частичный Т-стык

Sheet::split_edge / split_edge_to_vertex / split_edge_at_vertex сохраняют
sealed исходный Cap и историю разрезов. Мировые children и каждый actual UV
child сравниваются непосредственно с исходными raw cap curves, с сохранением
physical domains, reflection обратных coedges и весов. После каждого разреза
заново доказывается nesting всех actual UV loops и agreement всех current
coedges с actual surface. Повторные разрезы не меняют authority на округлённый
промежуточный proxy. Replay снова строит исходные contact/root/region/cap/sheet
и повторяет все разрезы, включая numerical parameter search.

edge_partition::partition_to_vertex допускает общий literal vertex только как
предложение: обе полные части проверяются относительно оригинального edge
и всех его исходных support surfaces. Это не endpoint-only snap. Existing
partition API сохраняет прежнее поведение. Поиск параметра samples/Gauss-Newton
имеет явный evaluation budget; его численные расстояния не дают admission.

Тест частичного стыка строит две короткие contact caps, передаёт их shared
vertex длинному rim, автоматически предлагает параметр, доказывает разрез и
сшивает три actual faces по трём complete seams. Начальный strict endpoint
index находил лишь один seam из-за расхождения vertex на последнем бите;
исправлен proposal/qualification общего vertex, index не ослаблялся.
Ребро и target vertex выбирает вызывающий API; полный автоматический поиск
всех partial/interior contacts пока не следует из этой возможности.

BRep owner — existing edge_partition и contact_pair::cap::partition;
публичных NURBS roots не добавлено. Split lineage сохраняется внутри Sheet;
перенос полного persistent feature-history lineage через финальный Network
остаётся отдельным этапом. Closed volume/material/global embedding не выданы.

66 тестов прошли, 0 failed/ignored: 62 BRep + 4 NURBS, семь новых случаев.
8-case native partition run заменён финальным 9-case run, повторы не
суммируются. Проверены rational reversed hole/nonunit clocks, повторные cuts,
прямое original ownership, сохранение Split lineage, automatic parameter
proposal и весь 3-face T-junction replay. Отдельно проверен shared-vertex edit
двух incident supports: исходный Model неизменён, обе lift-проверки проходят,
far vertex и wrong pcurve cut отклоняются. Нелинейная BRep регрессия завершена.

Source hashes совпали. Scoped format, explicit source sizes, 242 API roots,
200 catalog entries и 11 clean files проходят. Общая архитектура остаётся
красной на двух прежних UI-файлах; pins/exceptions не повышались.
Evidence: r72-cap-rim-subdivision.json, r72-*-tests.log; initial T failure и
vertex diagnostic сохранены. Все процессы завершены. Следующий этап:
automatic partial/interior contact discovery, global embedding, volumetric
material roles и closed-solid qualification. Остальной полный scope открыт.


### r73 — автоматическое conform по соседним vertices

contact_pair::cap::sewing::conform::build сам перебирает исходные vertices
соседних Sheets и текущие edge pieces; вызывающий код больше не выбирает
ребро/vertex/параметр для обычного partial T-junction. Successful cuts добавляют
только буквальные исходные target positions, поэтому новых непроверенных
позиций для следующего прохода не изобретается. Positive rational control hull
с outward-expanded tolerance даёт строгий broad-phase exclusion. Совпавший
literal endpoint лишь означает, что vertex уже есть в этой границе.

Остальные пары получают bounded samples/Gauss-Newton parameter proposal.
Только полные direct-original world/UV, actual support lifts и actual nesting
допускают разрез. Затем выполняется whole-edge sewing, включая canonical
edges и final actual lifts. Неудачные proposals и complete-proof refusals
сохраняются отдельно; они не становятся доказанным отсутствием контакта.
Даже пустой список refusals не даёт full interior-contact coverage.

Counters имеют общие pair/candidate/evaluation allowances и bounded успешные
splits; каждая complete subdivision имеет явные независимые stage budgets.
Неуспешный numerical call без visit count расходует весь выделенный quota;
успешный — actual evaluations. Вложенные options проверяются до поиска даже
при отсутствии кандидатов. Все admitted cuts и их parameter search replay
из исходного Cap; target vertex ownership перепроверяется по retained соседу.
Полный повторный перебор отказавших proposals этим replay не обещается.

Автоматический Т-стык проходит при обоих порядках inputs. Rational/nonunit
partial rim и in-hull off-curve candidate проверены; последний остаётся
CompleteSubdivision refusal, а не ложным join или proved absence. Near endpoints
не snap-ятся автоматически. Full interior curve/face intersections, material
sectors, global embedding, closed solids и Network feature lineage открыты.
API принадлежит existing BRep contact_pair::cap::sewing; NURBS roots не добавлены.

73 теста прошли, 0 failed/ignored: 69 BRep + 4 NURBS, семь новых случаев.
39 qualified cases + 30 regression cases + 4 NURBS; промежуточные conform
прогоны не суммируются. Контрольный budget fixture сначала ошибочно ожидал
отказ при 17 evaluations для одной grid-resolvable вставки: теперь exhaustion
проверяется на двух внутренних targets, а достаточный 17-evaluation single-cut
остаётся отдельным положительным тестом. Геометрические gates не ослаблялись.

Source hashes совпали. Scoped format, explicit new-source sizes, 242 API roots,
200 catalog entries и 11 clean files проходят. Общая архитектура остаётся
красной на двух прежних UI-файлах, pins/exceptions не повышались.
Evidence: r73-automatic-cap-conform.json и r73-*-tests.log. Все процессы
завершены. Следующий этап — complete interior curve/face contacts, certified
near-gap healing, global embedding и volumetric material/closed-solid roles.
Full original parity scope остаётся открытым.


### r74 — interior curve/curve contacts и transactional rim imprints

contact_pair::cap::sewing::crossings перебирает пары actual входных рёбер разных
sealed Sheets, отсекая строго разделённые positive-rational hulls с outward
расширением на tolerance. Source хранит frozen фактическую входную Curve,
включая ранее допущенные разрезы. Exact root existence относится только к
этому actual определению, а не к идеальному normal-offset manifold.

Existing original-curve certified provider доказывает supported affine/planar
roots и явно сохраняет unresolved/overlap. Для нерешённых пространственных
пар general bounded solver предлагает candidates. Origin::Proposal даже после
успешной геометрической квалификации не превращается в Exact. Отсутствие
numerical components при unresolved report не становится proved absence.

Обе границы разрезаются в общий literal vertex только после полного допуска
обеих candidate subdivisions. Если второй разрез отказал, первый temporary
Model не публикуется. Дети проверяются напрямую относительно raw Cap world/UV,
actual support lifts и полного UV nesting. Frozen-input exact root и допущенный
с tolerance authored vertex — разные утверждения. Они не означают слияние
материальных секторов, face imprint reconstruction или global embedding.

Replay заново строит исходные caps/sheets/все разрезы и каждый retained exact
или numerical pair report ровно один раз, включая roots с construction refusal.
Exact reports сравниваются по root boxes/positions/coverage, numerical reports
остаются proposals. Event clocks и literal common vertex сверяются с retained
partition records. Попарное обнаружение и импринты имеют явные shared
pair/cell/box/candidate/split limits; failed provider calls расходуют quota.

API принадлежит existing BRep contact_pair::cap::sewing, новых NURBS roots нет.
Следующий этап — actual face cuts/material-sector reconstruction по этим
контактам, general endpoint/near-gap healing и global embedding до closed body.
Остальной полный первоначальный parity scope сохраняется открытым.

79 тестов прошли, 0 failed/ignored: 75 BRep + 4 NURBS, шесть новых случаев.
45 qualified + 30 regression + 4 NURBS; предварительные crossing-only runs не
суммируются. Проверены certified interior actual-edge roots, spatial numerical
proposal с независимым geometric admission, separated hulls, resource refusals,
rollback при отказе второго subdivision после успешной подготовки первого и
unresolved spatial pair без fallback, не превращённая в proved absence.

Source hashes совпали. Scoped format, explicit new-source sizes, 242 API roots,
200 catalog entries и 11 clean files проходят. Global architecture остаётся
красной на двух прежних UI-файлах; pins/exceptions не повышались.
Evidence: r74-interior-rim-imprints.json и r74-*-tests.log. Все процессы
завершены. Actual face cuts, material-sector reconstruction, полное face/face
coverage, endpoint/near-gap healing, global embedding и closed-solid
qualification ещё открыты. Full original parity scope не сокращён.


### r75 — actual cap face sectors, local UV selection и joint sewing

contact_pair::cap::regions сохраняет sealed исходный Sheet и строит actual
BRep sectors по существующим outer-loop vertices. split_uv сохраняет UV poles,
weights и physical clocks, предлагает world lift на ФАКТИЧЕСКОМ carrier с
нулевым дополнительным offset и доказывает его целиком. split_world сохраняет
авторскую 3D-кривую, а projected UV используется только после fresh full
agreement и trim audits. Endpoints должны быть уже owned; implicit nearest
welding здесь нет. Multi-segment C0 paths и последовательные descendant cuts
сохраняются без сглаживания. Original source/actual carrier остаются неизменны.

Native face_cut проверяет обе Jordan sectors и распределяет existing holes
только после полной проверки loops. Пересечение отверстия, неверный endpoint,
off-surface interior, недостаток proof/work и malformed поздний input дают
отказ без partial Regions. Final all-coedge lifts и trim regions проверяются
повторно. Existing bounded face_cut helper стал pub(crate); алгоритмические
predicates не ослаблялись. UV fitting allowances отдельны для каждого из
1..64 segments; максимум 64 сохранённых face cuts. Остальные stage allowances
явно bounded. World projection использует native limit до 100000 cells.

Regions::select классифицирует все текущие faces по strict UV witnesses и
даёт retained/discard indices, но НИ ОДНУ грань не удаляет. Witness на boundary,
снаружи, в двух областях или unresolved даёт отказ. Selection заимствует
immutable Regions; full replay повторяет исходные cap/edge cuts, все face cuts,
fit/projection, native Split provenance и winding. Это локальная UV material
selection, не объёмные outward/cavity roles и не solid export authority.

regions::sewing соединяет полученные sectors через общий private sewing kernel.
Внутренние cut edges сохраняют два uses; третий mate до material-role choice
отклоняется. Exposed edges проходят полное correspondence и final actual lifts;
связность, FaceUse orientation и vertex fans проверяются. Original Regions и
их history остаются retained для полного replay. Bodies не создаются.
Владелец API — existing BRep contact_pair::cap; новых NURBS roots нет.

97 тестов прошли, 0 failed/ignored: 93 BRep + 4 NURBS, десять новых случаев.
63 qualified + 30 regression + 4 NURBS; предварительные focused runs не
суммируются. Дополнительно проверены native face_cut и retention_network.
Rational cylindrical actual offset carrier разрезан curved world lift на две
подтверждённые области; выполнен полный source/fit/sector/winding replay.
Предикаты и tolerances не ослаблялись. Initial fixture ошибочно ожидал отказ
при одном trim cell для exact linear case: теперь отрицательный тест ограничивает
реальные pair visits, а достаточный one-cell exact proof проверяется отдельно.

Source hashes совпали. Scoped format, explicit changed/new-source sizes,
242 API roots, 200 catalog entries и 11 clean files проходят. Общая архитектура
остаётся красной на двух прежних UI-файлах, pins/exceptions не повышались.
Evidence: r75-cap-face-sectors.json и r75-*-tests.log. Все процессы завершены.
Открыты automatic actual face/face cut generation, объёмные material/outward/
cavity roles, endpoint/near-gap healing, global world embedding и closed-solid
qualification. Full original parity scope остаётся открытым.


### r76 — owned actual cap/cutter automatic face cuts

Regions::intersect принимает source/cutter Regions, face IDs и Fiber/Ruled
method; endpoints, actual trace, pcurves, search box и drive axis не задаются
пользователем. Native constructors сканируют все original OUTER edges по
actual trimmed cutter, требуют два retained owned roots, связывают vertices,
строят local trace и обе source sectors. Это не полный SS census. Замкнутые
скрытые ветви и более двух roots требуют отдельного branch ownership.

Fiber evidence дополнительно требует конечный root error <= половины world
tolerance и положительные root-fiber/trace-piece counts. Ruled support/lift
proof не выдаётся за fiber uniqueness. Final all-coedge lifts и all-face trims
повторяются; actual source carrier должен остаться буквально тем же. Cutter
hole проверяется по всему trace, не только endpoints. Пересечение относится
к фактическим surface definitions, не ideal normal-offset manifold.

Cut record владеет исходным cutter Regions вместе с прежними C0/automatic cuts.
Replay заново строит все caps, предшествующие операции и текущую transaction;
сверяет original face IDs, endpoints, coverage identities/root boxes, counters,
Split provenance и exact final Model. Ownership tree ограничено 256 nodes/
depth 32, source history — 64 cuts. UV selection и sector sewing остаются
доступны, все source sectors сохранены. Ни Body, ни объёмные роли не выдаются.

111 тестов прошли, 0 failed/ignored: 107 BRep + 4 NURBS, восемь новых случаев.
8 automatic + 69 qualified + 30 regression + 4 NURBS. Дополнительный one-case
hole-root assertion recheck не суммируется. Рациональный цилиндр разрезан обоими
методами без manual trace с полным replay; тяжёлый 192.77-секундный результат
переиспользован после проверки неизменности implementation hashes.
Два auto-generated источника сшиты в 4 faces по двум shared-root pieces.
У cutter с отверстием отдельно доказаны два retained boundary roots, затем
проверен отказ trace через hole. Checked prior C0 cutter history, source hole,
invalid face IDs и shared proof/work refusal.

Initial fixture задала разные signed-offset magnitudes и закономерно была
отклонена contact-chain до нового оператора. Исправлено положение исходной
опоры при равных offset magnitudes; predicates не ослаблялись. Source hashes
совпали. Scoped format, explicit source sizes, 242 API roots, 200 catalog
entries и 11 clean files проходят. Общая архитектура красная на двух прежних
UI-файлах, pins/exceptions не повышались. Evidence:
r76-automatic-cap-face-cuts.json и r76-*-tests.log. Все процессы завершены.
Открыты complete multi/closed-branch coverage, объёмные material/outward/cavity
roles, near-gap healing, global embedding и closed-solid qualification.
Full original parity scope не сокращён.

### r77 — boundary-connected multi-branch cuts on actual cap carriers

`Regions::intersect(..., Method::Branches, ...)` uses the existing native
multi-root planner on the actual source and cutter definitions. It queries the
original outer boundary once, owns every retained root, tries candidate root
pairings with complete original-fiber proofs, and preserves every resulting
sector. Record faces/identities are variable-length; both accepted branches and
rejected proposals are retained and compared during full source/cutter replay.
Every current face still needs the same final lift and trim audits. No caller
root, search box, trace or positive certificate is accepted.

A new two-branch actual-cap fixture exposed two native construction problems.
The piecewise fiber-cover fallback previously tried only the narrowest search
box, excluding the interior excursion of a curved branch. It now tries the same
eight bounded proposal widths, sharing the original remaining work allowance.
The endpoint binder updated the world/source endpoints but left the cutter UV
endpoint stale. A bounded eight-step numeric UV proposal now updates that pole
inside the original qualified box; every surface evaluation is charged against
the same allowance. This proposal grants no geometric authority: the unchanged
whole original-root trace proof, both support agreements, monotone clock and
box ownership must all pass again. No tolerance, predicate or work cap changed.

Branches expose per-branch endpoints/edges/root-fiber counts and conservative
charged work. The existing native planner does not expose aggregate root-error
or trace-piece totals, so those fields remain None/zero for this method rather
than inventing a certificate. Failed pair proposals are not absence proofs.
The test cutter is a genuinely nonplanar authored quadratic cap, independently
admitted against its retained original offset and every unchanged rim. Four
owned roots produce two curved cuts and three selectable sectors. Selection
replays the entire retained source/cutter construction.

This is boundary-connected branch matching, not complete surface/surface
coverage. Hidden closed branches, pole/periodic propagation, general contacts,
material/outward/cavity roles, certified near-gap healing and global embedding
remain open before closed-solid qualification. General noncircular blends,
high-valence corners, extraordinary quad reconstruction and the other original
parity requirements remain in scope. The frozen clean comparison is preserved.

Qualification: 116 tests passed, 0 failed/ignored (112 BRep + 4 NURBS),
three new actual-cap cases. All four runs are terminal success. Full Rust
source/test snapshot of 2168 files stayed unchanged throughout the final runs.
Scoped format, explicit changed/new-source size checks, 242 API roots, 200
catalog entries and 11 frozen clean files pass. General architecture remains
red on the same two unrelated UI files; pins/exceptions were not raised.
Evidence: r77-multibranch-cap-face-cuts.json, r77-source-sha256.json and
r77-*-tests.log. Initial narrow-cover and stale-endpoint refusals are retained.

### r78 — closed interior face sectors on actual carriers

`face_cut::partition_closed` accepts a literal closed CCW chain of authored
world/UV segments. This adds the topology stage missing from the open-path
constructor: the original face keeps its outer loop and gains a reversed cut
hole; a new face owns the positive cut loop. Both sectors survive in the same
oriented shell. New vertices are owned by the contour; every new edge has two
opposite coedge uses. Existing hole loops are assigned to exactly one sector by
complete region audits, without duplication or abandoned loops. The original
carrier and all original boundary definitions remain unchanged. Split lineage
and complete persistent naming are retained.

Every world/UV segment must be clamped, noncollapsed and join literally at both
world and UV endpoints, including the closing seam. The original region, new
interior region and outer-minus-cut region need full positive/simple trim
proofs. Touching, crossing, clockwise, outside and unresolved contours refuse.
Actual forward and reversed pcurves both need original whole-curve lift proofs,
with shared bounded trim/lift/exact work. No tolerance or predicate is loosened.
Two to 256 segments, nonperiodic charts and at most fourteen existing holes are
admitted; single closed-edge seam normalization and periodic charts are separate.

`Regions::split_closed_uv` retains the original UV chain and fits world curves
on the actual carrier at zero offset, with literal shared generated endpoints.
Fits remain proposals until the native closed transaction and final all-face
lift/trim gates pass. The closed operation is explicitly retained in provenance;
full replay rebuilds the sealed cap, each preceding cut, all fitted curves and
the exact final model. Selection and subsequent nested closed cuts preserve
both sides; no material removal or Body is inferred.

This is a necessary topology constructor, not an automatic closed-intersection
finder. Automatic mixed-coordinate fiber transitions, owned closed-root trace
construction and complete hidden-branch/contact coverage remain the next open
steps. Existing exact algebraic level-set pullback and mixed-fiber coverage are
available lower-level evidence; neither is promoted to a general closed BRep
intersection certificate without the missing transition/trace proofs. All
original general blend/quad/constraint/history/exchange/embedding requirements
remain in the full parity scope.

Qualification: 123 tests passed, 0 failed/ignored (119 BRep + 4 NURBS),
seven new cases. Final ten affected-consumer rechecks are not added. All final
runs are terminal success, including the rational cylindrical closed-cut replay.
An initial outside-fixture domain error was fixed in test construction; engine
predicates remain unchanged. Source snapshot covers 2171 Rust files. Broad
matrix preceded only a final rustfmt reexport ordering; new constructor and
affected consumer suites were freshly rebuilt from the formatted tree. Scoped
format, explicit source sizes, 242 API roots, 200 catalog entries and eleven
clean files pass. Architecture remains red on the same two UI files. Evidence:
r78-closed-cap-face-sectors.json, r78-source-sha256.json and r78-*-tests.log.

### r79 — automatic mixed-coordinate root connections

`ss_intersection::discover_fiber_transition` takes two sealed original root
charts and resource limits, not a caller-authored bridge box. It intersects
actual chart domains, searches bounded free-coordinate intervals and proposes
unknown-coordinate contraction using fresh original root images. The unchanged
full transition verifier independently replays both original charts, proves the
new bridge and both endpoint images, then owns its local orientation. Failed
proposals, their original/contracted boxes and all rectangle work are retained;
replay repeats the entire search. A failed search is not a separation proof.

`connect_mixed_fiber_cover_automatic` uses these discovered bridges to propose
a component forest. Half the original total allowance is reserved for the
existing final complete coverage and ALL cross-component root-box separation
proof. Failed searches cannot separate components. The final owner retains
all searches and original sources and independently reconstructs the same
transitions, point connections, components, separations and counters.

Closed discovery cells can meet only at an original root point. The mixed
component owner now also accepts exactly one fixed parameter coordinate in a
bridge spec, provided a fresh original singleton fiber is certified inside
BOTH charts. `FiberPointConnection` is stored separately from oriented open
transitions. Automatic point fallback reuses the existing source-point owner
and charges the same pair/total allowance. Neither a sampled point nor touching
boxes imply connectivity, and point connections supply no orientation.

A genuine closed circle is the exact intersection of a plane and a quadratic
graph. Thirty-two independently source-certified chart proposals cover the
entire original surface product; automatic mixed transitions prove one complete
parameter component, including free-axis changes. Its whole coverage uses
1415 cells / 7606 original rectangles; complete automatic connection uses
24261 rectangles and 31 oriented transitions. A separate two-root example
feeds the new owner from the existing fully automatic original atlas discovery,
with no authored seed, chart or bridge box in that pipeline.

This exposed a real depth failure in mixed coverage: choosing the first chart
boundary repeatedly peeled tiny slabs from a dense atlas. Boundary proposals
now balance the closed product tree; each child still needs the same fresh
original exclusion or full chart-containment proof. The original depth/cell/
rectangle limits and proof predicates are unchanged. Transition proposal work
is reserved in viable local batches within the SAME total allowance, rather
than starving every proof with tiny equal fractions of hypothetical future
attempts. Unknown-coordinate contraction is charged before the independent
full bridge proof; native errors consume their whole remaining reservation.

These are complete PARAMETER components and local oriented transitions.
A transition forest, even on this circle fixture, does not classify a physical
closed loop, periodic quotient, branch degree or world embedding. Automatic
cycle/end ownership, common endpoint-bound mixed-axis traces and generated
closed BRep trims remain open. The full original modeling parity scope,
including material roles, global embedding, blends, quads and constraints,
history and exchange remains active.

Next physical-loop proof to investigate (not implemented/certified in r79):
use complete original parameter components, a source-wide C1 guard, original
rank-three/contraction proofs and strict interior root enclosures. A compact
connected transverse one-manifold without boundary is a circle; independent
global injection of both original carriers could then establish a world
embedding. Every hypothesis needs an actual native gate, especially interior
coverage and knot continuity; a cycle in the transition graph is insufficient.
Only after physical ownership is proved should mixed-axis trace endpoints be
bound and passed through the r78 closed-sector constructor and actual trims.

Qualification: 89 selected tests passed, 0 failed/ignored (63 NURBS + 26
BRep), eight new cases. Seven private source/replay unit checks ran with the
explicit fiber_coverage filter; other unit tests were not part of this matrix.
All four final runs are terminal success. Snapshot of 2174 Rust source/test
files stayed unchanged throughout final runs. Scoped proof-module/test format,
eight explicit source-size paths, 242 API roots, 200 catalog entries and eleven
clean files pass. Full architecture remains red on the same two UI files; the
results.json current gate now reflects that, preserving its older snapshot.
Evidence: r79-automatic-mixed-root-connections.json, r79-source-sha256.json and
r79-final-*-tests.log. Initial proof refusals are retained separately.


### r80 — physical closed branches; reference alignment remains open

A sealed closed-branch owner now checks complete original components,
source-wide C1/clamped/nonperiodic structure, rank-three operators and support
immersions, strict interior root enclosures and native global injection of both
carriers. This establishes physical circles without inferring loops from chart
adjacency. Output is procedural trim/root-selector ownership, not fitted curves,
actual-face trims, seams or volumetric Bodies. Two separate C1 piecewise
quadratic ovals and the open/local-cover/C1/fold/work refusal cases pass.

Original root-image coverage and mixed-axis query replay replace excessive
subdivision with native source enclosure proofs. All failed and successful
queries/axes/work are retained. Point contraction, fallback reservations and
bounded candidate scheduling keep the original public limits. Initial failed
chart/depth/connection proposals are preserved as evidence.

76 selected tests passed (70 NURBS + 6 BRep), seven new cases; all recorded
final runs are terminal success. Own proof paths stayed unchanged. Unrelated
production and test files changed concurrently, so these results do NOT qualify
the entire current worktree. Both manifests and explicit drift are retained;
parallel work is preserved and not attributed to r80. Eleven frozen clean files,
scoped format, seven explicit production sizes, 242 API roots and 200 catalog
entries pass. The recorded architecture audit remains red on one UI file;
no r80 pin/exception changes. Evidence: r80-physical-closed-branches.json.

User reminder: parity comes from Plasticity/plasticity-beta native libraries,
including Parasolid/XNURBS/Bodyshop/converters. This stage did not re-observe a
specific Plasticity behavior. Restore a concrete reference -> implementation
-> equivalent behavior matrix before further generic roadmap expansion. The
entire original parity scope remains active; fitted mixed-axis closed traces,
automatic actual BRep trims, broad modeling families and integrated delivery
remain open.


### Exact profile material relation — 2026-11-02, qualification pending

Implemented complete shared original-curve graph and certified nonzero-winding
source/result sector comparison, including holes and coincident winding
multiplicity. `NoAddition` / `NoAddedProfile` require no exact added-material
sector; the latter retains actual source/result Full Round models. Five focused
tests passed in the preceding source snapshot. Receipt:
`docs/qualification/exact-profile-material-relation-2026-11-02/results.json`.
Current-worktree regression remains pending. Rational curved-support Full Round
still refuses exact relation on incomplete algebraic projection; diagnostic
example `full_round_material_relation` identifies failing original pairs.
No tolerance-band substitute, predicate weakening or increased budget is used.
This is profile material inclusion, not general 3D material/Boolean parity.
Full original scope and active goal are unchanged.


Latest continuation: seven exact-profile/material tests and three dedicated
polygon-contact-network tests passed (10 total, disjoint targets). The extracted
network module needed an explicit child-module path and parent-scope function
visibility; both are repaired without changing contact predicates. Exact
homogeneous conic residual has Bernstein signs `[0,+,+,+,+]` on the actual
retained rational result segment, proving its whole open interval is on the
exterior conic side. This forbids an approximate-common-component shortcut.
Receipt: `docs/qualification/exact-profile-material-relation-2026-11-02/`.
General original trim ownership, complete rational census and full scope remain
open; the goal is active.


### Rational projection exclusion — 2026-11-02

Implemented exact real-domain exclusion of rootless eliminant factors in both
producer and independent verifier. Factors remain in the full squarefree,
disjoint eliminant-product census; exact Sturm count includes projection-range
endpoints. No map is needed only when that count is zero. Limits unchanged.
Eight focused tests passed, including the previously unproved actual rational
support/retained-segment pair: exactly one endpoint, no overlap, and rejection
of a certificate with its real event erased. This overlaps the seven prior
material tests. Full curved Full Round rerun and broader root regressions are
still pending. The whole-open-interval exterior conic diagnostic remains valid;
this intersection fix does not turn approximate retained geometry into exact
source coincidence or establish full material parity.


### Geometry-scaled transverse probes — 2026-11-02

The rational Full Round progressed through complete intersections but next
refused exact side ownership within the existing radius budget. Implemented
initial-radius proposal from exact normal-line intersection distances. Exact
root isolation/refinement and positive-weight separation supply lower bounds;
unresolved proposals fall back to the prior radius. The unchanged complete
transverse census, center ownership and all original wire winding jumps remain
the authorization. Radius-halving and arithmetic allowances are unchanged.
Nine focused tests passed, including a removed band of width 2^-45, the rational
projection regression, six prior material tests and straight Full Round.
Receipt: `exact-profile-material-relation-2026-11-02/transverse-radius-tests.log`.
Full rational Full Round is being rerun; general parity remains open.


### Exact rational Full Round relation completed — negative inclusion result

Latest radius stage passed 9 focused and 17 broad tests (26 disjoint targets;
older reruns overlap). The default rational Full Round now completes the exact
material graph: 10 edges, 3 added sectors, 6 removed sectors. An exact witness
has source winding 0/result winding 1, so NoAddedProfile correctly refuses.
This resolves census incompleteness for the fixture but disproves strict
inclusion of its actual stored rounded result. Next work must address retained
source trim ownership/one-sided material construction without replacing actual
geometry by ideal curves or treating tolerance cover as exact inclusion.
Receipt: `exact-profile-material-relation-2026-11-02/full-round-after-radius.log`.
General 3D material validity, reference comparison and full parity remain open.


### Exact retained-source curve restrictions — implementation stage

Added immutable original Curve + exact rational parameter restriction, reusing
exact homogeneous source-span extraction. Rational controls and evaluations are
not rounded back to binary64; reversed and nonbinary natural parameters retain
source clocks. CurvedFullRound retains two such support owners. Initial five
tests passed (two restriction, two curved construction, one volume target with
multiple fixtures). A follow-up requires exact continuity across original knot
cuts and asserts actual Full Round owner retention; its final run is pending.
Receipt: `docs/qualification/original-curve-restriction-2026-11-02/results.json`.
This is necessary source-preserving trim data, NOT a repaired stored Model:
known exact added sectors, exact contact joins and closed body reconstruction
remain open. Existing NoAddedProfile continues to inspect actual result curves.


Exact restriction final qualification passed six tests (2 curved Full Round,
1 volume target with multiple fixtures, 3 original restriction tests). Original
knot clocks, reversed/nonbinary parameters, exact continuity across internal
knots, refusal of discontinuous source, and actual Full Round retained-owner
replay passed. Source hashes and final log are saved in
`docs/qualification/original-curve-restriction-2026-11-02/`. The stored Model
still has the known negative strict-material relation; exact contact sewing
and source-preserving body construction remain next, not completed by this API.


### Exact source G0 wire qualification — 2026-11-02

Eight tests passed: 2 exact-wire tests, 3 restriction tests, 2 curved Full Round
construction tests and 1 multi-fixture volume test. New Full Round assertions
replay exact wires and compare all fixed pieces with their original restricted
homogeneous data. A nonbinary rational contact is shared without rounding the
fixed support; insufficient edit tolerance and fixed/fixed gaps refuse. Full
span displacement is bounded by unchanged-positive-weight convexity.
Receipt and source hashes: `docs/qualification/exact-source-wire-2026-11-02/`.
Clean baseline remains 11 files unchanged; API map check passed.

Next: G1/regularity admission on the new exact rational pieces, then downstream
source-preserving body construction and material/embedding proofs. The existing
stored Model is still independent rounded geometry with its known negative
strict material inclusion. NoAddedProfile has not been authorized for it.
Full original native-library parity scope remains active and open.


### Exact rational wire regularity and G1 qualification — 2026-11-02

`Wire::qualify_smooth_edits` now verifies every rational span has no common
zero of its two derivative numerators on the CLOSED parameter interval, using
exact gcd/Sturm census, including endpoint and even-multiplicity stationary
points. Every join touching an editable piece and every internal editable-span
join requires positive tangent dot product and an exact squared-sine bound.
Unchanged original corners explicitly retain C0. Immutable SmoothEdits owns its
wire and replays all checks. CurvedFullRound retains this qualification as
`exact_smoothness()`, with declared sin^2 <= 1/10^10 (not zero-angle G1).

Twelve focused tests passed: 4 smoothness tests (complete rational circle,
reversed collinear tangent, hidden/endpoint stationary points, original C0
corners), 2 exact-wire tests, 3 restrictions, 2 Full Round construction tests
and 1 multi-fixture volume test. Actual Full Round assertions compare the
qualified wire with exact_profile and check complete span/junction coverage.
Receipt: `docs/qualification/exact-source-wire-g1-2026-11-02/`.

Next: construct the 3D shell from exact rational data, then independently prove
embedding and source/result material relation. The old rounded Model and its
known added-material sectors remain unchanged; this profile qualification does
not authorize NoAddedProfile or full native-library parity.


### Exact rational 3D source shell — 2026-11-02

`original_restriction::wire::prism::extrude` now constructs exact rational
vertices, Bezier edges, plane caps, ruled extrusion surfaces and pcurves on
existing generic `brep_topology::Model`. Profile coefficients and nonbinary
heights are never rounded to binary64. Source loop/piece/span provenance maps
to every wall. Closed incidence, vertex fans, regular profile spans and exact
edge endpoints are checked. Full oriented edge/surface-pcurve agreement uses
projective polynomial identities on the whole interval; positive-weight control
hulls prove UV chart containment. Replay rebuilds and compares all source data,
topology and geometry. ShellGeometry deliberately has no Body role.

CurvedFullRound retains this actual 3D rational geometry as `exact_shell()`;
its source profiles match exact_profile and qualified contact data. Fifteen
focused tests passed, including nonbinary contact/height, hole shell Euler
characteristic, refusal of invalid height, and geometry-tampering cases that
mere incidence would accept. Legacy Full Round volume tests passed as regression
coverage, not admission of the new shell's volume. Clean 11 files preserved;
API-map check passed. Receipt: `docs/qualification/exact-source-shell-2026-11-02/`.

Next: exact trim Jordan/embedding/nesting and source/result material relation,
then solid admission and downstream integration. The legacy rounded Model and
its known added-material sectors remain separate; new rational shell construction
does not authorize its NoAddedProfile or establish whole-library parity.


### Exact rational Jordan profile census — 2026-11-02

Added `wire::jordan::qualify` and `ShellGeometry::qualify_profile_jordan`.
Every exact span is regular and checked for self-intersection; every unordered
span pair is checked. Only actual adjacent endpoint contacts on the same wire
are admitted, including both contacts in a two-span lens and the closing
contact. Positive-weight hull separation, exact implicit endpoint identities
and the independently verified algebraic engine prove the complete pair census.
Non-endpoint crossings, interior tangencies, unauthored contacts, overlaps,
unknown algebraic representations and exhausted budgets issue no certificate.
Immutable JordanWires owns all originals and replays the whole census.

Eleven tests passed: 6 Jordan tests, 3 exact shell tests, 2 Full Round tests.
Coverage includes collinear subdivision, disjoint hole boundaries, bowtie,
loop contacts/overlap, interior quadratic tangency, a regular cubic with an
internal self-crossing, rational circle closure, complete pair counts and low
budget refusal. Actual polynomial/rational Full Round assertions replay the
new profile proof. Receipt: `docs/qualification/exact-source-jordan-2026-11-02/`.

Next: certify outer/hole orientation and nesting by exact rational ray winding.
A point on each Jordan loop is valid for containment queries against every
OTHER disjoint loop. Own-loop orientation needs an exact regular transverse
probe plus complete winding, not a sampled area sign. Require CCW outer,
CW holes contained in the outer, and pairwise nonnested holes for the current
single-shell cap layout. Existing generic ray_winding primitives accept exact
homogeneous nets and can be reused. Only then can the Cartesian-product
extrusion proof establish embedded bounded material and assign a Body role.
Source/result material inclusion, old rounded-Model defects, pipeline delivery
and full original native-library parity remain open.


### Exact rational extrusion solid admission — 2026-11-02

Added exact transverse orientation and ray-winding containment in
`wire::nesting`: CCW outer, CW contained holes, no nested holes, with shared
ray-cell budget and unchanged transverse allowances. Own-loop orientation is
proved using a complete transverse census and exact left/right winding;
other-loop containment queries use an exact point on the disjoint Jordan loop.
No sampled polygon area or rounded point supplies admission.

`ShellGeometry::qualify_solid` assigns one Body role only after these proofs.
For this Cartesian-product family, simple disjoint correctly nested profiles
and positive height imply embedded plane/ruled boundary patches, outward
orientation and a bounded positive-volume region. Internal profile holes are
through-holes, not independent cavity shells. Immutable Solid retains all
sources and replays source geometry, Jordan, nesting and role construction.
CurvedFullRound exposes the new geometry through `qualify_exact_solid`; the
legacy rounded Model is not substituted or qualified by that API.

Sixteen tests passed: 5 solid/nesting, 6 Jordan, 3 shell, 2 actual Full Round
construction tests. Cases include two through-holes, nonbinary heights, a hole
of width 2^-45, wrong orientations, outside/nested holes, budget exhaustion,
bowtie refusal and replay of polynomial/rational Full Round solid promotion.
Receipt: `docs/qualification/exact-source-solid-2026-11-02/`.

Next: audit raw BRep source-clock binding (prism::directed_edge normalizes knot
clocks; retained exact owners must be traced to actual raw definitions), then
prove source/result material relation and target Full Round radius fidelity on
new rational data. Current source restrictions exactly preserve the Curve they
own, but recognized profiles are not a substitute for raw-clock ownership.
Generic extrusion admission is not general NURBS parity. Legacy added-material
sectors, converter/pipeline delivery and full original scope remain open.


### Raw indexed source binding and exact clocks — 2026-11-02

Full Round now binds each recognized hint to the actual top-cap coedge and edge
ID, accounting for face-use reversal. Raw coefficients/knots are retained;
reversal uses exact t -> -t reflection. Contact parameters transfer to raw
natural clocks in rational arithmetic, without a floating affine intermediate.
The normalized curve is only a numeric proposal, never source authority.
Unmodified outer and hole pieces now use raw originals as well. Selected middle
and support face ownership uses actual native edge IDs. RawProfileCurve replay
checks indexed coedge, traversal, trim and raw definition against the source.

Eleven tests passed: 10 across raw binding, source clocks, Full Round and exact
solid tests, plus the volume regression. Cases include different nonbinary
clocks on every edge, a reversed edge, a reversed top carrier/coedge traversal,
a contact value not representable as binary64, and source-definition tampering.
The six volume fixtures now also admit/replay the new exact Solid (not only
legacy rounded geometry). Receipt: `docs/qualification/full-round-raw-binding-2026-11-02/`.

Next: source/result material relation and exact original SIDE SURFACE binding.
Raw rim ownership does not alone prove whole-support surface equivalence;
new-geometry radius admission, downstream integration and full original scope
remain open. The old rounded Model remains independently negative for strict
material inclusion; no tolerance or ideal-curve substitution hides that result.


### Whole original side-support image qualification — 2026-11-02

Added `CurvedFullRound::qualify_source_supports` and standalone source qualifier.
Exact tensor factorization proves XY independence across the whole height chart;
positive constant cross-column weight ratio gives a monotone projective Z map.
Source UV boundaries must form an exactly closed rectangle, with complete
nonoverlapping side coverage and the full native height range. Positive-weight
rational straight pcurves are admitted by their entire segment image, not sampled
positions. Every native top rim is matched to its actual side/profile interval
using existing source-bound exact multi-span or bidirectional image certificates.
The full top-edge census covers every original side surface; no source carrier
is replaced by a normalized/trim-fitted surrogate.

Ten tests passed: 6 support cases, 2 raw binding, 2 Full Round construction.
Cases include exact degree elevation, projective height reparameterization with
correct inverse vertical pcurves, transposed UV axes, and rejection of a warped
height interior despite an unchanged top rim, UV endpoint gaps, and complete
profile mismatch. Model validation was retained: the initial projective test
needed inverse pcurve clocks rather than weakening the boundary checker.
Receipt: `docs/qualification/full-round-source-supports-2026-11-02/`.

This proves source SIDE SURFACE IMAGE equality for the extrusion family, not
oriented source-solid validity or general freeform support equivalence. Next:
source caps/complete boundary authority and source/result material relation;
new exact radius fidelity and downstream integration remain open. Full original
native-library parity scope is unchanged and active.


### Native source cap charts and honest seam modes — 2026-11-02

Added `original_restriction::plane` and `CurvedFullRound::qualify_source_caps`.
The original positive constant-weight bilinear chart is retained, including a
nonzero mixed coefficient; four strict Jacobian signs and convex boundary turns
prove global chart regularity/injectivity. Actual pcurve containment uses exact
polynomial sign/multiplicity census, admitting even boundary tangencies without
requiring the control hull to fit. Original UV loops pass complete Jordan,
orientation and hole nesting checks. Native physical cap normal signs are checked.

Each native world rim is compared with actual surface/pcurve composition over
common rational knot cells. Exact Bernstein products and positive denominators
bound the WHOLE parameterized displacement. Zero and nonzero bounds remain
distinct: SourceCaps::require_exact refuses a tolerated nonzero seam. No affine
surrogate, samples or idealized replacement supplies the cap geometry.

Thirteen focused tests passed (5 cap/plane, 2 raw binding, 6 side supports).
An additional 2-test raw-clock/reversed-carrier replay passed and overlaps the
main count. A box has exact cap identity; the default curved source has a
nonzero certified seam within model tolerance. Mixed bilinear data, actual
chart containment, folded-chart refusal and strict-mode refusal were tested.
Receipt: `docs/qualification/full-round-source-caps-2026-11-02/`.

Next: explicit source-bound cap pullback/canonicalization preserving original
3D rims and carriers, with a separate repair receipt for legacy UV auxiliaries;
verify lower-rim/side compatibility before any strict original-source solid
claim. A nonzero paired-parameter seam is not hidden as exact closure, and it
alone is not a universal image-set mismatch proof. Then compare source/result
material on exact rational geometry. Original input and old rounded result stay
available; full initial scope, new radius qualification and delivery remain open.


### Exact original-carrier cap pullbacks — 2026-11-02

Added `plane::pullback`: the unchanged original world curve is primary.
Exact whole-span half-plane sign checks prove containment in the actual convex
bilinear chart image. Affine carriers expose exact rational UV Bezier controls;
genuinely bilinear carriers retain a unique implicit inverse and return exact
rational isolating UV boxes. Root isolation, domain checks and explicit refinement
allowances fail closed. No rounded UV controls, replaced surface or sampled
containment supplies admission.

`SourceCaps::canonical_pullbacks` retains the original SourceCaps and its legacy
UV/seam receipt, binds each new inverse to the actual indexed boundary, and proves
Jordan/orientation/nesting on the unchanged world rims. The chart homeomorphism
transfers these region properties to the implicit UV boundaries. Original Model,
world curves and surfaces are unchanged; old nonzero seams remain nonzero.

Ten tests passed (3 inverse/containment, 5 native caps, 2 raw binding). Irrational inversion
checks bracket signs and requested width; zero-budget refusal, exact 5/6 UV
controls, even boundary tangency, outside excursion and actual curved Full Round
repair/replay are covered. Distinct raw clocks, reversed edge and transposed/
reversed carrier canonicalization/replay also passed. Clean baseline: 11 files
unchanged. API map check passed (242 roots, 200 catalog entries; not parity).
Receipt: `docs/qualification/full-round-cap-pullbacks-2026-11-02/`.

Next: verify lower-rim/side compatibility and construct explicit complete source
boundary authority, then source/result material relation on exact rational data.
This auxiliary UV canonicalization does not admit an original source Solid or
prove radius fidelity. General freeform charts, complete original library scope,
full regressions, pipeline delivery and external runtime comparison remain open.


### Complete original lower-rim image binding — 2026-11-02

Added `SourceSupports::qualify_lower_rims` with immutable source ownership and
replay. Every actual lower-cap coedge must have one unique original side owner;
its original world curve, with actual knot clock and traversal, is certified
against the native side profile on the authored bottom-isocurve window. Exact
parametric or bidirectional curve-image certificates authorize agreement, not
model tolerance. The full lower-cap census must cover every side bottom edge.
Original curves/carriers are unchanged; partial/lifted coedges are refused.

Eight tests passed: the six source-support cases plus two lower-rim regressions.
Positive cases include original nonbinary lower knot clocks, exact degree
elevation, projective vertical reparameterization and transposed/reversed side
axes. A 1e-10 lower-curve interior perturbation still passes Model validation and
top/whole-support qualification but correctly refuses exact lower-rim binding.
These eight overlap previous support qualification; do not add counts across
receipts. Clean baseline verified (11 files unchanged), API-map check passed.
Receipt: `docs/qualification/full-round-lower-rims-2026-11-02/`.

Next: exact vertical native edge/side-seam compatibility and complete source
boundary authority, including orientation/incidence. Lower-rim image binding is
not a source Solid certificate and does not prove coupled-parameter equality.
Source/result material relation, target radius on new rational geometry, global
spatial families, complete original scope and downstream delivery remain open.


### Exact native vertical seams and complete edge census — 2026-11-02

Added `original_restriction::vertical` and
`LowerRims::qualify_vertical_seams`. Original vertical edges retain full native
curves/weights/clocks; exact XY equals the side profile evaluated at the actual
constant UV coordinate. Each side's positive projective vertical map supplies
exact authored endpoint heights, including partial height intervals. Both side
uses must agree, traverse oppositely in the shell, and have exact native vertex
endpoints. Complete rational derivative-root census on every original Z span
proves strict monotonicity and regularity; even stationary roots and endpoint
stationarity refuse. Thus a certified edge maps one-to-one onto its segment.

Top, lower and vertical bindings must form a complete disjoint census of ALL
original edges. Legacy pcurves are retained; image agreement does not assert
coupled-parameter identity. Immutable owners replay all definitions and facts.

Twelve tests passed (9 source-support cases, 3 exact vertical tests). Positive
cases cover rational weights, nonbinary native clocks, both traversal directions,
degree-elevated/projective/transposed source surfaces. Tiny vertical interior
XY bow still passes Model validation and exact horizontal-rim proofs but refuses
vertical admission. Complete even-root, endpoint and backtracking refusals pass.
Clean frozen baseline: 11 files unchanged. Receipt:
`docs/qualification/full-round-vertical-seams-2026-11-02/`.

Next: complete oriented source-boundary authority: original side-profile
regularity/injectivity on actual trimmed windows, coherent carrier orientation
and complete cap/side assembly. Exact edge census alone is not source-solid
admission. Then prove source/result material relation and new exact radius.
Full original library scope, spatial/general freeform families, pipeline gates
and external runtime comparison remain open.


### Native carrier profile regularity, simplicity and outward normals — 2026-11-02

Added `Wire::qualify_source_regularity`: exact rational derivative census on every
source span and exact oriented G1 at internal knots, while authored corners
BETWEEN separate source pieces remain allowed. No sampling or nonzero angular
allowance authorizes internal carrier joins.

`SourceSupports::qualify_native_profiles` restricts the actual original SIDE
SURFACE profile on indexed top-rim windows, retaining native coefficients/knots.
Exact directed endpoints must match the original raw rim. Closed fixed wires,
whole Jordan pair census, orientation and hole nesting prove a simple native
profile region. UV-axis permutation, projective height direction, profile window
direction and shell face-use together must yield the outward physical normal.
This does not substitute a top rim for the source surface profile.

Thirteen tests passed (9 supports, 2 exact source regularity, 2 raw binding).
Coverage includes internal collinear knots, authored corners, internal kink and
even stationary-root refusal, actual degree-elevated carrier coefficients,
projective height, transposed side axes, different raw clocks and reversed cap.
Clean baseline: 11 files unchanged; API-map check passed. Receipt:
`docs/qualification/full-round-native-profiles-2026-11-02/`.

Next: exact LOWER directed endpoint/native vertex compatibility and regularity
of original WORLD cap rims, then combine cap pullbacks, native profiles and
vertical seams into complete source-boundary authority. Image-set agreement
alone must not be used as a directed endpoint identity. No source Solid admission,
material inclusion, new radius fidelity or full original parity is claimed here.
General spatial families, full regression/pipeline and runtime comparison remain
open under the unchanged original scope.


### Complete canonical source boundary and material reference — 2026-11-02

Added `CurvedFullRound::qualify_source_boundary`: original native profiles,
vertical seams and explicit cap pullbacks share the SAME unchanged Model.
Lower-rim image certificates now also require exact directed native/rim endpoints.
All horizontal world endpoints must equal their indexed native vertices. Original
world cap wires pass source regularity/internal-knot G1. Complete disjoint cap/side
face census and two opposite DISTINCT face uses for every original edge bind the
geometry to actual source incidence. No sampled/tolerance-only fact supplies
these exact endpoint or regularity admissions.

`SourceBoundary::canonical_material` constructs a SEPARATE exact rational body
from the certified native profile region times the original positive height.
Original side factorization and full rim/seam coverage identify this material;
cap domains use the explicit original-carrier inverse expressions. This is an
explicit auxiliary-UV canonicalization reference, NOT a claim that old nonzero
pcurves were identical, not a mutation of original Model, and not admission of
the old rounded Full Round result. Immutable material replay rebuilds the body
and checks exact source profile geometry/ownership. Through-holes remain holes
of the profile region in one body, not independent cavity shells.

Seventeen focused tests passed (4 boundary/material, 2 raw, 9 support, 2 source
regularity). Positive cases include polynomial/rational source data, actual raw
clocks, reversed cap, degree elevation, projective/transposed side charts and a
through-hole. Tolerance-valid vertex drift and stationary original world curve
refuse complete boundary admission. Fixture corrections were explicit: retain
the actual exact UV receipt where the fixture has binary-exact coefficients;
scale the stationary fixture by exact 1/256 to stay within the original allowed
model tolerance. No validator or predicate was weakened. Logs retain initial
fixture failures and final successful run. Clean baseline: 11 files unchanged;
API-map check passed. Receipt:
`docs/qualification/full-round-source-boundary-2026-11-02/`.

NEXT: source/result material partition on the NEW exact rational Wire data,
using this explicit canonical source reference and preserving the old rounded
negative material counterexample. Then exact target-radius/tangent fidelity on
new bridge geometry. General spatial/freeform families, complete original native
library scope, pipeline/WASM/full regressions and external comparison remain
open. A canonical source reference does not itself prove no material addition
or full parity.


### Exact rational Wire material census; NEW Full Round counterexamples — 2026-11-02

Added `wire::material::inspect` on already certified Jordan/nesting owners.
It consumes actual rational homogeneous nets, not rounded f64 Curve surrogates.
Each selected span is cut at a complete projected critical set from every
opposite-region span: isolated-system resultants plus common-image branch
endpoints/leading/discriminant critical values. Checked common-component
factorization retains residual isolated intersections. Pairwise coprime
squarefree factors retain the whole root union, with exact algebraic ordering;
no large global polynomial product is needed for root isolation.

Each open parameter atom has a rational representative. Complete point-preimage
census declares ALL coincident center aliases; unchanged transverse proof excludes
all other branches before exact ray winding labels both sides. For two bounded
simple oriented Jordan regions, every nonempty open difference component has an
open boundary atom; finite critical vertices cannot hide an open material cell.
Immutable replay reconstructs projections, root order, sample census and labels.
NoAddition refuses any source-winding-zero/result-winding-nonzero witness.

`CurvedFullRound::inspect_exact_source_material` owns the explicit canonical
original-source material body, NEW exact result Solid and equal-height proof.
It does NOT compare or authorize the old rounded Model. Product-family planar
inclusion transfers to equal-height material; source UV canonicalization remains
explicit and original Model/legacy pcurves remain unchanged.

Six focused tests passed (5 exact material, 1 actual Full Round test covering
both weights plus replays). Cases cover exact 1/3 controls, identical/partial
common boundaries, crossing rectangles, two hidden quadratic crossings, even
tangency, filling a source hole with identical outer boundary, and a 2^-45 added
band. Pair-budget exhaustion refuses. Real Full Round test/replays took 60.67s.

IMPORTANT NEGATIVE RESULT: the NEW source-preserving rational Full Round is
also NOT strictly inside canonical source material. Weight 1 has 7 added and
10 removed SIDE-ATOM WITNESSES; weight 0.7 has 4 added and 11 removed witnesses.
These are not distinct connected-component counts. Exact witness coordinates,
parameters, winding [0,1], logs and source hashes are saved. NoAddition correctly
refuses both. This is distinct from the previous old-rounded 3-added-sector
counterexample, which remains valid and available.

Runtime sampling located global-product Sturm isolation; the owned initial run
was intentionally interrupted for this evidenced optimization after weight 1
comparison completed. The final run uses smaller coprime factors, exact ordering
and unchanged engine budgets; both comparisons and replays passed. Receipt:
`docs/qualification/exact-wire-material-2026-11-02/`. Clean 11-file baseline and
API-map checks passed. Focused receipts do not qualify the whole shared worktree.

NEXT: repair rational bridge tangent controls/geometry while preserving exact
retained source restrictions, then re-run this strict material census and target
radius proof. Current endpoint-only repair and tolerance-G1 are NOT no-addition
proofs. Do not mask the counterexample with a tolerance band or return a false
success certificate. Full original library scope, spatial/freeform families,
pipeline/WASM/full regression and external runtime parity remain open.


### Exact edited tangents, original crown contact and whole-span radius — 2026-11-02

Replaced endpoint-only adjustment of the NEW exact Wire by explicit quadratic
control repair. Each edited span's middle control is the exact intersection of
its desired endpoint tangent lines. Fixed original source tangents control the
external joins; a shared original proposal tangent controls internal arc joins.
Positive intersection distances prevent reversal. All original positive weights,
fixed source restrictions and source natural clocks remain unchanged.

An explicit LineContact owns the actual raw middle source segment. The crown
junction is projected onto that finite original segment in exact rational
arithmetic (strict interior), and both incident arc tangents use the original
line direction. No normalized surrogate or support extension supplies contact.
Every modified Cartesian control is included in the displacement bound, which
bounds the WHOLE rational curve by positive-weight convexity. Immutable replay
retains the repair mode and source-owned contacts. Parallel/invalid tangent
intersections, reversed directions or displacement beyond the ORIGINAL tolerance
refuse. `qualify_smooth_edits` now uses ZERO angular allowance for new Full Round.

Added `Wire::qualify_edited_radius` / `CurvedFullRound::exact_radius`: exact
Bernstein coefficients of (X-Cx W)^2+(Y-Cy W)^2-r^2 W^2 bound the whole radial
deviation by max|N|/(min(W)^2*r). Every edited span is checked to the unchanged
proposal center/radius within source.tolerance_mm/32. This is a rigorous radial
ERROR BOUND, not an assertion of exact analytic-circle identity when nonzero.
The certificate owns/replays the actual repaired Wire, not old f64 bridge curves.

Ten focused tests passed. Full Round strict material/replays now yield ZERO
added witnesses for both weight 1 (8 removed witnesses) and weight 0.7 (10 removed).
NoAddedExactMaterial admits/replays both. The same test reconstructs the previous
endpoint-only Wire from unchanged proposal sources and finds 7 added witnesses;
NoAddition still refuses it. The predicates/limits were not weakened. Tangent-only
repair had left a crown overshoot at y>8 in the polynomial case; exact original
line contact eliminated it. Intermediate/final logs and own source snapshots are
retained. Old rounded Model and all prior negative receipts remain separate.

The 10-test set includes 2 restriction, 2 tangent/control-bound, 2 construction,
1 volume (six symmetric/asymmetric polynomial/rational multispan fixtures),
1 material (two full cases plus replays and old-G0 negative), 2 raw binding tests.
A middle-control-only move has exact squared bound 1/64 and rejects a smaller
allowance; parallel tangent refusal passes. Volume fixtures now also run the new
radius admission at construction. Strict material census was run on the TWO
explicit symmetric cases, NOT all six volume variants. Clean baseline 11 files
unchanged; API-map check passed. Receipt:
`docs/qualification/full-round-exact-tangent-radius-2026-11-02/`.

NEXT: strict exact material/radius qualification on remaining asymmetric and
multispan fixtures, then connect admitted new rational geometry into downstream
Model/pipeline without silently returning the old rounded construction. General
spatial/freeform support families, variable-radius/high-valence blend networks,
full regression/WASM/external comparison and complete original parity scope
remain open. Two admitted cases do not complete the full Full Round family.


### Six-case strict family and admitted exact native entry — 2026-11-02

Extended strict NEW rational material qualification to all six existing volume
fixtures: symmetric/asymmetric polynomial/rational supports with arc span layouts
1+1, 2+2, 2+1 and 1+2. EVERY case has zero added-material witnesses; NoAddition
and all source/result material certificates replay. Whole edited-span radial
bounds and ZERO-allowance G1 are also checked on each actual output. The count
of removed side-atom witnesses varies by case and is not a component count.

Added public `feature_ops::full_round_exact` and `AdmittedFullRound`. The API
returns the ACTUAL rational `prism::Topology` owned by NoAddedExactMaterial,
with matched radius and tangency owners. Geometry equality binds these proofs
to the output profile. No f64 conversion or old rounded Model is returned as a
fallback. Original source and selected-face identities remain accessible, with
immutable full replay. Existing legacy `full_round` / `CurvedFullRound::model`
remain separate; downstream consumers have NOT been silently migrated.

Usage: `feature_ops::full_round_exact(&source, faces, Default::default())?`;
`result.model()` is exact rational topology, `result.reverify()` replays proofs.
This is a native library entry, NOT completed ModelGraph/WASM pipeline wiring.

Nine distinct focused tests passed: six family cases plus the latest three
public-entry cases. Earlier overlapping entry reruns are not added. Entry tests
prove returned-model pointer ownership, correct source retention, no fallback on
missing material budget/wrong original selection, and strict admission with a
preserved through-hole (one body, two cap holes, no cavity shell). Six-case run
27.34s; latest entry run 23.69s. Own code snapshots/hashes and logs:
`docs/qualification/full-round-admitted-family-2026-11-02/`.
Clean baseline: 11 files unchanged; API-map check passed (not a parity count).

NEXT: rational topology transport/ownership in ModelGraph/WASM/main pipeline,
source-bound topology lineage and exact native exchange/roundtrip. Preserve the
actual rational payload and proof linkage; rounding it into the old Model does
not transfer material admission. General spatial/non-extrusion support families,
variable-radius/high-valence/reblend networks, full regressions and external
runtime comparison remain open under the full original scope.


### Frozen-source native/WASM transport — 2026-11-02

Added an explicit frozen Recipe (original natural-clock retained ranges, original
numeric bridge proposals, exact target center/radius). `recipe::restore` reads
these as UNTRUSTED proposal inputs, binds selected faces to actual raw original
rims, refuses reversed/changed unselected source intervals, reconstructs exact
tangent/crown geometry, and freshly proves original source boundary, result Solid,
whole radius, zero-G1 and strict no-addition. It DOES NOT rerun floating Newton/
trigonometric contact generation. This is essential for identical geometry across
native and WASM math implementations. Source/range ownership is never inferred
from a caller success field.

Codec formats `admitted-full-round/1` and `exact-rational-nurbs-brep/1` retain the
ACTUAL output topology: rational vertex/homogeneous controls/knots are integer
string pairs, together with exact indexed incidence and source wall mapping.
Curve spans are normalized clamped Beziers; surface native parameter domains are
preserved (including low/high-corner -> per-axis knot mapping). Source Model,
selected faces and frozen recipe stay explicit. Import rebuilds and proves the
body, then compares complete output/evidence; serialized flags cannot authorize
geometry. Transport has an explicit <=4 MiB byte budget. JSON numeric tag changes
are permitted only as numeric semantics; rational coefficients remain strings.

Shared geometry-bridge router now exposes `cad_full_round_exact` and
`cad_full_round_exact_restore`. Unknown request success fields and incomplete
material budgets refuse. No f64/legacy Model fallback is returned. This is the
actual native/WASM operation router, not yet ModelGraph schema/result plumbing.

TWELVE Rust tests passed: 2 codec/recipe, 2 public bridge, 6 multispan family,
2 raw-clock/reversed-carrier. The filtered fixture replay overlaps one codec test
and is not added. Frozen reconstruction reproduces identical exact topology in
all six families (1+1, 2+2, 2+1, 1+2) and with nonbinary clocks/reversed source.

FRESH REAL WASM was built (wasm32-unknown-unknown, dev opt1/debug0, import-free)
and exercised through operation 0 / MGV1 binary ABI using the actual TS codec.
Six cases passed: native frozen artifact -> identical WASM rational geometry;
original-source WASM construction; WASM JSON frozen roundtrip; forged exact
coefficient refusal; caller-success-key refusal; changed original interval
refusal. SHA256:
`69eb251798727f99df9c2a5b88691a56b5db3ce4ac2122edadc565a06af2adf6`.
Own native artifact, source snapshots/hashes, runtime timings, logs and build
recipe are saved at `docs/qualification/full-round-frozen-recipe-transport-2026-11-02/`.
The 43 MiB custom module stays at the recorded /tmp build path; the packaged
application kernel/identity were NOT replaced. Clean 11-file baseline and API
map checks passed. This is focused qualification, not a shared-worktree full gate.

NEXT: ModelGraph exact-rational carrier/schema and downstream operation/result
handling, persistent topology IDs/source-owned lineage, exact payload consumers
for display/edit/exchange, then packaged source-bound kernel and real worker
integration. Do not coerce rational geometry into the legacy f64 Model and assume
proofs transfer. General spatial/freeform families, global blend networks, full
regressions/external comparison and original whole-library parity remain open.


### Indexed source-face lineage and portable exact topology IDs — 2026-11-02

Added `AdmittedFullRound::lineage`. Parent selection follows the ACTUAL indexed
constructor plan and certified original rim/side bindings, never a nearest/similar
geometry fingerprint search. Retained-side records own original face/rim/loop
TopoIds, actual coedge slot, source piece and exact oriented AND original stored
natural-clock span ranges; reversed raw clocks map back by exact negation.
Generated blend records keep all three selected source-face contributors and
bridge/span indices. Cap parents are the unique certified original height owners.

Every output vertex/edge/loop/face/shell/body receives an ID. SHA namespaces use
all current source identities and exact output homogeneous geometry; they only
disambiguate new names, never establish ancestry. IDs are checked unique and
newly generated IDs disjoint from current input IDs. An unchanged whole side
with one child and complete single-rim source coverage retains its original face
ID. Partial one-child sides/caps are Modified; multi-child source-face replacement
is Split. Blend face contributors stay explicit; the consumed middle face is the
replacement parent in the ChangeSet. Source change history is retained and the
combined ChangeSet validates. Shell/body identities are modified from the original
single owners. Vertex/edge/loop IDs are currently GENERATED, not claimed preserved
under later edits or complete source edge/vertex ancestry.

Transport adds optional derived `geometry.topologyIds` / `faceOrigins` to v1.
Current exports include them; supplied naming metadata must equal fresh native
reconstruction. Older v1 artifacts lacking BOTH fields still receive complete
geometry/material reconstruction and upgrade to newly derived metadata. Partial
or forged ancestry cannot authorize an input. Native/WASM IDs are identical for
the same frozen source/recipe. General occurrence/branch/edit naming invariance
has not been qualified.

FOURTEEN Rust tests passed (4 admitted/naming, 2 codec, 6 multispan family,
2 raw binding). Coverage includes an authored source-ID rename that would alias
a former output child without source-context rebinding, forged ID/parent refusal,
whole-face persistence, hole faces, all source span layouts, reversed/native
clocks and exact original stored-domain membership. EIGHT real MGV1 WASM ABI
cases passed, including native naming equality, legacy naming upgrade and forged
source-lineage refusal. Fresh import-free WASM SHA256:
`6335e44e7345131f2cccd316f3b5875cd9ff4f674144cf935e134c482413ffe5`.
A hash-named module copy is retained outside Git at the path in the receipt.
Own source snapshots, native artifact, logs and reports:
`docs/qualification/full-round-source-lineage-2026-11-02/`.
Clean baseline: 11 unchanged files; API map check passed. The packaged application
kernel is still NOT replaced; this is focused custom-WASM qualification.

NEXT: ModelGraph exact-rational carrier/schema/result handling and operation
execution. Relevant paths are `modelgraph-runtime/src/nurbs/compile*`,
`src/services/rushGraphNurbs.ts`, `rushGraphNurbsKernel/createOwnNurbsResolver.ts`,
and native `geometry-bridge/src/brep_semantic.rs` / `brep_session.rs` for the
SemanticProgram DAG. Their current solid consumers deserialize legacy f64 Model;
do NOT smuggle exact string pairs through that type. Exact result admission must
retain/reconstruct the frozen Rust-owned body, and session/worker budgets must
charge the complete artifact. Then extend edge/vertex/loop source ancestry and
edit/occurrence stability, source-bound packaged kernel/worker delivery, display/
edit/exchange consumers and full regressions. Whole original parity remains open.


### Native semantic DAG exact Full Round carrier — 2026-11-02

Added unary native semantic node `full-round-exact` (`input`, original indexed
`faces`, optional material limits/transport byte budget). It consumes the actual
owned upstream source and invokes the material-admitted native factory. The
result uses the EXISTING shared `rational-brep` / `representation-preserving`
carrier, geometryKind solid, d3. The actual result is rational topology plus its
complete frozen artifact/IDs, never a cast of the old f64 Model.

`brep_session` recognizes the new reference arity and rational carrier. Before
publishing a nonempty exact lease it requires matching model/artifact geometry
and freshly reconstructs the native recipe/material/G1/radius proofs. The owned
snapshot retains the whole artifact. Byte and aggregate character budgets charge
that COMPLETE value, not only the visible model. An allowance that fits source
and model but omits frozen source/recipe/evidence correctly refuses.

Legacy operations explicitly reject the exact-rational encoding with
BREP_UNSUPPORTED_SEMANTIC; they do not round it into Model. A legacy primitive
cannot claim rational output by changing its declared carrier. Wrong Full Round
carrier and missing material work refuse. Failed direct-session evaluation keeps
prior entries until close/abort, as existing transaction semantics require, but
cannot publish or commit; the public graph runner aborts all entries on failure.
Initial tests incorrectly expected immediate clearing on direct Session failure;
those expectations were corrected, not the existing transaction contract.

FIFTEEN Rust tests passed (4 native graph integration, 11 existing Session
regressions). Tests cover public `brep_graph`, exact frozen output, proper carrier,
full retained-geometry budget, zero material budget, cast refusal, unsupported
legacy transform, owner/release/commit/typed-empty behavior. SIX actual WASM ABI
cases passed on a fresh custom import-free build; public graph execution produces
rational strings/IDs and every negative path refuses with the expected category.
WASM SHA256:
`20c3db1d2ae0a1226a182b1bbef51afadc2d7510e4bbc41ed3c3fb512a427f21`.
Logs, own source snapshots, timings and hash-named retained module path:
`docs/qualification/full-round-native-graph-2026-11-02/`.
Clean baseline 11 files unchanged; API-map check passed. Packaged kernel unchanged.

SCOPE: this enables the NATIVE semantic DAG/session/router path. High-level
ModelGraph/SemanticProgram node unions, frontend schema, capability closure,
operation identity/lowering and result/display readers are NOT yet migrated.
Requests with full envelopes must retain strict validation; do not bypass them
just to route an unknown operation. Relevant next files include
`src/core/semanticProgram.ts`, `semanticProgram/semanticResultItems.ts`, host
validators/backends, `geometry-bridge/src/brep_envelope.rs`, plus the separate
NURBS `rushGraphNurbs` schema/resolver pipeline. The existing rational carrier
is already declared, but the new operation kind is not. Then rational downstream
edits/exchange, edge/vertex/loop lineage, packaged worker delivery/full regressions
and all original spatial/freeform parity obligations remain open.

### Native exact Full Round capability admission — 2026-11-02

`brep_envelope` now recognizes `full-round-exact` and includes its unary input
when deriving capabilities. The required closure includes the actual
`analytic-brep` → `rational-brep` transition. Five graph integration tests pass,
including semantic result selection and refusal when the operation, rational
representation or transition capability is removed; out-of-range dependency
references also refuse. Clean baseline remains 11 unchanged files.

Evidence: `../qualification/full-round-native-envelope-2026-11-02/`.
This fixture supplies no operation identity rows and therefore does not qualify
occurrence-production replay. TS union/schema/lowering/capability support and
the native production rule remain open, together with exact downstream
consumers, packaged WASM/worker integration and the original parity scope.

Native envelope qualification: 5 graph integration + 8 envelope regression tests passed. Receipt: `../qualification/full-round-native-envelope-2026-11-02/results.json`. TS schemas/lowering, native occurrence-production and packaged WASM remain open.


### Native Full Round operation production — 2026-11-02

Added authored `geometry:full_round_exact` → `full-round-exact` map production.
Every output consumes its matching ordered child frontier, all outputs of one
occurrence share face/limits/maxBytes parameters, and the feature result owns
its own scene identity. Face selection must contain three unsigned integers.
Native fixture IDs are re-derived from actual structural paths.

16 tests passed (3 new + 13 existing production regressions). New coverage
includes wrong input/identity/kind and malformed faces, plus an actual graph
construction with complete operation/occurrence identity rows and capability
closure, returning rational geometry and the feature-owned result identity.
Receipt: `../qualification/full-round-native-production-2026-11-02/results.json`.
The 11 clean files remain unchanged.

The new fixture is authored natively, not lowered by the TS frontend. Host node
union/schema/producer names/production replay and language lowering remain
open, followed by exact consumers, new WASM/package/worker qualification and
the complete original parity scope. Source-text attestation is not claimed by
this fixture.


### Host Full Round contract and real source-to-native execution — 2026-11-02

Added `SemanticFullRoundExactNode`, unary dependency/remapping, producer name
and capability closure; strict brep-1 validation requires an original analytic
solid and rational solid output, distinct indexed faces and bounded optional
material budgets. Normal and interrupted host map replay recognize the new
operation with canonical shared parameters and new result identity. Added
`full_round_exact(faces=[5,4,3]) cube([4,8,6]);` source lowering, including user
modules and binary codec roundtrip.

The public trusted lowerer generates the saved full envelope and native request.
Rust admits the actual source text/hash, operation/occurrence IDs, capability
closure and execution plan, constructs exact geometry, and refuses altered
source text or omitted capability. 279 host tests, both TS typechecks and
22 Rust tests (16 production + 6 graph/source) pass. Receipt:
`../qualification/full-round-host-contract-2026-11-02/results.json`.
11 clean files unchanged. Common map parameters and shared DAG remapping are
extracted to dedicated owners; all changed oversized files respect existing
pins. Global architecture still fails on five other recorded worktree files;
no pins were raised.

Exact transport now has a separate rational type. Existing f64 scene display
explicitly refuses it; this is an open consumer, not completed display support.
New WASM/package/worker delivery, exact display/edit/exchange, downstream exact
operations, separate NURBS graph integration, broader interrupted/multi-output
qualification and the original complete parity scope remain open.


### Source-owned rational Full Round display sampling — 2026-11-02

Added `prism::display::tessellate` and `cad_full_round_exact_display`. The public
boundary reconstructs/proves the frozen artifact, evaluates the actual rational
controls by exact De Casteljau and rounds only final display points. It never
creates or substitutes a legacy f64 BRep. Shared cap/wall samples preserve
through-holes and topology face ownership. Bounded sampling/triangulation,
binary64 collision, closed/oriented mesh and existing Float32 collapse/reversal
checks can refuse the display result. Source/body coefficients remain exact.

Four native integration tests passed: polynomial/rational profiles at 2/8/16
samples, through-hole cap preservation, resource/forgery refusals and independent
analytic rounded-box volume convergence. Receipt:
`../qualification/full-round-exact-display-2026-11-02/results.json`.
11 clean files unchanged; API map 242/200 passes, not parity. No architecture
pins changed; the same five other recorded worktree files keep the global gate
red. A TS transport helper is separate from the application's existing scene.

Uniform display sampling has no Hausdorff error certificate, self-intersection
proof or exact mass certificate. It is qualified only for this admitted exact
extrusion family. The scene builder still refuses exact output pending exact
artifact snapshot/edit ownership. Fresh WASM/package/worker delivery, broader
rational display, downstream exact operations/exchange and the entire original
parity scope remain open.


### Actual WASM source and exact display qualification — 2026-11-02

A fresh import-free WASM module passed 12 actual ABI cases: source-attested
Full Round graph construction; display at 2/8/16 samples; independent rounded-box
volume convergence; restoration and display of the native rational frozen
artifact with identical exact geometry/IDs; invalid sampling/triangle budgets,
forged geometry, changed source text and missing capability refusals.

WASM SHA-256: `da45c8d325c8295264649e09f075920ba5f4950cc1fd043a67355c726fecb5e9`. Retained qualified bytes:
`/tmp/rea-full-round-qualified/da45c8d325c8295264649e09f075920ba5f4950cc1fd043a67355c726fecb5e9.wasm`. Source program, original native artifact, WASM-generated artifact,
display data, build/runtime logs and selected source snapshots are saved in
`../qualification/full-round-display-wasm-2026-11-02/`. Receipt:
`../qualification/full-round-display-wasm-2026-11-02/results.json`.
The sampled rounded-box volume errors at 2/8/16 are
`[3.7579863461232037, 0.24324684347024572, 0.060921291870073446]` mm³. These are display convergence observations,
not exact mass or certified mesh-distance bounds.

The application package/kernel and scene builder were not changed. Exact
snapshot/edit ownership, actual packaged worker delivery, general rational
consumers, downstream operations/interchange and the full original parity
scope remain open. Native through-hole display coverage is in the previous
receipt; this WASM slice does not separately qualify through-hole rendering.


### Exact Full Round snapshot ownership — 2026-11-02

`NativeGeometryArtifact` now has a separate `rational-brep` kind.
`createExactFullRoundSnapshot` saves the entire native-verified frozen artifact;
`restoreExactFullRoundSnapshot` verifies storage metadata and then reconstructs
fresh native geometry/material/G1/radius proof, returning deeply frozen exact
geometry. No f64 BRep conversion occurs. Valid hashes do not authorize forged
geometry; wrong carriers/unknown payload fields/accessor metadata refuse.

Five opt-in snapshot tests passed with the SHA-pinned qualified real WASM
(only test module delivery is substituted; geometry uses the production host
kernel and Rust ABI). Thirty existing scene/contract regressions and
`vue-tsc --noEmit` pass. Tests verify lossless rational strings/IDs/recipe,
sampling-independent geometry revision and stale face selection after an actual
height edit. Receipt:
`../qualification/full-round-exact-snapshot-2026-11-02/results.json`.
11 clean files unchanged. Global architecture currently reports six other
worktree files; this change raises no pins and all changed files fit limits.

The saved document is metadata, not proof of parametric-history regeneration.
Application scene publication, exact edit/lineage transfer and released
kernel/worker delivery remain open, followed by general consumers/exchange and
the complete initial parity scope. The opt-in test does not qualify released
module delivery or worker bootstrap; without its external qualified WASM it is
explicitly skipped.


### Internal exact scene publication and root-map identity repair — 2026-11-02

The internal BRep scene adapter now publishes rational Full Round display with
its complete verified exact snapshot and actual topology face IDs. Sampling
changes retain source geometry, semantic entity and snapshot revision; native
face selection re-anchors onto the new mesh. Source geometry edits make prior
selection stale. Metrics remain display estimates, not certified exact mass.

The two-output test found a real bug: duplicated output rows of one root map
invocation were incorrectly compared as separate branches. TS and native
result admission now compare canonical invocation roots while retaining exact
node/owner/branch validation. Host/native owner-forgery regressions remain green.
Two Full Round outputs publish distinct semantic entities and snapshots.

Evidence: 7 actual-WASM scene tests, 199 host regressions, 3 native result tests,
12 fresh ABI cases and both TS typechecks passed. Cancellation during committed
snapshot disposal prevents publication; evaluation recovers after aborting a
computed exact node. WASM SHA: `a0d2c6d24432429cd88e14683f5f4256c6e0d914bf84d41752fb1951327f441c`.
Receipt and complete logs/source snapshots/ABI artifacts:
`../qualification/full-round-exact-scene-2026-11-02/results.json`.
11 clean files unchanged. No architecture pins changed; the same six other
recorded worktree files keep the global architecture gate red.

This qualifies the internal executable adapter with test-only SHA-pinned module
delivery, not production engine registration, the released package or actual
application Worker/bootstrap. Released source-bound delivery and full app
qualification remain open, as do general edit/history/lineage transfer,
downstream rational operations/exchange and the complete initial parity scope.


### Installed bounded release kernel and real browser worker — 2026-11-02

Built and installed the release package through the standard bounded packing
format. Kernel SHA: `c0701cb91cf7b719fad299c8ec1e713a820fea0521dd3150c8f778328c4f49c4`, 16225953 bytes; the 16 MiB decompression
limit and all other transport/deadline bounds remain unchanged. Previous local
package is preserved in `/tmp/rea-full-round-delivery-before`. Current qualified
bytes are retained at `/tmp/rea-full-round-qualified/c0701cb91cf7b719fad299c8ec1e713a820fea0521dd3150c8f778328c4f49c4.wasm`.

The initial optimized candidate exceeded the bound (17,130,275 bytes).
Additional pass/profile experiments were insufficient and were not adopted.
Final delivery stores the same default font losslessly compressed in WASM
(native font path unchanged) and omits implementation panic locations only in
release WASM. Typed public errors and source-language spans remain. All 75 SVG/
PDF tests pass, including every-font-byte equality; actual WASM Unicode PDF
output is byte-identical to the original storage path.

Added the browser diagnostic executor using the actual owned Worker. Real
browser testing exposed a worker receipt validator that admitted only f64 BRep
snapshots; it now correlates exact snapshot encoding/carrier/topology with the
admitted program without promoting metadata into a geometry certificate.
Five actual disposable workers passed on the installed package with no artifact
alias/compiler substitution: single/multiple Full Round output, invalid native
selection, cancellation/recovery and main-loop responsiveness. Wrong carrier,
encoding and face-list binding are explicitly rejected. All five workers were
terminated and the lane returned idle.

12 release ABI cases, 59 ordinary installed-kernel tests (exact snapshot/scene
cases now run without opt-in), 17 Node packaging tests (one optional native
optimizer case skipped), both TS checks and the production Vite build pass.
Package audit has exact roundtrip, no imports and unchanged export names/kinds;
this is not by itself whole-engine semantic equivalence. Vite reports large
chunk warnings. 11 clean files unchanged; the six other recorded architecture
violations remain, with no pins raised. Receipt, browser/ABI/font evidence,
build logs and source snapshots:
`../qualification/full-round-worker-delivery-2026-11-02/results.json`.

The real worker is still the explicit diagnostic lane, not a newly registered
production BRep provider or automatic fallback. Full app/distribution/performance
qualification, general edit/history/lineage transfer, broader rational operations
and exchange, and the complete initial parity scope remain open.


### Native exact Full Round history/regeneration — 2026-11-02

Added immutable proof-owned native History: source rebasing resolves the same
three authored source face IDs (including a reordered native face array), then
actually reconstructs/admit-proves Full Round. Explicit selection edits rebuild
on the current original source; a tested opposite-end selection changes actual
geometry. Undo/redo retain every revision; edits after undo require an explicit
prefix branch and preserve the original redo. Failed edits are transactional.

`exact-full-round-history/1` stores frozen choices and full expected artifacts.
Import freshly proves every revision, including hidden redo, verifies source-ID
continuity/selection semantics and preserves exact coefficients/naming/cursor.
It refuses forged geometry, even an independently valid body with wrong source
ID ancestry, inconsistent operation tags and insufficient caller budgets.
Actual material pair/atom/ray work is counted natively and aggregated over all
retained revisions; saved allowances cannot expand the caller's policy.

8 history tests plus 15 material regressions passed (23 total). Receipt:
`../qualification/full-round-native-history-2026-11-02/results.json`.
11 clean files unchanged; no architecture pins raised, six other recorded
worktree violations remain. The byte limit is a serialized-document allowance
when codec is enabled; this is not a total accounting of every independent
geometry proof subsystem.

This is native Rust feature history on the admitted extrusion family. Bridge/
WASM/editor/scene history integration is not yet done and the installed kernel
is not claimed to expose it. General source-construction/dependency graphs,
lineage transfer for changed source IDs, branch merges, general exact downstream
operations and the complete initial parity scope remain open. No proprietary
history code was transferred and Plasticity runtime history equivalence has not
been established.


### Full Round history public bridge and actual WASM ABI — 2026-11-02

Added `cad_full_round_exact_history` and an immutable TypeScript transport helper.
The router uses native bounded replay before every action, including creation;
unknown credit fields refuse. Three public bridge tests and fourteen real
import-free release WASM ABI cases pass. Native history documents and actual
material work counters replay identically; live source edits, actual reselection,
undo/redo and prefix branching are exercised. Hidden redo forgery, wrong edit
semantics and insufficient caller budgets refuse.

Evidence: `docs/qualification/full-round-history-bridge-wasm-2026-11-02/`.
Raw tested WASM SHA: b657f30eb89365e8cc695f915549dc0bf77eaf1c4d4826244cd9132c5950408c.
Installed history endpoint delivery and editor/scene history integration remain
open, followed by general dependency/lineage history and the entire original
parity scope. No runtime equivalence with Plasticity is claimed.


### Installed exact history endpoint — 2026-11-02

Optimized and installed history-capable WASM SHA
`e552c5d978af4690563c20211a9e40e745b87a832fe265129d5ff72ee5523fac`
(16,243,008 bytes, unchanged 16 MiB bound). All 14 optimized ABI cases,
14 ordinary installed-package history/snapshot/scene tests, TypeScript and
five actual Chromium Workers pass. Browser tests cover existing scene worker
behavior, not editor history actions. Added direct persisted-document restore
transport with fresh native replay; responses remain immutable.
Receipt: `docs/qualification/full-round-history-bridge-wasm-2026-11-02/results.json`.
Prior package preserved in `/tmp/rea-full-round-history-delivery-before`.
Editor/scene history ownership, general dependency/changed-ID lineage/branch
merges and all original library parity requirements remain open. Clean baseline
11 files unchanged; six other recorded architecture violations remain.


### Exact Full Round scene mesh history edits — 2026-11-02

Added `editExactFullRoundSceneMesh`: native history restore/current-body binding,
actual native edits, replacement mesh/BVH/edges and exact snapshot with persisted
history. Undo/redo survive existing scene serialization; display detail does not
change exact revision. Original source program remains attribution only.
Two installed-WASM integration tests and TypeScript pass. Valid but mismatched
history and native failures refuse without changing the input mesh. Multiple
triangle-dependent provenance runs require future lineage mapping and refuse.
Receipt: `docs/qualification/full-round-history-scene-2026-11-02/results.json`.
This is the scene library API, not editor command/store or history Worker wiring.
Aggregate scene metrics, history Worker cancellation, general dependency/lineage
history and the entire original parity scope remain open.


### Exact Full Round history browser Worker — 2026-11-02

Added an explicitly invoked browser worker lane for the native history scene-edit
API. Each operation owns a disposable actual Worker; input buffers are cloned,
not transferred. Bounded transport, concurrent admission refusal, hard abort,
deadline, remote failure and subsequent recovery are exercised in Chromium on
the installed package with no candidate substitution. Real platform Worker API
instrumentation records six constructions and six terminations; very short
expiry can precede Chromium's debug Worker event. Undo/redo exact revisions,
source edits, display-detail independence and original input retention pass.
TypeScript passes; clean baseline 11 unchanged. Existing six architecture
violations remain and Vite reports large chunks; no bounds changed.
Receipt: `docs/qualification/full-round-history-worker-2026-11-02/results.json`.
Editor commands/stores, aggregate scene metrics, general dependency/lineage
history, external Plasticity runtime comparison and all original parity scope
remain open. This lane is not a registered production provider.


### Earlier native Full Round selection regeneration — 2026-11-02

`History::regenerate_selection` now edits root/earlier explicit selection
parameters and freshly rebuilds every descendant, including hidden redo, while
retaining the cursor. Source rebases inherit authored face IDs; later explicit
selections retain their choices. Saved source models remain exact inputs.
Actual work/storage budgets are enforced and failed later-ID resolution is
transactional. Three new native tests and eight history regressions pass.
Receipt: `docs/qualification/full-round-history-regenerate-2026-11-02/results.json`.
Clean baseline 11 unchanged; six other recorded architecture violations remain.
Bridge/WASM exposure of this new method, earlier source-parameter edits,
source-expression dependency replay, general lineage/branch merges and the full
original Plasticity library parity scope remain open. Installed kernel unchanged.


### Актуализация полного остатка — 2 ноября 2026

Консолидированный текущий план: [plasticity-remaining-2026-11-02.md](plasticity-remaining-2026-11-02.md).
Регрессия текущего endpoint `regenerate-selection`: 4 bridge-теста, 17 raw ABI,
17 actual installed ABI и 7 настоящих browser Workers прошли. Текущий пакет
`1a70b8d7c3c94767c70df5c99b4a5fcc136161151df4a12cbf8a67cbe40ce99c`
появился в общей рабочей папке и не заменялся нашим raw-кандидатом; его
происхождение не выводится из совпадения endpoint-проверок. 11 clean-файлов
неизменны; свежий architecture gate красный на 7 файлах.
Receipt: `docs/qualification/plasticity-status-refresh-2026-11-02/results.json`.
Полный scope пересечений/Boolean, offset/replace, blends, quad/G2, patterns/
history, exchange и внешней квалификации остаётся открытым.
