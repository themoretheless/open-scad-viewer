# Roots on the actual trimmed support face

## Implemented native behavior

`face_crossings::inspect` checks immutable source/cutter geometry and persistent
identities. The selected support face must have one unambiguous shell use, a
proven simple outer UV contour with valid disjoint holes, and continuous
edge/pcurve lift agreement on every selected boundary use.

The native original-coefficient curve/surface query is run afresh. Each known
root is classified using its entire outward UV enclosure, including hole and
outer-contour winding. The report retains Inside, Outside and Unresolved roots
in original parameter order; `eligible` contains only Inside indices. It does
not merge roots by world position. A boundary/tolerance band does not authorize
membership.

Every unresolved surface-product cell is also classified by its whole UV box.
Only a proved Outside box can be excluded. Consequently the trimmed-face cover
can be complete even when the full untrimmed surface query remains incomplete,
for example when all unresolved tangency cells lie inside an excluded hole.
Inside or ambiguous unresolved cells keep the face cover incomplete.

`edge_partition::partition_at_face` accepts only complete face coverage and a
selected retained crossing. It uses the existing atomic child/lift checks and
shared vertex, preserves split lineage, and accounts for trim, boundary, root,
membership and child checks in one work ledger. Crossing direction includes the
selected shell face-use reversal. This is oriented crossing evidence, not a
Boolean material-inside certificate. The result still requires fresh global
solid qualification.

## Root search partition correction

Multiple intersections at centered UV coordinates exposed an artificial
midpoint-subdivision boundary. Search cuts now use a biased proposal. The actual
stored binary64 cut is authoritative, and an outward interval encloses its exact
ratio within the previous cell. Bernstein restriction uses this interval ratio,
so rounding a nonuniform cut cannot change the original parameter map. Both
children cover the previous cell; unchanged strict Krawczyk proof conditions
still grant every root. Difficult boundary/tangent cells remain explicit.

Existing proof limits are unchanged: native roots use at most 100000 cells,
1024 span products and 4096 coefficients per product; face preparation and
partition share at most 1000000 cells and a separate 10000 trim-pair ceiling.

## Native qualification

Five new face tests passed: holes and nonrectangular outer exclusion; valid
membership with reversed shell use; boundary ambiguity and exhausted work;
unresolved tangency excluded only when wholly inside a hole; and two actual
intersections where the first is excluded by a hole and retained index zero
correctly selects the second. Source and cutter inputs remain unchanged.

Eight edge-partition and eight original-boundary correspondence regressions also
passed. The revised root search passed eight certified curve/surface tests, six
curve/plane and five line/surface regressions: 40 distinct native tests total.
The correspondence suite took 77.75 seconds in this run. No UI/worker latency
qualification follows from these native results.

## Independent STEP evidence

The updated `owned_edge_partition_audit` exports four control cuboids, including
one partition authorized by a support face with an off-center hole. All four
fresh native audits passed exact lifts/joins, trim validity, boundary embedding
and volume admission. OpenCascade independently confirmed validity, one solid,
13 unique edges and 9 unique vertices, dimensions 10×8×12 mm and volume 960 mm³.
The vertex/edge counts establish that the exported partition survived import;
volume/bounds checks alone would not establish this. Maximum reported bounds
error was 1e-7 mm and volume error was zero.

`verify-cad-step-occt.py` now checks optional expected unique topology counts.
A manifest without such expectations reports that check as absent, not passed.
Generated evidence is in
`output/qualification/trimmed-face-partition-2026-10-15/`.

## Still required

This is root admission and edge partitioning. It does not author retained face
sectors or surface/surface trim curves, assemble fillet end transitions or
unequal-radius corners, qualify complete fillet thickness/G2, or complete the
bracket/flange/enclosure and history acceptance. Periodic/tangent root support,
WASM/worker/UI admission, P0 and remaining P2–P3 work stay open.

Two external-oracle regression tests passed (three real STEP variants): wrong
vertex count and wrong edge count each reject an otherwise valid one-solid
960 mm³ model, while a legacy manifest without topology expectations passes
and reports that optional check as absent. Run them with the qualified OCP
Python environment using `tests/test_step_topology_qualification.py` after
exporting the generated evidence. The run took 205.256 seconds, so no interactive
latency claim is made.

## Atomic all-root edge subdivision

`edge_partition::partition_all_at_face` now consumes all retained transverse
roots of one actual trimmed face in one transaction. Coverage is computed on the
immutable original curve and surface. Strictly separated root intervals are
processed in descending original parameter order, with a shared work budget.
Both incident face-local uses receive each shared vertex and child edge. Final
children are verified directly against the original curve to bound accumulated
subdivision rounding, retaining original natural parameter domains.

The two-root parabolic-face control creates three ordered children at x=2 and
x=8, with two shared vertices and two uses per child. It preserves naming,
passes continuous closure, and refuses an insufficient shared
budget without changing either input. Additional controls cover empty retained
root sets and unresolved trim-boundary membership. The focused edge_partition
and face_crossings integration suites contain 15 passing tests.

This is automatic edge subdivision, not yet general B-rep clipping. It does not
classify retained open face sectors, generate surface/surface contact curves,
remove material, construct unequal-radius corners, or grant solid admission.
