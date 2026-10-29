# Open cuts between vertices of one original hole — 2026-10-28

## Actual retained-face construction

`face_cut::partition_same_hole` takes an open saved-curve path whose endpoints are existing vertices on the same original hole. It forms two cycles from the path and the complementary original hole arcs. Complete simplicity and winding proofs must establish one positive material cap and one negative enlarged-hole boundary. The original outer sector keeps its outer boundary and the enlarged hole; a new bounded cap stays on the identical support with the original oriented shell use. Both material sectors remain. A path inside the original hole cannot create a false material face.

Existing other holes are assigned by full region separation/containment/orientation/non-nesting audits of both possible sectors. Endpoints must literally match their owned 3D vertices and actual UV points; all intermediate 3D/UV joins must be exact too. Sub-tolerance gaps do not authorize welding. Different-loop bridges, unresolved contours, crossing paths and insufficient proof work return no partial model. Inputs are validated before indexing their path controls. Complete source coedge windows are preserved; partial/directed/lifted windows refuse.

Original boundary world curves are copied unchanged, even when an old hole arc moves to the new cap. Every new path edge has opposite actual coedge uses in the two sectors. Both saved UV directions are directly checked against the original path, including rounded multispan reversals, and both actual world/pcurve/support lifts are continuously checked. World curves themselves remain literal input copies. Shared cell/pair limits include source audits/lifts, both sectors, hole assignment, reference checks and final new-edge lifts.

## Existing entry points and policy

The existing `partition`, `partition_projected`, projected network and projected retention APIs now resolve unique endpoints across all original outer/hole loops. Same-hole requests dispatch to the new constructor. Different-loop requests and ambiguous current-sector ownership still refuse. Sequential disjoint cuts on one current hole can retain two distinct caps without caller-managed descendant face indexes. Existing material seeds still undergo strict winding classification; no region is deleted by this planning stage.

Original local edge/vertex tolerance overrides and sewing marks are preserved through a shared topology-policy helper for outer, closed and same-hole subdivisions. The outer-loop route also directly verifies reversed saved UV references, rather than inheriting an exactness assumption from floating knot reflection. Optional coefficient-identity work remains separate from mandatory reference/lift cell work; the same-hole dispatch uses no optional coefficient identity stage and reports zero such work.

Adaptive audit allocation respects the requested cell/pair ceilings. A previous successful run's observed cell count is a report, not a promised minimal sufficient budget for a different allocation. A tighter retry may be inconclusive and must return an explicit unproven error rather than a partial construction. No model/UV tolerance, input geometry gate or proof ceiling was increased to qualify these cases.

## Native edited-body STEP

The STEP /9 source-preserving round trip is tested on the actual result of a rational curved hole cut. The imported model must have the edited face/edge counts, retain the new rational world curve with knot domain [10,14], preserve both pcurve domains [-3,5], pass both export/import topology certificates and remain watertight within tolerance. Exporting the original cuboid or its pre-cut topology cannot pass these checks. This is native Rust interchange evidence; it does not claim new WASM/Rush/editor transport or external CAD interoperability.

## Remaining full-plan scope

This adds actual same-hole face partitioning to existing native workflows. It does not yet construct a source-owned open SS path from the original root bands, solve arbitrary different-loop/junction networks, choose Boolean material roles automatically, sew the final operand result or grant a new Solid admission. Those source-binding and assembly stages remain required. General rational/multispan SS supports, unresolved contacts and periodic/partial windows remain open where their source proof is unavailable.

Central corner completion, offset cleanup, exact simplification, full WASM/Rush/UI/edited-STEP integration and current architecture/distribution gates retain their original scope. No new catalogue-count or complete-plan claim is made by this stage.

Final focused qualification passed **12/12** tests. The complete B-rep library passed **1948 tests, zero failed, five explicitly ignored**. Before the shared outer-reference/policy refinement, the full library passed 1946 tests. Changed-source rustfmt, diff whitespace and API mapping (239 public modules / 200 catalogue entries) pass. Architecture retains 71 existing violations with unchanged limits; its gate remains open.

Tests cover actual same-hole cap/enlarged-hole topology in an existing closed shell; reversed path role inference; other original holes inside/outside the cap; source world-curve and local policy preservation for both untouched and moved source edges and endpoint vertices; rational curved paths with independent 3D/UV knot domains; original outer-route UV-reference and policy preservation; rounded UV reversal with a direct source bound; a path inside the original hole; crossing paths, sub-tolerance endpoint gaps, malformed controls and different-loop refusals; existing direct/projected/network/retention routes; two sequential cuts on one current hole; adaptive cell/pair exhaustion with no partial output; and native STEP /9 export/import of the edited rational body with unchanged independent clocks and watertightness.

The first focused run's 6/7 result and its incorrect minimal-budget test assumption remain explicit. The corrected test permits only a result within the requested ceilings or a proof-unproven error; one-cell/one-pair failures and source immutability remain mandatory. No implementation budget, geometry gate or tolerance was enlarged. [matrix.json](matrix.json) records all exact source/log hashes, actual gate exits and the five ignored-test reasons. Browser assets and transport are unchanged by this native stage.
