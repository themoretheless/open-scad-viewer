# Physical SS arcs and contact-aware sewn periodic faces

Status: qualified and applied in main Rust. The catalog now has200native-or-integrated entries (79integrated,121native), following its explicit native completion rule. Qualification: `../qualification/nurbs-physical-branches-sewn-faces-2026-10-19/native-tests.json`. This document describes admitted domains rather than universal surface/surface intersection or universal manifold Body construction.

## Surface/surface physical branch assembly

`nurbs_core::ss_intersection::assemble_physical_fiber_branches` consumes a sealed complete automatically discovered original-root component owner. It produces physical branch records, distinct endpoint vertices and two original-support procedural trim projections per branch. `station` selects the original root at an original free coordinate and returns a fresh isolating `QualifiedFiber`; an interval midpoint is never published as an exact endpoint.

For every nonempty component, all root charts must use the same free coordinate, their free intervals must be nondegenerate and have a complete connected union, and all four chart bounds must remain in the original atlas domain. The already-qualified connection forest supplies actual common original roots, and the full atlas supplies the exhaustive root/exclusion census and cross-component separation. Each chart is a continuous uniform unique-root graph. On a common free interval, equality of two root graphs is closed by continuity and relatively open by their local uniqueness. A common actual root therefore glues the entire overlap. Induction along the original connection forest yields one graph over the complete union interval; no graph-nerve cycle count is used as a branch degree.

Both full original support rectangles must independently pass global injectivity. Equal world points then force both original UV pairs equal, so the root-set world map is injective. Compactness gives an embedded physical arc; different parameter components have disjoint physical images. Knots can retain corners: this API does not infer tangent continuity across them.

The exact trim is the original zero set `S1(u,v)-S2(s,t)=0`, projected onto each original UV chart with the common branch clock. This API does not replace that expression with a fitted NURBS curve. Existing qualified local trace builders remain available for approximation with their own explicit world-error proof.

Default limits:1024branches,10000injectivity span evaluations per support, refinement depth2,10000original rectangles per endpoint. Original component replay has its own retained allowance. Station queries independently bound their rectangle work. Invalid, exhausted or unproved requests return an error and never a partial physical branch owner. Empty original intersections produce no branches, vertices or trims.

Mixed-coordinate/closed branches, tangencies and singularities outside the original fiber admission, noninjective support charts and periodic source quotients are outside this constructor's admitted family. They must be subdivided/reparameterized with additional original proofs; no sampled marching result authorizes them.

Independent geometry regressions include the exact two roots of `v²-v+3/16`, analytic world stations `x=1/4+u/4`, a nonlinear original root graph, empty intersection and a folded support whose parameter component must not be promoted to a physical embedded arc.

## Original chart loops, contacts and patch ownership

`own_periodic_chart_loop_winding` reuses the exact prepared original source/frame expressions over the sealed original-edge loop partition. It returns each loop's winding and the complete vector. Boundary contact is explicit; arithmetic/allowance errors do not become an exterior label. The sum reproduces the original regularized material characteristic, with no periodic normalization of chart-plane queries.

The original chart-cycle owner now retains every actual source/frame germ rotation as `(edge,end)` port IDs. These are rebuilt from exact displacement sectors and already-certified higher-germ ranks. `own_periodic_chart_fan_patches` checks every pair of loop turns at every shared vertex: their two port pairs must not alternate in the exact cyclic order. Vertex-simple original-edge loops therefore have noncrossing contacts rather than silently inferred crossings. An original source-qualified transverse edge witness then proves the constant inside/outside relation of each other noncrossing loop. The complete signed containment matrix must form an alternating parent hierarchy, including all transitive ancestor relations.

A positive loop owns its direct negative holes; a nested positive island owns another patch. Shared vertices remain explicit. Actual touching filled regions and an actual tangent hole exercise separate material fans and negative-hole containment. Arbitrary point sampling or polygon area does not authorize a patch.

## Actual supporting-surface faces and native seam sewing

`brep_core::profile_certificate::own_periodic_sewn_faces` publishes sealed contact-aware face, vertex, edge and coedge records. The original supporting surface must pass the existing complete rank/local chart/cross-chart embedding qualification, modulo only its actual admitted Surface seams. The embedding's wire owner must be the very same augmented original support owner.

World edges retain either an original selected source trace (including original algebraic clocks and cuts) or an original native Surface iso-coordinate interval. `world_point` evaluates that exact original composition at an admitted rational native clock. No shifted floating-point control net, rounded algebraic endpoint or independently fitted world curve is authored.

Every retained auxiliary frame edge needs exactly one original opposite-shore counterpart. Sewing verifies the same fixed axis, opposite actual end, auxiliary role, opposite CCW orientation, an actual full Surface seam identity, and exact equality of both original varying-coordinate endpoint selectors. Equality of complete native intervals proves whole world-edge equality; a corner or midpoint coincidence is insufficient. Unpaired auxiliary intervals, including authored boundaries lying exactly on the seam without extra admission, refuse.

A chart vertex is first normalized into its actual incoming/outgoing material fans. Opposite edge sewing joins the corresponding fan-link ends, preserving separate unrelated point-contact fans. Coedges retain every original chart edge and the opposite orientation of paired uses. Sewing joins incident chart patches; a complete bijective successor census then removes auxiliary edges from the actual remaining material boundary cycles.

Chart patch Euler data uses the number of ORIGINAL material-fan boundary walks (`2-b` for each planar normalized patch), not `1-number_of_holes`. This distinction matters for a hole tangent to the outer boundary. The exact seam quotient supplies the final vertex/edge counts; connected oriented face Euler data must yield a nonnegative integral genus. Counts are consistency checks after the original fan and sewing proofs, not substitutes for those proofs.

The abstract surface normalization is contact-aware: unrelated material fans can meet at a source-owned world point while retaining separate topological vertices. This is not a claim of a contact-free closed manifold Body or a serialization into the viewer's legacy mesh model.

Defaults:100000chart edges,1000000seam candidate comparisons,100000regularity cells, existing injective atlas defaults and a complete chart-pair budget of1000000span evaluations with refinement depth4. Each lower-level certificate retains its own explicit work allowance. Unknown embedding, unmatched/multiple counterparts, incomplete boundary incidence or exhausted work refuses without a partial sewn face owner.

Native geometric gates include a full actual torus with both axes and corners sewn (one genus-one closed face, no authored trim), a curved seam-crossing trim with irrational original cuts and exact world evaluation, point-touching filled components, and a tangent hole's normalized material boundary. Existing Boolean regressions and original cycle/strict hierarchy cases remain in the selected qualification set.

## Closed and degenerate extension

The separate [exact quadratic-graph SS constructor](nurbs-closed-degenerate-ss.md) extends the admitted family with closed ellipses and explicit singular/overlap owners. The regular open-arc proof described here retains its original domain. See the linked qualification receipt for current validation status.
