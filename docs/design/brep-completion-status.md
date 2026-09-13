# B-rep completion audit

Lattice decimation, lightening cell construction and FDM print-settings fitting now execute in Rust. The native edge-collapse decimator reproduces legacy ordering — midpoint contraction, cumulative-radius tolerance guard, 0.2 normal-dot fold rejection, 1e6 coordinate compaction, one-million attempt cap — and now refuses negative or non-finite tolerance at admission instead of silently returning the input. Lightening cells, the 125-node spatial graph and the clip/inset/cutter/Boolean lightening pipeline sit behind typed transport-only adapters; nozzle/layer quantization (1e-8 epsilon, 112.5% line width, 1e6 compaction) and the bridge warning moved to cad_lattice_print_fit / cad_lattice_bridge_warning with the legacy user-facing refusal message. [Scoped evidence](../qualification/brep-native-lattice-decimation-v1.json): 56 tests in five files, both typechecks and 68 verified artifacts (4,637,132 bytes). Determinism, target-bound reduction preserving closed oriented meshes, malformed-input and admission refusal, legacy print-fit values and spatial/lightening/workbench regressions pass. The per-body lighten orchestration loop in cadWorkbench.ts and the panel volume-reduction percentage remain host/presentation TS; this is scoped evidence, not full B-rep completion.

Spatial lattice graph preparation and indexed component checks now execute in Rust. Node/edge order and unsigned seeded jitter match captured legacy fixtures; component volumes use local origins and compensated summation for remote pieces. [Scoped evidence](../qualification/brep-native-lattice-preparation-v1.json): 70 tests in four files, both typechecks and 68 verified artifacts (4,629,160 bytes). The 125-node limit, malformed inputs, negative-volume components, 1e12 placement and spatial/bone/source-roundtrip regressions pass. Lattice decimation, planar lightening construction and host operation orchestration still contain TS computation. Indexed numerical component checks do not close B-rep topology or the full requirements.

Surface texture now runs entirely in Rust: six pattern heights and deterministic noise, shared-edge subdivision, area-weighted displacement normals, selection-boundary fade, triangle orientation checks and result validation. TS retains only typed transport. [Scoped evidence](../qualification/brep-native-surface-texture-v1.json): 71 tests in five files, both typechecks and 68 verified artifacts (4,627,221 bytes). Analytic landmarks, captured legacy noise fixtures, source/parser roundtrip and atomic refusal pass. Retained B-rep now explicitly refuses instead of pairing a modified mesh with stale topology. Limits are 40,000 refined triangles and 20 million boundary-distance pairs. These local numerical mesh checks do not prove global B-rep validity; analytic texture, shared qualification refresh and full requirements remain open.

Convex mesh push/pull, chamfer, faceted fillet and shell now execute entirely in Rust, including convex admission, support-plane intersections, shared-vertex face construction, cavity/cutter preparation and result validation. directSolidTools.ts contains transport and presentation metadata only. [Scoped evidence](../qualification/brep-native-mesh-planes-v1.json): 82 tests in seven files, both typechecks and 68 verified artifacts (4,621,275 bytes). Independent prism/cavity volumes, adjacent-support survival, multi-opening shells and atomic failure cases pass. Fixed-tolerance mesh construction and faceted blends do not close certified B-rep; shared qualification still needs refresh and the full requirements remain open.

Mesh plane splitting now runs as one native transaction: Rust admits the plane, constructs and places the cutter, performs both Booleans and validates both closed halves before returning. The TS adapter retains only transport and UI names/IDs. [Scoped evidence](../qualification/brep-native-mesh-split-v1.json): 73 tests in five files, both typechecks and 68 verified artifacts (4,622,021 bytes). Axis/oblique and concave cuts match independent volumes; extreme normal scaling and atomic refusal pass. Shared qualification below predates this kernel; numerical mesh splitting does not close general B-rep or the remaining Rust migration.

Shared qualification refreshed after native workbench machining/Boolean chains and ruled B-rep loft with Solid preview/history integration. SVG 122 and own-CAD 218 reports bind the same rebuilt WASM; 335 expanded tests in 44 files pass without failures/pending cases, plus three manifest tests. Native B-rep 107, polygon-core 73 and ModelGraph-runtime 56, both typechecks and 68 distribution artifacts (4,624,043 bytes) pass. [Checkpoint](../qualification/brep-shared-ruled-loft-v1.json). Prior stale-shared-report caveats below are historical for this checkpoint. The full requirements table remains incomplete; regression qualification does not close general B-rep or Rust migration.

Ruled loft correspondence admission now refines quadratic Bernstein support bounds by bounded de Casteljau subdivision. A failing native fixture reproduced the former rejection of a valid 90-degree section rotation; 90/120/150-degree cases now pass with volumes matching an independent section-area integral, while collapsed twists still refuse. [Scoped evidence](../qualification/brep-ruled-loft-bound-refinement-v1.json): 107 native B-rep tests, 86 integration tests, both typechecks and 68 verified artifacts (4,624,043 bytes). Depth 24 and a shared one-million-node budget bound work with typed unresolved/resource outcomes. Coefficient arithmetic is not fully certified; general loft and B-rep completion remain open.

Solid editor now exposes B-rep loft for ordered Shift-selected polygon sketches. Native selection/placement constructs the ruled model and display mesh; UI uses a stable preview ID and commits the full B-rep document. [Scoped evidence](../qualification/brep-ruled-loft-solid-ui-v1.json): 62 tests in four files, both typechecks and 68 verified artifacts (4,622,408 bytes). Headless Vue tests verify nonmutating preview, Apply retaining six B-rep faces and correct volume, exact Undo, and disabled Apply for reversed section order. This preserves the local Solid document, not topology through SCAD mesh export. General loft, provider integration and full completion remain open.

A new native ruled B-rep loft retains bilinear side surfaces and trimmed planar caps with shared edge ownership across stations. It reuses parallel convex section admission and checks positive Bernstein support coefficients for intermediate profiles; this is sufficient numerical admission, not a complete certificate. Rust/WASM exposes brep_nurbs_ruled_loft via createRuledBrepLoft. [Scoped evidence](../qualification/brep-native-ruled-loft-v1.json): 105 native B-rep tests, 71 integration tests, both typechecks and 68 verified artifacts (4,620,769 bytes). Rotated-section volume matches an independent section-area integral, display refinement converges, multi-interval topology closes, and collapsed correspondences refuse. Editor/schema integration, general smooth loft and full B-rep completion remain open.

Workbench Boolean chains now run through one Rust adapter for homogeneous mesh or retained B-rep operands. Rust admits operands, preserves order, reduces the chain and returns a body list; mixed representation refusal is native. Mesh empty results now remove selected bodies consistently with B-rep, replacing the prior host-side empty-result error. [Scoped evidence](../qualification/brep-native-mesh-boolean-fold-v1.json): 71 tests in five files, both typechecks and 68 verified artifacts (4,619,154 bytes). Ordered union, repeated difference, empty intersection, late-invalid input and retained-body regressions pass. General Boolean coverage and full completion remain open.

Workbench hole operations now share one Rust entry point: retained bodies use the existing native B-rep path, while mesh bodies build/place 48-sided cutters, combine counterbores/countersinks and subtract natively. The remaining host cutter formulas and vector placement were removed. [Scoped evidence](../qualification/brep-native-mesh-hole-v1.json): 64 tests in four files, both typechecks and 68 verified artifacts (4,617,458 bytes). Six mode/axis cases match polygonal prism/frustum volumes; invalid geometry refuses without mutation. General retained countersinks, full B-rep completion and shared qualification refresh remain open.

Workbench mesh threading now runs as one Rust transaction: generate the faceted cutter, place it along the requested axis, perform union/difference, inspect the closed result and return updated body metadata. Retained B-rep refusal is enforced natively. [Scoped evidence](../qualification/brep-native-thread-body-v1.json): 66 integration tests, both typechecks and 68 verified artifacts (4,617,610 bytes). X/Z cuts match cutter volume; external partial union matches independent triangle clipping/integration, replacing an invalid half-length/half-volume assumption without relaxing tolerance. Analytic B-rep threading and full completion remain open.

The TS helical thread generator and radius formula are replaced by typed Rust/WASM adapters to the existing ModelGraph-runtime generator. Native generation now exposes mesh buffers alongside source/report; malformed public inputs are admitted before unchecked field access. Workbench threading of retained B-rep now refuses instead of changing its mesh while keeping stale B-rep. [Scoped evidence](../qualification/brep-native-thread-generator-v1.json): 56 native runtime tests, 65 integration tests, both typechecks and 68 verified artifacts (4,617,040 bytes). This remains a faceted basic thread; analytic retained threading, remaining mesh-operation orchestration and general completion remain open.

The direct point transformation API now computes centroid, rotation, scale and translation in Rust; TS only transports the batch. The centroid sums divided coordinates to avoid overflowing a representable average. [Scoped evidence](../qualification/brep-native-direct-point-transform-v1.json): 88 tests in seven files, both typechecks and 68 verified artifacts (4,619,650 bytes). Mixed 2D/3D dimensions, large finite centroids, malformed/overflow refusal and recovery pass. Other host geometry and full B-rep completion remain open.

Convex B-rep profile admission now checks every nonincident vertex against every oriented edge support. A failing native regression reproduced the previous acceptance of a same-turn pentagram; adjacent turn signs alone also admitted repeated loops. [Scoped evidence](../qualification/brep-convex-profile-admission-v1.json): 104 native B-rep tests, 74 integration tests, both typechecks and 68 verified artifacts (4,618,785 bytes). WASM loft/sweep reject stars and double traversals, preserve inputs and recover with a valid pentagon. These remain numerical fixed-tolerance tests, not certified global solid validity; full B-rep completion remains open.

Native faceted B-rep loft now accepts consistently oriented parallel convex sections in an arbitrary rigid frame, replacing its global horizontal/Z-order restriction with local-plane admission. World coordinate bounds remain authoritative; translated local coordinates do not add a tighter bound. [Scoped evidence](../qualification/brep-loft-parallel-frame-v1.json): 103 native B-rep tests, 73 integration tests, both typechecks and 68 verified artifacts (4,619,440 bytes). A rotated translated frustum is closed with volume 28 cubic mm; nonparallel/reverse ordering refuse. Side surfaces remain planar facets, not smooth analytic loft; general loft and full B-rep completion remain open.

Workbench loft/sweep now submit complete sketch/selection requests to Rust. Native preparation selects and validates profiles, places section/path coordinates, resamples unequal sections and constructs/inspects the output mesh; TS applies document IDs and names. [Scoped evidence](../qualification/brep-native-workbench-sections-v1.json): 81 tests in six files, both typechecks and 68 verified artifacts (4,618,755 bytes). Workbench results remain polygonal solids; existing restricted faceted B-rep constructors were not substituted for broader mesh behavior. General B-rep loft/sweep, SectionMatch/FrameLaw and full completion remain open. Shared qualification predates this change.

Shared qualification refreshed after native sketch operations, point/path placement and retained prism draft. SVG 122 and own-CAD 218 reports bind the same rebuilt current WASM; 306 expanded tests in 41 files pass with no failures or pending tests, plus three manifest tests. Native B-rep 101 and polygon-core 73, both typechecks and 68 distribution artifacts (4,616,974 bytes) pass. [Checkpoint](../qualification/brep-shared-sketch-draft-v1.json). Prior shared-artifact caveats below are historical for this checkpoint. This is scoped regression evidence; the full requirements table remains incomplete.

Workbench path arclength and point interpolation now execute in Rust. Loft requests each section sample batch through the native adapter; source segment lengths are computed once per batch. [Scoped evidence](../qualification/brep-native-path-sampling-v1.json): 80 tests in six files, both typechecks and 68 verified artifacts (4,616,974 bytes). Repeated vertices, endpoint clamping, 2D/3D dimensions and invalid/resource refusal are covered. Analytic arclength certification, general section matching, further host migration and full B-rep completion remain open.

Sketch local-to-world point placement now executes in Rust, including basis cross-product, placement and finite output admission. TS point helpers transport requests; extrusion, workbench loft/sweep paths and mesh cutters batch their vertices. Loft placement is computed once per section. [Scoped evidence](../qualification/brep-native-world-points-v1.json): 78 tests in six files, both typechecks and 68 verified artifacts (4,615,605 bytes). Existing scaled/skew basis semantics remain; this is not an orthonormal workplane certificate. Other host computation and general B-rep completion remain open.

Zero-angle prism draft now returns the admitted authored model unchanged, avoiding unnecessary topology reconstruction. Two native regressions check complete model identity, signed draft vertex coordinates, surviving caps, axis magnitude 1e300 and collapse/incompatible-axis refusal. [Scoped evidence](../qualification/brep-prism-draft-invariants-v1.json): 101 native B-rep tests, 58 integration tests including WASM model equality, both typechecks and 68 verified artifacts (4,615,510 bytes). General B-rep completion remains open.

Retained convex planar prisms now support native geometric draft about an axis-normal neutral plane. Rust tilts lateral supports, keeps caps fixed, rebuilds B-rep topology and derives its display mesh in one transaction. [Scoped evidence](../qualification/brep-native-prism-draft-v1.json): 99 existing native B-rep tests, 58 integration tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,615,200 bytes). Signed angles, shifted neutral plane, rotated placement and independent volume formulas pass; vanished supports and curved/oblique unsupported inputs refuse. The prior blanket retained-draft refusal is superseded for this prism subset. General selected-face/curved draft, certified validity and full B-rep completion remain open.

Mesh draft now executes in Rust; the host only submits body records and applies the returned transaction. The previous host deformation preserved stale B-rep while changing its mesh. Retained or mixed B-rep draft requests now explicitly refuse before modification until topology-aware draft is implemented. [Scoped evidence](../qualification/brep-native-mesh-draft-v1.json): 56 tests in four files, both typechecks, WASM/Vite builds and 68 verified artifacts (4,613,040 bytes). This is mesh vertex deformation, not analytic face draft; retained B-rep draft, general completion and other host migration remain open.

Sketch polyline trim and endpoint extension now execute in Rust, including boundary intersection search, chain construction, nearest forward hit selection and extension self-intersection validation. TS entry points are transport adapters. [Scoped evidence](../qualification/brep-native-sketch-trim-v1.json): 71 tests in six files, both typechecks, WASM/Vite builds and 68 verified artifacts (4,612,983 bytes). Output point-limit refusal and recovery are tested. This remains numerical local-coordinate polyline geometry; analytic trimming, cross-workplane arrangements, other host geometry and general B-rep completion remain open. Shared qualification predates these sketch changes.

Sketch miter offset and bounded simple-contour validation now execute in Rust. Analytic arcs/circles retain parameters with adjusted radius; polygon offsets check self-intersection and collapsed edges. TS implementations were replaced by transport adapters. [Scoped evidence](../qualification/brep-native-sketch-offset-v1.json): 66 tests, both typechecks and 68 verified artifacts (4,611,341 bytes). This does not implement multi-loop topology-changing offsets or certify numeric contours; trim/extend, other host geometry and general B-rep requirements remain open.

Circle/arc sampling and planar sketch translation/rotation/scaling now run in Rust; analytic parameters are retained and display points are resampled natively. TS entry points only transport arguments. [Scoped evidence](../qualification/brep-native-sketch-transform-v1.json): 63 tests including direct editor/profile workflows, both typechecks and 68 verified artifacts (4,613,804 bytes). Other contour operations and document semantics remain host-side; sampling is not certified approximation and general B-rep completion remains open.

Shared qualification refreshed after the native editor, Boolean/hole and proximity changes: SVG 122 and own-CAD 218 reports bind the same current WASM; an expanded run passes 288 tests in 41 files with zero failures/pending tests. Native B-rep 99 and polygon-core 73 tests, both typechecks and 68 distribution artifacts (4,612,647 bytes) pass. [Checkpoint](../qualification/brep-shared-editor-proximity-v1.json). Earlier shared-artifact caveats below are historical for this checkpoint. These are scoped regression results; general certified geometry, production provider, remaining host migration, exchange and release requirements remain incomplete.

Native closest-point-on-triangle now normalizes relative coordinates and the cross-product normal before projection and barycentric tests. A tiny-scale regression previously returned an edge point instead of the interior projection. [Evidence](../qualification/brep-native-triangle-distance-conditioning-v1.json): 73 polygon-core tests, 32 integration tests including reconstruction and empty consumers, both typechecks and 68 verified artifacts (4,613,082 bytes). Interior/edge/vertex regions and vertex permutations are tested across 1e-100..1e100; this remains floating-point proximity, not certified B-rep distance or full completion.

Native mesh-clearance segment minima now normalize coordinates and use cross-product parameters rather than an absolute parallel threshold and subtractive squared-dot determinant. Two failing regressions were reproduced before correction: tiny-scale skew segments and nearly parallel crossing segments. Nonfinite distance candidates explicitly refuse. [Evidence](../qualification/brep-native-clearance-conditioning-v1.json): three native tests (including scale/reversal matrix and transverse components down to 1e-200), 30 integration tests, both typechecks and 68 verified artifacts (4,612,677 bytes). This improves numerical robustness without claiming certified mesh or analytic B-rep distance.

CAD pair clearance and mesh-overlap inspection now execute in Rust, reusing native closest-triangle and Boolean code with bounded exhaustive edge/triangle comparisons. `cadInspection.ts` is a transport adapter. [Scoped evidence](../qualification/brep-native-clearance-v1.json): 30 integration tests, native segment cases, both typechecks and 68 verified artifacts (4,612,592 bytes). This is floating-point display-mesh measurement; certified analytic B-rep distance, numeric conditioning across the full domain and complete migration remain open.

Native pattern expansion now measures one prepared group before constructing remaining instances and checks multiplied byte/node counts and nesting against existing binary transport limits with envelope reserves. [Scoped evidence](../qualification/brep-native-pattern-capacity-v1.json): 34 integration tests, one native wire-footprint test, both typechecks and 68 verified artifacts (4,611,450 bytes). Oversized expansion refuses with source unchanged and subsequent requests recover. Preparing one group still allocates; this is not a browser peak-memory certificate or full B-rep completion.

Plain holes and counterbores in retained workbench bodies now construct rational cutters, place them and perform Boolean subtraction in Rust, returning updated B-rep and display mesh together. [Scoped evidence](../qualification/brep-native-hole-v1.json): 31 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,610,870 bytes). Through/blind/sideways/repeated holes and stepped counterbores match analytic volumes. Countersink conical Boolean explicitly refuses; general angled intersections, retained-body thread/draft and remaining migration are not complete.

Workbench Boolean operations on retained bodies now use ordered native B-rep reduction and complete result tessellation, eliminating the stale-B-rep/changed-mesh path. Regularized empty results remove selected records; unrelated bodies survive. Mixed retained/mesh-only inputs explicitly refuse. [Scoped evidence](../qualification/brep-native-workbench-boolean-v1.json): 55 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,609,128 bytes). Coaxial cylinder subtraction retains rational walls; general curved Boolean, mixed-representation conversion and full migration remain incomplete.

The direct editor split preview now routes through the shared native retained-body split service instead of separately calling the lower-level B-rep operation and host tessellation. Positive-side identity/order is consistent with mesh splitting. [UI checkpoint](../qualification/brep-native-split-ui-v1.json): 30 tests across the editor and retained-body suites, both typechecks and 68 verified distribution artifacts (4,608,092 bytes). The UI test verifies unchanged preview state, committed B-rep/mesh volumes, fresh second ID and Undo; general split support and full B-rep remain incomplete.

Retained-body chamfer and faceted fillet editing now performs native display edge resolution, connected-chain operation and complete result tessellation in one request. The shared service preserves B-rep instead of entering its mesh-only algorithm, and Vue uses the same batch path. [Scoped evidence](../qualification/brep-native-edge-editor-v1.json): 39 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,608,241 bytes). Fillets remain explicitly planar tangent facets; analytic blends, general offsets and remaining native migration are not complete.

Retained-body push/pull and open planar shell now execute native display grouping, unique support admission, B-rep editing and result tessellation in one Rust request. The editor and shared service use this path; successful results retain B-rep, while unsupported curved inputs refuse without mesh fallback. [Scoped evidence](../qualification/brep-native-planar-editor-v1.json): 39 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,606,032 bytes). Operations remain convex planar and support admission remains numerical; mesh-only legacy editing and general curved features are not complete.

Editor face workplane construction now runs in Rust, including vertex admission, tangent direction, oriented basis and coplanarity checks. `facePlane` only transports mesh and selection data. [Scoped evidence](../qualification/brep-native-face-workplane-v1.json): 49 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,604,810 bytes). Tilted-face local reconstruction and existing vertical drawing/extrusion workflows pass; general surface frames and full B-rep completion remain open.

Display mesh face grouping, normals, offsets, centers and edge adjacency now run in Rust; `solidTopology` is a thin host adapter. Degenerate triangles cannot create undefined face references, and plane comparisons have an explicit 20,000,000 work limit. Fixed-seven seam rounding preserves the legacy half-tie rule using integer arithmetic. [Scoped evidence](../qualification/brep-native-display-topology-v1.json): 55 integration tests, one native rounding regression, both typechecks, WASM/Vite builds and 68 verified artifacts (4,604,230 bytes). Numerical display grouping is not authored B-rep topology, persistent naming or general solid certification; other host geometry remains.

Legacy editor straight-edge admission now runs in Rust: selected vertices must form a mesh edge and match exactly one authored straight edge under the model tolerance. Matching endpoints alone no longer admits a curved-edge chord; native control-net collinearity and segment-range checks are required. Vue endpoint comparisons were removed. [Scoped evidence](../qualification/brep-native-edge-selection-v1.json): 34 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,602,188 bytes). This does not establish general curved-edge correspondence, persistent naming or full migration of host topology grouping.

Legacy editor planar support selection now runs in Rust, validating model, mesh, triangle indices and planarity before requiring exactly one authored support. The Vue component no longer projects control points onto the support plane. [Scoped evidence](../qualification/brep-native-support-selection-v1.json): 35 tests, both typechecks, WASM/Vite builds and 68 verified artifacts (4,600,964 bytes). Three display details agree with tessellator face IDs; ambiguous coplanar supports refuse. This remains numerical support-plane admission, not certified trimmed-face correspondence or persistent naming; explicit revision-bound display identity transfer remains open.

Direct editor split now dispatches retained B-rep bodies to the native convex-planar operation and prepares both result meshes before returning. B-rep is preserved for subsequent edits; unsupported curved input refuses rather than falling through to mesh CSG. Bodies without retained B-rep still use the earlier mesh path. [Scoped evidence](../qualification/brep-native-editor-split-v1.json): 34 tests across six files after correcting a signed-zero-only bounds assertion, both typechecks, WASM/Vite builds and 68 verified artifacts (4,600,044 bytes). Native push/pull editor integration still requires trustworthy selected-face correspondence. Shared qualification below predates this new split integration.

Shared qualification has now been refreshed after the native transform migrations. SVG (122) and own-CAD (218) reports bind the same current WASM; 32 manifest/transform checks, 99 native B-rep and 72 polygon tests pass, as do native SVG/planar suites, both typechecks and the 68-artifact distribution check (4,598,718 bytes). [Checkpoint](../qualification/brep-shared-qualification-cad-transforms-v1.json). Earlier statements that shared qualification predates these transform changes are historical. This is scoped regression evidence; the requirement table still contains open general B-rep, certification, semantic execution, provider, exchange and release work.

Directional and placed-sketch-path body patterns now compute normalization, world path points, arc-length stations and group translations in Rust. Each copy transforms mesh and retained B-rep together; the host assigns new body IDs after successful batch return. [Scoped evidence](../qualification/brep-native-body-pattern-v1.json): 43 integration tests, both typechecks, WASM/Vite builds and 68 verified distribution artifacts (4,598,718 bytes). Maximum repetition peak memory remains unqualified; general B-rep and remaining host geometry migration are incomplete.

Slider and revolute joint movement now compute their axis, translation and offset-pivot rotation in Rust, updating mesh and retained B-rep together. Parent and unselected records remain unchanged; host joint persistence is still separate. [Scoped evidence](../qualification/brep-native-joint-transform-v1.json): 40 integration tests including saved-joint/source-rebuild behavior, both typechecks, WASM/Vite builds and 68 verified distribution artifacts (4,596,412 bytes). General B-rep and remaining migration are incomplete.

Body alignment and distribution now compute axis admission, bounds keys, ordering and translations in Rust and move retained B-rep together with meshes. The workbench adapters only pass selected records and replace returned bodies. [Scoped evidence](../qualification/brep-native-body-arrangement-v1.json): 38 integration tests, both typechecks, WASM/Vite builds and 68 verified distribution artifacts (4,596,086 bytes). Pattern, joints, draft and other geometry remain outside this migration; general B-rep completion is not proved.

Mixed body/sketch selection transforms now execute entirely in Rust: shared bounds/pivot, rotation and scale, mesh and retained B-rep, sketch plane placement and analytic arc/circle parameters. `transformSelection` is a transport adapter; its host arithmetic and control-point mutation were removed. Native admission refuses nonfinite generated sketch geometry before returning a document. [Scoped evidence](../qualification/brep-native-selection-transform-v1.json): 61 integration tests, both typechecks, WASM/Vite builds and 68 verified distribution artifacts (4,593,767 bytes). This supersedes the earlier mixed-selection migration gap below, but does not complete other CAD geometry or general certified B-rep.

Body-only editor selections now use native group translation, axis rotation and uniform scaling. Rust derives the common pivot, normalizes the axis and atomically transforms mesh and retained B-rep. Mixed body/sketch selections still require migration. [Scoped evidence](../qualification/brep-native-body-transform-v1.json): 47 integration tests, both typechecks, WASM/Vite builds and 68 verified distribution artifacts (4,590,732 bytes). Full completion remains unproved.

Native CAD mirror now constructs the plane reflection in Rust and atomically transforms both mesh and retained B-rep, including orientation reversal. The TS branch only transports body records and applies copy/replacement bookkeeping. Offset oblique planes, involution, copy mode, invalid-late-body refusal, and recovery pass through WASM. Evidence: [native CAD mirror](../qualification/brep-native-cad-mirror-v1.json): 72 native polygon tests, 71 application/integration tests, both typechecks, Vite and 68 distribution artifacts (4,590,491 bytes). This scoped result does not complete general B-rep or migration of remaining host computations.

Date: 2026-09-12; updated 2026-09-13. Objective: **«цель B-rep завершен полностью, перенести все в rust»**.

This is a working engineering checklist, not a qualification certificate or a
replacement for the requested scope. The implementation is substantial, but
full completion is **not proved**. A successful primitive, mesh validation, or
test run is evidence only for the behavior it actually exercises.

The scope comes from [the 14-stage master plan](brep-nurbs-14-stage-master-plan.md)
and [the kernel design](rust-brep-nurbs-kernel.md), including their later feature
contracts. Their historical implementation stop instructions do not supersede
the user's current explicit implementation request. Their geometric, numerical,
identity, integration, and verification requirements still identify work needed
before a full-completion claim. No approval is requested by this audit.

## Current executable foundation

CAD group resize now executes as one native operation. Rust derives a shared
affine transform about the selection minimum and applies it to both polygon
meshes and retained B-reps, preserving body records. This fixes the previous TS
path that resized display positions while retaining stale B-rep geometry.
Any invalid later B-rep rejects the complete result before host replacement.
Tests verify group placement, source/unselected-body preservation, mesh/B-rep
box volumes and a rational sphere resized into an ellipsoid while retaining
its surface weights and UV loops. All 71 polygon-core and 69 CAD/B-rep/UI/MCP/
export/packing tests pass. Vue/MCP typechecks, Vite and 68 distribution artifact
checks pass (4,589,631 bytes). Other CAD operations remain to migrate; shared
qualification predates this WASM and full B-rep completion remains open.

Selected-body bounds used by CAD placement, inspection and drawings now execute
in Rust. The host transfers placed position groups and receives min/max arrays.
The native reduction retains binary64 coordinates and unreferenced vertices,
ignores empty groups when other vertices exist, and rejects empty selections,
incomplete triples and nonfinite coordinates. All 70 polygon-core and 67
CAD/direct-modeling/UI/export/MCP/packing tests pass. Vue/MCP typechecks, Vite
and verification of 68 distribution artifacts pass (4,588,641 bytes). Other
CAD transformations and layout calculations still require migration. Shared
qualification predates this WASM; full B-rep completion remains open.

Scene flattening's full capacity is now exercised through the real WASM
transport: 100,000 expanded triangles with 300,000 distinct vertices retain
binary64 translated coordinates and complete indexing. Adding one distinct
unused vertex produces the expected 300,000-vertex refusal; the prior returned
result remains intact and a subsequent small request succeeds. All four focused
transport tests pass. Runtime source and WASM hashes match the preceding native
scene-flatten checkpoint, whose broader checks remain applicable. This verifies
functional capacity, not browser peak-memory or latency qualification.

Scene flattening for export and direct-modeling body exchange now runs in Rust
(`polygon-core::scene_flatten`). The TS adapter only transfers buffers and
returns the owned result. Native placement preserves the established
translation-first binary64 operation order, flips triangle winding for
reflections and welds only equal finite coordinates (signed zeros share a key).
Singular affine placement remains allowed; malformed strides/indices,
non-affine transforms and nonfinite results refuse without partial output.
The aggregate triangle limit is 100,000 and distinct output vertices are bounded
to 300,000. All 69 polygon-core and 68 export/direct-modeling/MCP/packing tests
pass, including touching 3MF objects, nearby distinct vertices and f64 placement.
Vue/MCP typechecks, Vite and 68 distribution artifact checks pass (4,587,145
bytes). Transport copies and browser memory/latency remain unprofiled; shared
qualification below predates this changed WASM. Full B-rep completion remains open.

Shared workflow qualification has been refreshed for the current Rust/WASM
artifact (SHA-256 c70d8a21b465eae6dafebf836fd1d649cbe8ad66cabfa2be01681c3c76cacd11).
The rebuild is byte-identical to the latest scoped residual-bounds checkpoint.
All 122 SVG/vector workflows, 218 own-CAD tests and three engine-manifest tests
pass; their generated evidence now identifies this artifact and current lockfiles.
The SVG native checks, Vue/MCP typechecks, Vite and verification of 68 distribution
artifacts also pass (4,586,940 bytes). Earlier statements below that shared
qualification predates the WASM describe their historical checkpoints. These
workflow checks do not prove complete-domain intersection certificates,
certified curved Booleans, the remaining Rust migration or full B-rep completion.

Ruled parameter admission now encloses residual construction itself, using
trimmed Cartesian coefficients, the normalized plane and outward bounds for
weight normalization. Rounded residuals are no longer treated as exact inputs
to subdivision. A finite in-budget cancellation fixture has zero computed
point residual while its input plane has no point on the surface; both U/V
conversion paths now return `BREP_INTERSECTION_UNRESOLVED`. Exact arithmetic
identities for zero, unit factors and opposite equal operands preserve tight
intervals without tolerance snapping. Consequently the exact dyadic boundary
tangency fixture now resolves, while genuine interval uncertainty still refuses.
All 99 brep-core and 68 integration/packing tests pass, along with Vue/MCP
typechecks, Vite and 68 distribution artifact checks (4,586,940 bytes). Earlier
knot edits and plane-normalization roundoff remain outside these bounds; this
is not a full independent completeness certificate. Shared qualification
predates this WASM; full B-rep completion remains open.

Ruled trace conversion now checks the complete active span's numerical
parameter range before returning a curve. A regression demonstrated that the
previous endpoint-only admission returned a NURBS curve even when the trace
left its source surface in the interior. Opposite-sign Bernstein boundary
residual bounds now admit each ruling interval; uncertain intervals subdivide
with outward arithmetic, capped at depth 32 and 4,096 visited boxes per span.
Proven numerical excursions reject the trace, while exhausted/ambiguous range
admission returns `BREP_INTERSECTION_UNRESOLVED`, including unresolved boundary
tangency. Mixed control signs can still resolve after subdivision. Both
single-curve and segmented U/V paths use this admission. Indeterminate interval
arithmetic now widens to the whole real line instead of yielding a false sign.
All 98 brep-core and 67 integration/packing tests pass, as do Vue/MCP typechecks,
Vite and 68 distribution artifact checks (4,587,425 bytes). Bounds cover the
subdivision of already computed residual coefficients, not prior knot edits,
plane normalization or coefficient construction; this remains numerical
admission, not an independent completeness certificate. Shared qualification
predates this WASM; general B-rep completion remains open.

The native segmented-trace API now handles ruled sections in both U and V.
Each active source knot span is converted independently, ordered in the trace's
direction and mapped onto its original global [0,1] fraction. Cropped intervals,
nonuniform knots and unequal boundary weights are covered. Global endpoints
are canonical 0 and 1, including reverse traversal. A later ambiguous span
rejects the entire call; no partial success or endpoint welding is returned.
Output is bounded to 256 total control points before per-span conversion.
All 95 brep-core tests and 66 integration/packing tests pass, including both
axis directions, oblique plane residuals, late ambiguity and resource refusal.
Vue/MCP typechecks, Vite and 68 distribution artifact checks pass (4,586,020
bytes). The existing single-curve API remains available. This numerical trace
representation is not a whole-domain intersection certificate or permission
to change topology. Shared qualification predates this WASM; full B-rep
completion remains open.

Retained diagonal UV traces now support multiple tensor knot spans in Rust.
`SurfaceTrace::to_curve_segments` and its WASM adapter return ordered rational
pieces whose knot domains use the original [0,1] trace fraction, including
nonuniform crossings and reverse direction. Each piece uses homogeneous tensor
restriction, not fitting. End control coefficients copy the corresponding
tensor corners to avoid a redundant floating multiply/divide. The single-curve
API can join pieces only when Cartesian endpoint values are identical; unequal
numerical endpoints remain independent pieces or a typed single-curve refusal.
No tolerance welding or topology authority is introduced. Degree remains at
most 25 and diagonal output is bounded to 256 total control points. All 95
brep-core tests and 65 integration/packing tests pass, including spherical
re-evaluation, analytic polynomial joins and resource refusal. Vue/MCP
typechecks, Vite and 68 distribution artifact checks pass (4,583,910 bytes).
This extends trace representation, not the general intersection solver or a
coverage certificate. Shared qualification predates this WASM; full B-rep
completion remains open.

The native intersection plane normalization now scales by the largest normal
component before taking its norm. This admits finite equations whose original
norm overflowed and preserves distance-tolerance units for subnormal normals.
Offset normalization reorders division only if the scaled offset overflows;
genuinely unrepresentable normalized offsets remain typed refusals. Native and
WASM metamorphic tests cover sign changes and coefficient scales from the
smallest positive binary64 through the largest finite value. All 93 brep-core
tests and 64 intersection/analytic/Boolean/executor/packing tests pass, as do
Vue/MCP typechecks, Vite and 68 distribution artifact checks (4,580,270 bytes).
This is numerical robustness, not an intersection completeness certificate:
`permitsTopologyChange` remains false. Shared qualification predates this WASM;
general intersections, trimming and curved Boolean completion remain open.

Both stored and compressed 3MF artifacts are now constructed entirely in Rust.
`polygon-core::package_3mf` owns fixed OPC parts, CRC32, local/central ZIP
records, deterministic timestamps and raw DEFLATE through the existing locked
Rust `flate2` backend. Compressed output is bounded while encoding; archive
overhead is reserved before payload admission. The exact 4 MiB boundary is
tested natively. The host only dispatches and copies committed bytes; its ZIP,
CRC and browser-compression implementation has been removed. All 67 native
polygon-core and 47 application/MCP/packing tests pass, including independent
Node CRC/DEFLATE/header checks, 3MF importer roundtrips and oversized expanded
model compression. Vue/MCP typechecks, Vite and all 68 distribution artifact
checks pass (4,580,260 bytes). Browser scheduling/latency remains unqualified;
shared qualification predates this WASM. General B-rep completion remains open.

3MF part admission and model XML serialization now run in Rust
(`polygon-core::model_3mf`). Distinct parts retain separate objects and build
items with their original world coordinates. The actual exported parts share
100,000-triangle/300,000-vertex budgets, closing the former aggregate-preview
budget bypass. Expanded XML is bounded to 64 MiB independently of the final
4 MiB artifact gate, so compressible documents larger than 4 MiB still export.
All 66 polygon-core tests and 45 application/MCP/packing tests pass, including
independent 3MF roundtrips, touching parts, invalid-part refusal and expanded
versus compressed size admission. Vue/MCP typechecks, Vite and verification of
68 distribution artifacts (4,581,246 bytes) pass. OPC ZIP assembly, CRC32 and
compression orchestration still run in TS and remain to migrate. Shared
qualification predates this WASM; full B-rep completion remains unproved.

The ModelGraph/independent mesh export path now admits and serializes ASCII
STL, binary STL, OBJ, PLY, OFF and AMF in Rust (`polygon-core::mesh_export`).
The host receives a committed byte-buffer handle, copies it and frees it in a
finally block. Native admission retains the 100,000-triangle and 4 MiB artifact
limits, rejects inconsistent topology and requires closed positive-volume
printing meshes. Binary STL checks float32 collapse before writing records;
text output preserves finite binary64 coordinates and bounds writes as they
occur. All 64 polygon-core tests and 55 focused application/MCP/packing tests
pass, including independent import roundtrips, copied-buffer lifetime and
oversized text refusal. Vue/MCP typechecks and Vite pass; distribution
verification passes for 68 artifacts (4,581,465 bytes). The 3MF model/XML/ZIP
path is still hosted in TS and needs migration. Shared qualification below
belongs to the preceding checkpoint, not this changed WASM. Full B-rep and
browser resource/performance qualification remain incomplete.

The distribution size gates now pass without increasing budgets or removing
geometry. The Brotli package uses a tagged fixed-width base85 literal decoded
by the Rust bootstrap; legacy base64 packages remain supported. Canonical
length/padding, invalid characters, u32 word overflow, Brotli stream boundaries,
4 MiB compressed/16 MiB output limits and the 64 MiB decoder memory maximum
remain checked. The geometry WASM is byte-identical to the preceding checkpoint
(SHA-256 08c3ee66a1963693cf830c72516403a061e482e4d7d33d2330091d1cff7b0de5).
The kernel chunk is 2,213,228 bytes and the complete distribution 4,582,433 bytes,
saving 143,534 total bytes. All 188 focused application/MCP/packing tests,
three native bootstrap tests, Vue/MCP typechecks and distribution verification
pass. An eight-round Node warm-bootstrap decode comparison measured medians
of 74.4 ms (base85) and 168.4 ms (base64); this is neither browser latency nor
full geometry-kernel initialization. Shared qualification has now been refreshed
against this WASM: 122 SVG/vector workflow tests and 218 own-CAD tests pass,
along with three engine-manifest tests, Vue/MCP typechecks and verification
of all 68 distribution artifacts (4,582,433 bytes). These checks qualify their
scoped workflows; full B-rep completion remains open. Earlier failing size
measurements below are historical checkpoints.

Primary scene STL/OBJ serialization now runs in Rust as well as preparation.
`polygon-core::solid::export_file::Builder` assembles binary STL headers/records
or UTF-8 OBJ with global vertex numbering. Failed appends poison the session,
discard partial bytes and prevent commit. Runtime-local monotonic sessions
admit at most eight active builders and bound their combined serialized payload
to 256 MiB (not total allocator/process memory); native cumulative source
triangle count remains 750,000. The host uploads each mesh, requests commit,
copies the byte artifact and disposes handles in finally blocks. OBJ decoding
is the only text conversion on the host. All 183 focused application/MCP tests,
61 polygon-core tests, one bridge session test, Vue and MCP typechecks pass,
including the expanded 750,000-triangle STL and failed multi-mesh exports.
Vite builds; the geometry chunk is 2,360,764 bytes versus 2,350,000. Other export
paths and the wider B-rep requirements remain incomplete. Browser resource
profiling and refreshed shared qualification are still required.

STL/OBJ mesh preparation now runs in `polygon-core::solid::export_prepare`:
finite affine/index admission, batched vertex placement, reflected winding,
degenerate-triangle filtering, normals and referenced-vertex compaction.
Binary STL rounds coordinates before degeneracy/normal computation; OBJ keeps
f64 coordinates and stable source vertex order. The host formats prepared
arrays into files; serialization itself remains to migrate. Input staging and
prepared f64/u32 views use explicit native release, including refusal paths.
An export-only 64 MiB staging allocator preserves the existing 750,000-triangle
ceiling with fully expanded 54 MB vertex input; that full-size STL case passes.
The general allocator's 32 MiB ceiling is unchanged. All 164 focused application
tests plus 15 MCP integration tests, 58 polygon-core tests, Vue and MCP
typechecks pass. Vite builds; geometry chunk/total sizes are 2,353,252/4,719,295
bytes against 2,350,000/4,700,000 budgets. Other export paths, shared
qualification, browser resource profiling and full B-rep completion remain open.

Code-to-Solid display placement now executes as one native operation per mesh.
`polygon-core::solid::placement` validates affine placement and triangle indices,
bakes all positions in f64, checks the 1e6 coordinate bound and reverses winding
for reflected placements. The raw ABI uploads interleaved source arrays once
and returns f64/u32 views, copied before result release; staged inputs and results
are freed on success/refusal. The placed output is capped at 32 MiB per call.
Authored B-rep authority remains retained and transformed by the existing native
B-rep operation. All 161 focused application/packing/conversion/export tests,
55 polygon-core tests, Vue and MCP typechecks pass. Tests cover offset views,
f64 precision beyond f32 output, reflection, malformed indices, coordinate
bounds, empty meshes and buffer release. Vite builds; the geometry chunk is
2,352,668 bytes versus a 2,350,000 budget. Other point-transform consumers,
document assembly and wider B-rep requirements remain open; full qualification
and browser performance are not claimed.

General matrix inversion, multiplication and transposition now call Rust via
the viewport ABI. Inversion uses the existing native pivoted implementation;
singular or nonfinite/f32-unrepresentable results refuse instead of fabricating
identity. Output writes occur only after a successful native result, including
in-place and overlapping views. Tests verify a small invertible scale formerly
lost to the determinant threshold, inverse residuals, unchanged failure output,
and renderer rejection before replacing a live scene or allocating new mesh
buffers. All 153 focused application/packing/animation/export tests, 12
math-core tests, Vue and MCP typechecks pass. Vite builds; size verification
fails at the geometry chunk (2,351,988 bytes; budget 2,350,000). Remaining host
point/vector transformations need batch migration to avoid per-vertex ABI
overhead; constructors and the wider B-rep requirements remain open. This is
numerical display math, not a certified inversion or full qualification claim.

Camera gesture arithmetic now executes in `math_core::camera_gestures`: yaw
wrapping/pitch clamping, basis-relative pan, wheel scaling and distance limits,
pinch span scaling and centroid pan. `cameraGestures.ts` contains protocol
transport only. The finite-only ABI explicitly encodes nonfinite scalar inputs
for existing wheel/clamp fallback semantics; invalid state updates refuse
instead of publishing nonfinite state. Eighty-one pre-migration gesture states
remain frozen; results agree within 1e-12 times max(1, absolute expected value).
Collapsed touches, nonfinite scalar fallbacks and unchanged rejected inputs
are covered. All 121 focused application/packing tests, 11 math-core tests,
Vue and MCP typechecks pass. Vite builds; distribution verification now stops
at the geometry chunk: 2,350,952 bytes versus 2,350,000. No size budget was
raised. General host matrix utilities, depth ordering, section clipping,
selection identities and the wider B-rep requirements remain open. Browser
gesture latency and shared qualification are not claimed.

Orbit camera frame construction now executes in `math_core::orbit_camera`:
view direction/eye placement, bounds depth projection, near/far fitting,
dolly-to-optical zoom, perspective/orthographic matrices, stable top/bottom
look-at and matrix multiplication. GPU matrices retain the previous f32 storage
at each stage. `orbitCameraProjection.ts` is only an ABI/typed-array adapter.
Seventy-two pre-migration camera frames are frozen independently; native output
matches scalar values within the checked decimal tolerances and GPU matrices
within 2e-7 times max(1, absolute expected value). Existing depth, thin-model,
pan-scale and zoom invariants also pass. All 115 focused application/packing
tests, eight math-core tests and Vue typechecking pass. Vite builds; the size
gate fails at 4,718,249 bytes against 4,700,000. Orbit gestures, general host
matrix utilities, depth ordering, section clipping, selection identities and
the wider B-rep completion requirements remain open. No browser performance or
shared-source qualification claim is made for this checkpoint.

Viewport ray construction (CSS coordinates, matrix inversion, WebGPU depth
unprojection and normalization), world-point projection and barycentric corner
snapping now execute in `math_core::viewport`. The renderer calls the fused
native client-ray operation instead of the TS inverse/unprojection chain, and
uses native projection and transformed-corner selection. Singular matrices
refuse a ray instead of substituting identity; nonfinite/invalid viewport inputs
are rejected. Tests cover perspective/orthographic near planes, independent
forward projection of 40 rays at three depths, CSS offsets, corner ties and
transforms, and public renderer integration. All 103 focused application/packing
tests, six math-core tests and Vue typechecking pass. Vite builds; distribution
verification fails at 4,719,554 bytes (budget 4,700,000). Matrix construction,
scene depth-candidate ordering, section clipping and selection identities still
have host computations. This display-math migration is not certified geometry
or full B-rep completion; browser latency and shared qualification remain open.

Scene broadphase construction, hierarchy traversal, ray/bounds intersection and
candidate ordering now execute in `polygon-core::solid::scene_bvh`. The bridge
retains immutable indexes with runtime-local handles (64 indexes / 100,000
aggregate admitted items); only rays and candidates cross the query ABI.
The host owns upload/disposal and finite-value transport admission. Renderer
publication stages the new index before replacing the scene and releases
retired indexes on replacement and teardown. Tests cover admission rollback,
80 consecutive publications, 500 independent brute-force reference queries,
tiny nonzero directions, inclusive bounds, zero rays and overflow-safe finite
ray parameters. Ninety application/packing tests, 53 polygon-core tests, one
bridge lifecycle test and Vue typechecking pass. Vite builds successfully;
distribution verification fails at 4,717,536 bytes against 4,700,000. Browser
latency, full shared-source qualification and remaining camera/point selection
and B-rep migration are still open. Earlier size/count measurements below are
historical checkpoints.

The legacy TypeScript triangle raycaster has now been removed from production
sources (273 lines of computation and related declarations). Its behavioral
tests execute the shipped Rust/WASM snapshot query instead. The unchanged
256-case independent reference corpus also runs through the WASM ABI: 125 hits,
exact triangle identities/orientations, and absolute error at most 1e-12 for
ray parameters, points, barycentrics and normals. Thirty-eight focused
picking/cache/renderer/selection tests and the Vue typecheck pass. This removes
the duplicate algorithm, not the remaining host scene broadphase, camera and
point-selection computations. The distribution gate and full B-rep
qualification remain open; no new distribution measurement is claimed here.

Renderer initialization now awaits asynchronous geometry-kernel compilation
before allocating its GPU device and reporting readiness. Warmup is shared,
retryable after failure, and never replaces a runtime already owning native
handles; stale renderer initialization stops after warmup. Seventy-six
warmup/picking/renderer/selection tests pass. On one Node run, preparation took
210 ms before readiness, first snapshot upload/build/query then took 16.8 ms,
and warm query median/p95 were 0.033/0.054 ms for 20,000 triangles. Startup cost
has been moved, not eliminated; first-snapshot work and browser latency remain
to qualify. Distribution verification still fails at 4,707,140 bytes.

The renderer now uses Rust picking for both initial hits and same-depth
continuations. `NativePickingCache` uploads an immutable snapshot lazily, keys
it by the shared GPU vertex buffer, bounds its LRU retention, and disposes it
with GPU geometry teardown. At that checkpoint the former TS raycaster had no
production caller and remained a migration reference (since removed above).
Seventy-three picking/cache/renderer/selection
tests pass, including repeated queries, shared-buffer reuse and teardown.
On one Node run with 20,000 triangles and 300 matching queries, native warm
median/p95 were 0.036/0.053 ms versus host-reference 0.011/0.023 ms. Cold native
initialization/upload took 228 ms: warmup and browser interaction qualification
remain required, as do large-scene/cache-pressure checks. Distribution is
4,706,876 bytes, still 6,876 over budget. The following paragraphs describe
earlier migration checkpoints rather than the current renderer route.

Picking snapshot upload now has a raw `abi_picking_create` entry point and a
thin `createPickingSnapshotInKernel` typed-buffer binding. It uploads source
subarray byte ranges once, frees temporary linear-memory buffers, and returns
a runtime-local immutable snapshot handle. Thirteen WASM/BVH tests pass,
including nonzero view offsets, empty arrays, and mutation of host arrays after
upload. The Vue typecheck and three native registry tests pass. Renderer
integration remains open; its existing GPU-buffer ownership/reuse lifecycle is
the relevant place to attach native snapshot retention and disposal. The new
artifact exceeds distribution budget by 8,827 bytes (4,708,827 total), and
shared qualification has not been refreshed for it.

`mesh_picking` now provides runtime-local native snapshot creation, repeat
queries and disposal over the existing geometry ABI. Each immutable snapshot
owns its vertex/index buffers and a Rust-built BVH; query calls send only ray
parameters and exclusions. The registry allows 64 snapshots and conservatively
charges up to 64 MiB of retained buffer capacity, including power-of-two BVH
reservation and caller Vec capacities. Handles are monotonic within the runtime;
disposed handles fail. Three native lifecycle/protocol tests pass, and the first
WASM boundary plus existing BVH test run passes 12 tests. Renderer integration,
large-upload/raw-buffer transport and interaction performance remain open;
production picking has not switched away from TS yet. Shared qualification
predates this new WASM entry point.
The final capacity-accounted snapshot passes the same 12 WASM/BVH tests;
distribution verification fails at 4,708,675 bytes (8,675 over the limit).
`docs/qualification/brep-native-picking-registry-v1.json` records this incomplete
integration checkpoint, including the distinction between retained capacity
accounting and unqualified total peak memory.

Native picking migration has started in
`crates/polygon-core/src/solid/bvh_query.rs`. The borrowed-buffer Rust query
implements nearest double-sided hits, exact distance ties, exclusions, affine
ray transforms and inverse-transpose normals, with refusal of reachable BVH
cycles. Fifty polygon-core tests pass, including 256 frozen host-reference
queries against tilted layered triangles (125 hits) and explicit tie/invalid
tree cases. Numeric vector comparisons use 1e-12, not universal bitwise parity.
This query is not yet wired into the renderer or WASM ABI: production picking
still runs in `src/services/meshBvh.ts`. Complete native buffer ownership,
lifecycle and interaction/performance checks before removing that TS path;
copying a whole mesh on every pointer query would regress its complexity.

The application native executor no longer imports the legacy TS B-rep backend.
Its shared diagnostic class lives in `brepSemanticErrors.ts`; the compatibility
adapter re-exports it to preserve error identity for existing consumers. This
removes the source dependency without changing geometry or serialized errors.
The Vue typecheck and executor/backend/scene/diagnostic integration tests pass.
Distribution size is unchanged at 4,701,331 bytes and remains over budget.
The existing SVG source fingerprint predates this host-only refactor; the raw
WASM artifact remains unchanged.

Rust now executes full and signed partial revolution of admitted planar regions
containing line/circular Bezier spans, holes and nested islands. The shared
constructor retains rational lateral surfaces, axis poles, cavity-shell
ownership for full turns, and common planar end faces with holed UV trims for
partial turns. Polygon revolution and torus construction use that foundation.
Profile component classification is shared with native prism extrusion and
respects non-unit curve parameter domains.

These operations remain bounded (64 spans and 256 constructed faces) and
numerical rather than certified. Control points must remain in the admitted
nonnegative radial half-plane. Endpoint agreement uses the profile tolerance
without moving the retained source curves; final model validation checks the
shared topological vertices. A horn torus is rejected. This is not general
NURBS revolution or proof of the complete feature/Boolean matrix.

Current native evidence includes 91 core tests: analytic volumes for full and
signed partial toroidal/spherical regions, multiple offset holes, nested islands,
shared poles, cap topology and invalid inputs. The latest feature snapshot has
63 passing application integration tests, including displayed hollow-torus
sectors and a clipped-circle-to-sphere workflow. Immutable qualification JSON
files under `docs/qualification/brep-*-revolution*.json` record exact historical
source/WASM hashes and scoped successes or failures. Those snapshots do not
qualify subsequent edits. The shared cap-frame helper is now covered by refreshed
122 SVG workflow tests plus native SVG/planar suites, 218 own-CAD tests, 69
B-rep integration/packing tests and 3 manifest tests, bound to WASM
`84b4f2ffcee29c0f0913d0b8311f274158dcc59264430a0869563b684651282d`.
The size experiment did not resolve the distribution gate: compiler-default
inlining gives 4,701,331 bytes against the unchanged 4,700,000-byte limit.
Forced out-of-line compilation was larger and was removed. This open size
issue and exact evidence are recorded in
`docs/qualification/brep-region-cap-frame-refresh-v1.json`.

The current tree contains native Rust `nurbs-core`, `brep-topology`, `brep-core`,
and a shared WASM geometry bridge. Indexed oriented topology, rational curves and
surfaces, planar constructors and Boolean operations, exact round construction,
faceted feature construction, serialization, topology IDs, display tessellation,
Solid UI, and ModelGraph NURBS entry points are executable. The current native
tree additionally contains exact sphere/torus, cone and revolve poles, signed
partial revolution, convex planar push/shell/split, numerical mass integration,
and bounded geometry intersection queries. Their documented
envelope is [brep-core/README.md](../../crates/brep-core/README.md).

These are distinct from the permanent `openscad-viewer/brep-1` engine route.
`src/services/geometryBuildEngine.ts` and `src/mcp/createServer.ts` still expose
that route as unavailable/not deployed. The existence of `brep_*` nodes under
`modelgraph/nurbs-1` does not establish a deployed B-rep source contract.

`Model::validate` explicitly reports `geometryAgreement =
sampled_with_tolerance` and `solidGeometryStatus = not_certified`. Retain these
honest labels until their stronger contracts are implemented and verified.

## Requirements and completion evidence

“Partial” means relevant executable work exists, but the full row remains open.
“Open” means the required end-to-end implementation or proving evidence is absent.
In-flight changes must be re-read and their tests inspected before updating a row.

| Plan requirement | Current authoritative evidence | Status and missing completion evidence |
| --- | --- | --- |
| 1. Permanent routing, provenance, same-engine failure/retry, immutable manifests | `src/services/geometryBuildEngine.ts`, `src/mcp/engineManifest.ts`, routing/refusal tests | Partial. Contracts and registry exist; complete with an authentic executable B-rep provider, immutable executable manifest, persisted/exported provenance, refusal, revocation and rollback drills. |
| 2. Versioned executable SemanticProgram, typed DAG, canonical encoding, distinct identities | `src/core/semanticProgram.ts`, `src/services/semanticProgram{Codec,Validator,Executor}.ts`, `brepSemanticBackend.ts`, `brepSemanticScene.ts`, `brepDiagnosticExecutor.ts` | Partial. A real Rust/WASM B-rep backend executes analytic primitives, admitted profiles/Boolean/extrusion and affine transforms, with typed empties, immutable snapshots, per-session leases and an aggregate retained-geometry budget. Scene identities remain distinct. Disposable browser and Node diagnostics provide hard cancellation and checked correlated transport; observed profile fixtures agree between hosts. Complete the supported semantic matrix, production isolation, native Rust SemanticProgram and MCP parity, provenance transport and qualified provider integration. |
| 3. Shared legacy adapter and compatibility seam | `src/services/openscadSemanticLowerer.ts`, qualification supervisors, `docs/qualification/g0-contract-pack-status-v1.json` | Partial. G0 status remains `closed: false`; G0.14 records missing clean evidence across the frozen matrix. Local discovery tests cannot substitute for the declared clean-run matrix. Do not rewrite frozen oracle hashes merely to make current results pass. |
| 4. Reproducible Rust workspace, dependency boundaries and allocation budgets | `crates/Cargo.toml`, `crates/Cargo.lock`, `rust-toolchain.toml`, build scripts | Partial. Native/WASM implementation and locked builds exist. Complete artifact reproducibility, production dependency/provenance evidence, bounded ABI allocation and architecture tests for the actual implementation. |
| 5. Numeric context, certified predicates/constructions, typed ambiguity | `crates/cad-predicates`, `crates/math-core/src/lib.rs`, `crates/brep-core/src/operations.rs`, ADR 0006 and predicate oracle fixtures | Open for production certification. A native-only candidate adds immutable source/tolerance identities, binary64 → interval → bounded exact expansions, opaque distance residual evidence and explicit indeterminate/resource/cancellation outcomes. Version 10 supports retained 2D/3D construction DAGs, finite segment/triangle and triangle/triangle intersections, coplanar contours, exact coordinate ordering and point/triangle feature classification. Separate rational oracles check signs, nested constructions and 172 polygon pairs. Shared topology admits application-owned vertex geometry and codecs; bounded batch recipe export/replay roundtrips exact contour vertices against the admitted source without partial publication. Existing production geometry still uses its earlier arithmetic; general curved constructions, production integration and qualification remain open. |
| 6. Bounded NURBS foundation | `crates/nurbs-core/src/{curve,surface,edit}.rs`, `tests/nurbs{Curve,Surface}.test.ts` | Partial. Evaluation/derivatives, refinement, split/reverse, iso-curves, bounds and surface construction exist. Prove the accepted degree/weight/domain matrix, denominator conditioning, conservative numerical bounds and independent rational/Bernstein checks; periodicity/singularities need explicit contracts. |
| 7. Ownership, oriented topology, immutable snapshots and transactions | `crates/brep-topology/src/lib.rs`, `crates/brep-core/src/lib.rs`, topology IDL fixtures/reference tests | Partial. Indexed incidence validation and an immutable shared-topology snapshot with retained owner admission and checked copy editing exist. Native full-model transactions now cover geometry/identity edits, stale/ABA rejection, rollback and bounded history with real kernel tests. Versioned JSON archives retain complete kernel snapshots and bounded Undo/Redo history with strict decoding and fresh store identities. Production UI/WASM routing is still mutable and not connected to this store. Complete that integration, local and global solid typestates, seam/pole invariants, connected material components and independent graph/solid validation. |
| 8a. Complete CC/CS/analytic SS intersections | `crates/brep-core/src/intersections.rs` and its native tests | Partial numerical query foundation: curve/plane, curve/finite-segment, supported surface/plane traces, curve/affine-surface clipping and finite affine surface/surface pairs retain parameter information and unresolved cases. Coplanar affine area results preserve paired UV polygons and lower-dimensional contacts. Finite-segment and affine-domain coincidences isolate boundary roots and retain source intervals with explicit uncertain bands. Surface sections support rational rulings in U or V, unequal positive endpoint weights, multiple knot spans and continuous shared-boundary ownership; serialized traces run through WASM. Supported traces convert algebraically to rational curves, including bounded multi-span assembly with parameter-preserving weight rescaling and explicit seam/conditioning refusal. Reports explicitly say `NumericallyResolved`, never certified complete, and forbid topology changes. Complete the finite pair/contact matrix, general CC/CS/SS results, parameter correspondences and certified complete-domain coverage. Certify transverse, tangent, endpoint, coincident, closed-loop and empty cases. Queries alone do not prove topology-changing Boolean completion. |
| 8b. Lifted UV trimming and material classification | `crates/brep-core/src/operations.rs`, planar triangulation, rational constructor trims | Partial for constructor/planar trims. General surface imprint, curve arrangement/DCEL, periodic lifts, pole rules, holes and robust cell/material classification remain necessary. Prove coverage and non-overlap independently, including missed-branch mutations. |
| 9a. Canonical primitives | `crates/brep-core/src/{lib,analytic,operations}.rs`, `tests/brepAnalytic.test.ts` | Partial. Exact box/wedge, cylinder/frustum/tube, sphere and ring torus now exist. Native tests cover rational sphere/torus equations, genus, pole incidence, orientation and roundtrip behavior; cones and axis-touching profiles have explicit poles. Complete independent geometric certification, full scale/transform covariance and the supported-parameter boundary matrix. A faceted sphere does not satisfy an analytic sphere requirement. |
| 9b. Extrusion and revolution of ProfileSet | `extrude_polygon_with_holes`, `revolve`, direct modeling and ModelGraph tests | Partial. Concave polygon extrusion with holes, full-turn straight-segment profiles, supported axis contacts and signed partial revolutions with exact caps are implemented. Native tests cover their oriented volumes and pole topology. Complete arbitrary supported profile curves, revolution profile holes, general workplanes and consistent New/Add/Cut behavior, with the complete analytic/topological matrix. |
| 9c. Regularized curved Boolean / SolidSet | `crates/brep-core/src/operations.rs`, `stepped_prism.rs`, `tests/brepSteppedBoolean.test.ts` | Partial: bounded planar CSG plus analytic line/circle layered-prism CSG, arbitrary common-axis rigid coordinates, all four Boolean operations, holes, cavities, blind pockets, different heights, empty and separate results. Serialized stepped results support further Boolean use. General curved faces remain unsupported. Numerical classifications are not aggregate solid certificates. Complete the general intersection/split/UV-cell/classify/stitch/global-audit pipeline and its degeneracy and certificate matrix. No mesh fallback can satisfy this row. |
| 10. Canonicalization, exact sewing, explicit healing | Shared constructor edges; planar Boolean assembly | Open as general operations. Boundary correspondence, orientation repair plans, gap/duplicate cases, displacement and cumulative error ledger, idempotence and cancellation rollback are required. Post-hoc mesh welding is neither B-rep sewing nor authorized geometric healing. |
| 11. Certified shell-aware tessellation | `crates/geometry-bridge/src/brep.rs`, `tests/nurbsTessellation.test.ts`, B-rep mesh/LOD tests | Partial. NURBS display meshes now use a topology-owned shared-edge sample/index registry and verify each face boundary against oriented coedges; unrelated nearby shells retain separate indices. Planar polygon display still uses its existing separate finish path. General lifted UV constraints and separate certified coverage/incidence/positive-area/deviation evidence remain required. Shared detail values and observed watertightness are insufficient for all supported surfaces. |
| 12. Native/WASM execution, cancellation, ABI, scene v6 | `crates/geometry-wasm`, `src/services/geometry/kernel.ts`, protocol v6 fixtures, workers, `brepDiagnosticExecutor.ts` | Partial. Shared synchronous WASM and a disposable diagnostic lane exist; actual synchronous WASM abort/deadline and Node termination-before-publication are tested. Complete production integration, checked leases/chunks/epochs, full fault/OOM/cancel behavior, actual GeometrySceneV2 migration, snapshot/export leases and native/WASM topology/status/evidence parity. |
| 13. Browser/MCP B-rep activation | `src/mcp/createServer.ts`, geometry provider registry, NURBS MCP tests | Open for the permanent B-rep engine. Add the real shared evaluator/provider, per-engine admission/queue/quotas/watchdog/restart, immutable executable manifest and same-program build/export parity. Verify crash/hang/OOM/spoof/stale history/rollback; do not label a fake provider available. |
| 14. Full qualification and release | `docs/qualification/`, artifact tests, CI workflows | Open. Existing schemas and old matrix claims do not qualify newly changed kernels. Freeze and run the actual supported matrix with independent oracles/certificate mutations, exact toolchain/corpus/artifact hashes, resource/performance/security budgets, expected refusals and reproducible release/rollback evidence. |
| N1. Persistent naming across rebuilds | `TopologyIds`, `inherit_topology_ids`, serialized lineage, `src/core/topologyLineage.ts` | Partial. IDs now include complete rational curve definitions, cyclic oriented boundaries, holes and support surfaces; matching is unique across sources. Seven dedicated native regressions cover drilled caps, changed support geometry, curve endpoint collisions, hole/concave/collinear material overlap, nearby-coordinate distinction and reindexing. Geometric signatures/regions are cached per operation. General correspondence is conservatively omitted when unsupported. Complete roles/anchors, parameter rebuilds, generated/deleted lineage, symmetric ambiguity and selection transfer without nearest-face guesses. Array-order independence alone does not prove persistent naming. |
| R1/G6. General NURBS surface intersection | Kernel design §9.3 and master plan R1/G6 | Open. Research matrix and solver/verifier completeness remain necessary. A finite analytic matrix must remain honestly finite; general NURBS support cannot be inferred from rational primitive storage. |
| Later features: analytic fillet/chamfer, shell/offset, loft/sweep, direct face edits | `operations.rs` faceted features, `nurbs-core` surface loft/sweep, Solid tools | Partial. Native convex-planar push-face, inward shell with optional openings and plane split are now implemented and tested against volume/invalid-selection/curved-refusal cases. Complete analytic chain/corner solutions and trims, general shell offset intersections, bounded approximate NURBS offsets with deviation evidence, explicit SectionMatch/FrameLaw and self-intersection handling, generalized transactional face edits and surface replacement. Surface-only loft/sweep and planar faceted fillets do not prove solid feature completion. |
| Mesh-independent analysis | `crates/brep-core/src/analysis.rs` and native mass-property tests | Partial. Surface area, signed volume, centroid, inertia and control-net bounds are integrated from retained surfaces, with finite work budgets and convergence estimates. Box/cylinder/tube and translated holed-profile fixtures match analytic metrics. `converged_estimate` does not claim certified error bounds or global solid validity; complete the supported-domain matrix, independent numerical bounds and degenerate/singular cases. |
| Exchange and persistence | Current JSON B-rep roundtrips and mesh export; kernel design §18 | Partial persistence; STEP/IGES B-rep exchange open. Add canonical versioned snapshots, corruption/forward-version/limit handling, topology/units/placement-preserving exchange and roundtrip geometry/topology evidence. Mesh STL/OBJ export is not B-rep interchange. |

## Dependency order for the remaining work

1. Make constructor topology, validation, identity and shared-edge tessellation
   trustworthy; close concrete correctness defects before building operations on
   their results. Expand canonical primitive/pole/profile coverage in parallel.
2. Implement the numeric evidence foundation and immutable query reports, then
   complete CC/CS/analytic SS pairs and their independent coverage verifier.
3. Build general UV arrangements, classification, exact sewing and global solid
   audit on that foundation. Use them for curved Boolean, including degeneracies
   and empty/multiple outputs.
4. Connect a real B-rep program backend to browser/MCP using the same evaluator
   and source contract, with isolation/cancellation, snapshot and provenance
   semantics. Keep capability maturity honest while collecting the full matrix.
5. Extend common intersection/trim infrastructure into analytic blends,
   shell/offset, curved profile features, loft/sweep and interchange. Execute
   the qualification/release matrix against the final implementations.

Implementation may run concurrently where dependencies are satisfied. No stage
is complete merely because it has a corresponding module name or schema.

## Concrete correctness findings at audit start

- Fixed: endpoint-only edge identities, outer-vertex-only face matching and
  centroid-based planar lineage. Complete rational definitions and all oriented
  loops now contribute to identity; support surfaces cannot be inferred from a
  shared boundary. Actual triangulated planar material regions exclude holes
  and handle concavity/collinear vertices. Evidence:
  `crates/brep-core/tests/topology_identity.rs` (7/7 native regressions), plus
  existing operation identity/roundtrip tests. Unsupported rotated/curved
  correspondence returns no inferred relation; this is not full N1 qualification.
- `validate` samples each edge/pcurve/surface correspondence at nine parameter
  values. This is explicitly sampled evidence and cannot exclude defects
  between samples or certify global embedding/material validity.
- Fixed for the NURBS B-rep path: display positions now belong to authored
  vertices and shared edge samples; face interiors receive their own indices.
  Every face's generated boundary must match its oriented coedge sample chains.
  The separate polygon display path retains its existing proximity weld.
  Seven native registry regressions exercise sub-tolerance separated shells,
  sphere/cone poles/tube holes/partial turns over multiple detail levels,
  touching curved-prism XOR components, curved trims with holes and duplicated
  triangulation bridge endpoints, ambiguous shared UV refusal, and canonical
  empty meshes with `closed = false`, and periodic torus seams with genus and
  triangle-budget checks. This proves these boundary ownership and
  incidence cases, not certified surface deviation or general UV coverage.
- Fixed: a Direct document could retain an empty authoritative B-rep with
  stale nonempty display triangles and later export those phantom triangles.
  The document loader now rejects that inconsistent body entry while allowing
  nonempty open sheets. Six WASM consumer regressions in
  `tests/brepEmptyConsumers.test.ts` also verify empty text display, scene and
  Solid conversion, null bounds, immutable native snapshots, JSON rebuild,
  zero-element OBJ/PLY/OFF export and explicit printing-format refusal.

## Production bundle budget

The checked distribution at audit time contains a 1,363,160-byte shared geometry
JS chunk; `scripts/verify-dist.mjs` limits it to 1,200,000 bytes and the complete
distribution to 3,300,000 bytes. The preceding baseline also exceeded the chunk
limit. Neither raising the limit nor relabeling the chunk proves optimization.

The generated WASM is 3,187,057 bytes. Current level-9 DEFLATE is 1,022,346 bytes
before base64; changing `memLevel` to 7 saves only 292 bytes. Other DEFLATE
strategies tested were larger. This option cannot close the gap.

Concrete opportunities, requiring measurement before adoption:

- The installed `wasm-opt` can optimize a temporary copy (`-Oz`, preserve
  numerical semantics) without changing the Rust toolchain. Compare exports,
  byte size, native/WASM geometry regressions and timing; pin its exact version
  and flags if adopted. Existing build script strips symbols but does not run
  Binaryen.
- Current DEFLATE data is embedded as base64 in JavaScript, adding one third to
  its size. A separate compressed binary asset avoids that expansion, but the
  synchronous loader would need deliberate preloading, Node/browser parity and
  revised artifact-type budgets. This is a packaging change, not a trivial
  string edit.
- Node Brotli quality 11 compresses this WASM to 726,312 bytes (968,416 base64
  bytes). That measurement demonstrates potential, not a ready solution: the
  repository currently owns a synchronous DEFLATE decoder, so changing formats
  requires a supported decoder/loading contract and end-to-end performance,
  memory, licensing and browser checks.
- `polygon-core` deliberately uses optimization level 3 because of a measured
  runtime benefit. Reverting it to the size profile previously saved about
  68 kB packed at a material speed cost. Treat that as a measured tradeoff,
  not an automatic low-risk fix. First examine removable duplicated encoding
  and monomorphization costs and optimizer output.

A follow-up experiment used **only temporary copies** of a later generated
WASM. Installed Binaryen is `wasm-opt version 116`; the command was `wasm-opt
input.wasm -Oz --enable-bulk-memory --enable-nontrapping-float-to-int
--enable-sign-ext -o output.wasm`. These flags enable instructions already
present in the input; no fast-math option was used.

| Artifact | Raw WASM bytes | Level-9 DEFLATE bytes | Base64 payload including size prefix |
| --- | ---: | ---: | ---: |
| Input | 3,249,517 | 1,041,382 | 1,388,516 |
| Optimized temporary copy | 2,893,980 | 1,020,081 | 1,360,116 |

Input SHA-256:
`626b0a82fbf2d20bc557f564398856b1b7a67c1c7ef3b761f5cd3e4b7b2df430`.
Output SHA-256:
`e7b2f5f56cd6971f3791df834489cf67a2484fff61140498caf5e99bd2134376`.
Both expose the same 11 exports and zero imports. Seventy ABI response-byte
comparisons were identical: 64 successful constructor/inspect/tessellation/mass/
Boolean/blend/text requests, three invalid-size refusals, and three refusals for
push/split/shell operations not yet included in this captured input artifact.
The latter are not evidence for the new operations. One smoke run accumulated
907 ms original versus 945 ms optimized; this is not a performance qualification.

The optimized base64 payload saves only 28,400 bytes (about 2%) and remains above
the 1,200,000-byte chunk limit before wrapper overhead. No build script, decoder,
artifact, or budget was changed by this experiment. This optimization alone
cannot close the production size gate.

The subsequent implementation uses a separate synchronous Brotli bootstrap in
`crates/wasm-brotli`, preserving `geometry/kernel.ts`'s synchronous API. It pins
the offline-available `brotli-decompressor` 5.0.3 and its two allocator crates;
these are compression libraries, not foreign geometry kernels. The default
decoder's safe implementation is retained, and complete redistribution notices
are shipped in `public/third-party/brotli.txt`. The repository-owned DEFLATE
decoder still loads the small bootstrap and serves its other existing callers.

The wrapper accepts at most 4 MiB compressed input and 16 MiB declared output.
The decoder uses a fixed output slice, requires stream completion and exact
input/output consumption, and has a 64 MiB WASM memory maximum. Each call copies
the decoded result before releasing its temporary decoder instance. Two native
tests and six WASM tests cover boundary limits, trailing/truncated/concatenated
streams, exact decoded geometry/photogrammetry artifacts, reference-generated
compression corpora and ownership of returned buffers. Five existing
photogrammetry tests also pass after its lossless packaging change.

A temporary production build after geometry Brotli integration measured
1,122,724 bytes for the geometry chunk, 123,840 for the separate decoder chunk
and 3,531,144 bytes overall. Thus the geometry cap passed while the complete
3,300,000-byte distribution cap still failed at that point. Photogrammetry's
subsequent Brotli payload is 241,516 base64 bytes versus 313,296 before, saving
71,780 bytes. The final combined temporary Vite build, also using the same
decoder for HarfBuzz, measures **3,241,309 bytes** overall: geometry chunk
1,122,440, decoder chunk 123,184, photogrammetry chunk 241,546 and HarfBuzz chunk
179,586. The license asset is included. This is 58,691 bytes below the unchanged
total budget; final release verification must validate the actual packed WASM
artifacts in the production output as well as these byte counts.

The native browser `DecompressionStream` is a stream API, so replacing the
synchronous loader with it would require an explicit asynchronous initialization
contract or preload step. The separate synchronous module avoids that API
change. See the [Compression Standard](https://compression.spec.whatwg.org/) and
the [decoder's upstream source](https://github.com/dropbox/rust-brotli-decompressor).

## Final completion audit

Before calling the goal achieved, replace every open/partial row with inspected
implementation evidence and a passing test/certificate whose scope matches the
row. Re-run the repository's build/type/test gates; resolve or explicitly track
baseline failures without laundering qualification artifacts. Verify UI and MCP
behavior against real runtime results. Audit the actual final capability
manifest, limits, snapshots, exports, size/performance budgets and rollback
evidence. Missing, indirect or uncertain evidence leaves the goal active.

## Earlier verification checkpoint (before prismatic integration, 2026-09-12)

- Native B-rep/topology/NURBS/bridge suite: 80 passing tests, including identity,
  exact poles, partial-turn cap seams, narrow knots, extreme rational weights,
  shell thickness and mass-property regressions. The final normalization change
  additionally passed all 40 core unit tests and the WASM split smoke check at
  plane coefficient scales 1e-300 and 1e300 (volumes 400 and 600 mm³).
- Full Vitest run: 2351 passed, 12 failed, 5 skipped. Six failures were the
  previously observed oracle/manifest/error-type/engine-identity assertions;
  the other six were timeouts while native compilation competed for CPU.
  Rechecking those files and B-rep/Solid tests with one worker passed 271 tests.
  A subsequent affine B-rep reflection/ModelGraph run passed 17 tests.
- Final type checks passed. Production compilation succeeded; `verify-dist`
  still refuses the 1,459,552-byte geometry chunk against its 1,200,000-byte
  limit. No size allowance or qualification fingerprint was silently relaxed.
- Three transient failures in concurrently edited planar-geometry curve Boolean
  tests were no longer reproducible on the fresh tree: its 127 tests passed.
- Browser Solid check: exact sphere radius10 reports numerical volume
  4188.7902 mm³ and area1256.6371 mm² through the authored-surface analysis.

These observations do not close the full goal. The next dependency chain is
certified curved intersections, parameter-domain trim arrangements, sewing and
solid classification, then curved Boolean publication. Analytic blends,
remaining feature families, qualified brep-1 integration and distribution gates
remain open as recorded above.


## Prismatic curved Boolean and regularized empty integration

The native path now retains rational line/circle spans through analytic planar
arrangement and `prism::extrude`; it does not reconstruct geometry from meshes.
The supported common-axis family includes holes, disconnected results, all four
Boolean truth functions, arbitrary axial overlap for intersection, through cuts,
and interval algebra for equal/covering footprints. Equal-height profiles can
be transformed to arbitrary rigid orientations; coordinate corrections are
bounded and recorded by `prism_frame`, not claimed as exact-arithmetic proof.
The later bounded layered-cell implementation adds stepped side arrangements;
sphere/torus/general NURBS intersections remain open. Numeric analytic
classification is not a global solid certificate.

The audit found and fixed endpoint-encoded crossing contours, translation-driven
carrier coincidence, duplicate geometry IDs at independent component contacts,
and malformed legacy identity reconstruction that could index invalid topology.
Source curves remain intact during the local planar calculations; unresolved
parameter accuracy returns an explicit error. Colliding entity IDs use owning
shell geometry, preserving reindexing invariance without merging nearby objects.

Empty results now survive serialization, native analysis refusal, tessellation,
polygon conversion, ModelGraph snapshots and later Boolean operations. Solid
removes an emptied body atomically and Undo restores the inputs. Canonical empty
B-reps cannot be loaded with a stale nonempty display mesh. Nonempty open sheets
remain valid display entries; empty scenes produce neither phantom entities nor
an open-surface warning.

Verification at this integration point:

- 197 native tests passed across brep-core, brep-topology, geometry-bridge,
  nurbs-core, modelgraph-text and modelgraph-runtime, including adversarial mass
  integration and the new interval, pose, identity/decode and registry tests.
  Log: `/private/tmp/brep-native-expanded-final.log`.
- 66 B-rep/Solid/ModelGraph/WASM tests passed in 8 files after Brotli geometry
  startup integration. Log: `/private/tmp/brep-compressed-integration.log`.
- Browser Code mode built the supplied rational cylinder subtraction example
  into one visible mesh (3260 display triangles at detail16), with mesh export
  controls enabled. The displayed area/volume are mesh estimates.

The initial full production build passed: 63 artifacts, 3,241,309 bytes, with
all numerical budgets unchanged and emitted geometry/HarfBuzz byte identities
verified. A subsequent build of the stepped/profile artifact and current TS
sources passed the stronger four-runtime verifier: 65 artifacts, 3,254,080
bytes; geometry chunk 1,151,620 bytes. This latter Vite/verification run used
the generated WASM from 21:13:02, before concurrent SVG dependency edits.
It is not evidence for a later, still-changing native dependency closure.

## Stepped profiles and real semantic execution checkpoint

`stepped_prism` partitions retained line/circular-arc profiles and axial slabs,
cancels complete opposite cell faces and sews only their corresponding edges.
Cap-region differences independently check that no internal faces remain.
The structural recognizer verifies complete ruled side definitions, caps and
declared outer/inner shell orientation; serialization does not rely on hidden
construction metadata. Native tests cover arbitrary common axes and reflections,
holes, cavities, cap contacts, repeated operations and explicit resource limits.

`brep_extrude_curves` is now public in ModelGraph JSON and text. Nested references
to 2D NURBS curves retain their definitions through extrusion, including holes,
full circles and rational line segments. Degree/shape/units/reference/active-span
limits remain checked by the native path. No sampled polygon reconstructs these
contours. The separate `brep_profile_*` bridge exposes validation, even-odd
orientation, signed area and regularized profile Boolean for the semantic backend.

The new semantic backend supports analytic primitives, rectangle/circle/polygon
profiles, supported profile/solid Boolean, XY-preserving transforms and straight
untwisted extrusion without taper. Unsupported ellipses, general surfaces,
projection, twist and taper remain explicit errors. Source `$fn/$fa/$fs` intents
do not alter native geometry. The internal scene adapter takes an explicit
uniform patch-sampling policy with its own hash and does not claim enforcement
of source chord/angle tolerances or certified deviation. It is not yet registered
as the permanent production `brep-1` provider.

Concrete fixes found by independent execution/review:

- 3D `difference()` lowering now returns SolidSet rather than inheriting its
  single-solid base type; this permits legitimate empty/disconnected outcomes.
- Native selections use semantic entity identity rather than a DAG array index,
  preventing equal geometry in an unrelated occurrence from accepting stale IDs.
- Cancellation is checked again after asynchronous result-lease disposal,
  before publishing a scene. All failure paths release native payload ownership.
- Float32 conversion refuses collapsed/reversed triangles and newly merged
  distinct native vertices. Existing exact contacts retain native topology;
  mesh position welding does not determine native shell incidence.
- Retained semantic geometry and aggregate scene snapshots each have a bounded
  4 MiB character budget; scene triangles are capped at 20,000 in total.

Verification of the 21:13:02 geometry artifact:

- 15 WASM tests passed for stepped Boolean and the scene adapter (including
  closed shared-edge tessellation at multiple details, exact-volume comparison,
  sealed cavities, rotated reuse, Float32 failures and cancellation).
- 23 ModelGraph tests passed, including five new rational-curve extrusion cases;
  18 semantic backend tests and four profile bridge tests passed.
- The two affected semantic/lowering oracle suites plus initial scene tests
  passed 174 tests. Two stale ordinary test expectations were corrected to the
  actual own-Rust mesh identity and non-finite geometry error type; 17 tests pass.
- A real browser/Node fixture produced identical source/program/display hashes,
  entity IDs, native snapshot revisions, colors, face counts, triangle counts
  and display volume for an extruded annulus and a separate cube. This verifies
  that fixture through the shared WASM; it does not qualify native Rust or MCP.
- The broad run before these final additions passed 2,378 tests, with five
  frozen oracle/manifest/qualification failures remaining after the two ordinary
  expectation fixes. Seven HTTP tests initially hit sandbox port restrictions;
  escalated reruns reached the server but showed intermittent timeout/refusal
  under concurrent builds, so their final clean rerun remains required.

Current exact generated WASM SHA-256 at this checkpoint:
`36c414ecb8ee29fbd77e13571b5990b596d1bec31141bfd20347b778b9230b7a`
(3,758,472 bytes). This is candidate execution evidence, not a replacement for
frozen qualification manifests. Full completion remains unproved.

## Joint source-tree build and usable enclosure checkpoint

The later complete offline locked build compiled the newly added SVG runtime,
font and raster dependencies. Its original size gate failed: the geometry chunk
grew to 2,178,412 bytes against 1,200,000. A concurrent SVG change then explicitly
raised that gate to 2,300,000 and the distribution gate to 4,700,000. Those edits
are preserved. The resulting current distribution verifier passes **66 artifacts,
4,501,737 bytes**, including all four packed-runtime byte identities. This is
compliance with the expanded SVG budgets, not evidence of meeting the earlier
1,200,000/3,300,000 limits. Splitting SVG into another module alone would not
remove its bytes from the total distribution.

The production browser build displays the stepped enclosure as one mesh with
3,124 triangles, export and Code-to-Solid controls enabled. A warm full rebuild
took 120.7 ms in one observed run; this is not a performance qualification.
The display volume is 7,559.37 mm³ at detail 6; the independently checked native
B-rep volume is 7,619.809938 mm³. The difference is expected tessellation error,
not an exact-geometry change. The checked source is now included in the browser
and MCP example catalog, with a source equality assertion in its existing native
geometry/export integration test.

The latest native no-fail-fast regression run covered ten packages and 35
test/doc binaries: 447 passed, one existing ignored and one stale ordinary
unsupported-capability assertion. That assertion expected all round solids to
refuse a box union; cylinder/box union is now supported. Its replacement checks
one closed body and independent volume 45π+4 while retaining frustum and tangent
tube refusals; its focused rerun passes. A fresh complete run remains pending.

That fresh native run subsequently completed successfully: **449 passed, zero
failed or ignored**, across the same ten packages and 35 binaries. The current
full JavaScript run passed **2,490 tests**, with five skipped and four failures:
two frozen direct-evaluator mesh-byte oracle cases (`colored-transform` and
`boolean-difference`), G0 toolchain fingerprints and v18 qualification bindings.
The seven HTTP tests pass with their original timeouts in an isolated run;
temporary stage timings confirmed CPU contention during earlier failures.
No frozen expected hash was replaced to make these checks pass.

## Diagnostic isolation and numeric oracle repair

The [isolated semantic diagnostic lane](brep-semantic-diagnostic.md) now runs
real lowering and B-rep execution in disposable browser and Node workers.
Host cancellation stops synchronous WASM; Node publication waits for worker
termination, and a failed join quarantines the lane. Parent validation checks
the normalized program, source/policy/artifact correlation, output identities,
native snapshots and face lists, recomputed display asset IDs, baked transforms
and program-derived provenance. Tiny views cannot hide oversized backing buffers
from transport accounting. This lane remains explicitly diagnostic and cannot
provide production admission or independent geometry correctness evidence.

Ten lifecycle/protocol tests pass, including a real infinite synchronous WASM
fixture for abort/deadline, actual native refusal/recovery and adversarial result
mutations. Independent review and a separate run pass the same ten tests. Browser
and Node execution of an extruded annulus agree on the complete attestation,
packed-kernel digest, display policy, native snapshot and display asset identity.
Browser abort followed by cube(2) yields two terminated workers and volume 8.
The command-line diagnostic also builds the included semantic enclosure into
3,124 triangles with a disposed worker. V8 heap limits do not bound WASM memory
or process RSS; browser termination has no Node-style join acknowledgement.

The independent test-only numeric oracle had an actual underflow defect:
`orient2d(0,0,2^-600,0,0,2^-600)` reported filtered Zero despite its strictly
positive exact rational determinant. Each interval operation now rounds both
endpoints outward, including subnormal/overflow cases; unresolved intervals
never prove Zero. Exact BigRational distance comparison and interval 3D/distance
filters extend the finite inventory coverage. Thirteen oracle tests and two
artifact-presence tests pass. No production predicate, existing Boolean decision
or frozen qualification artifact is changed by this repair. A separate bounded
native predicate foundation was subsequently implemented as described below;
neither change closes a qualification gate.

The complete locked production build at this checkpoint passes with **66
artifacts / 4,510,944 bytes**, under the explicitly expanded SVG budgets noted
above. The raw shared WASM is 6,419,018 bytes with SHA-256
`99dc99c156ececc0f50014a147a83de4ec533b9a15da4ec9092bfcebc7b50ea5`.
The built gallery visibly lists the new enclosure and loads its actual public
source. These checks precede the isolated native predicate crate work; they are
not its build or qualification evidence.

## Code-to-Solid persistence and native predicate candidate

An actual production browser check exposed a lost-authority bug: Code-to-Solid
copied the display mesh but discarded its native B-rep carrier. Conversion now
validates the native snapshot, preserves all authored binary64 coordinates and
rational weights for identity placement, and applies nonidentity affine scene
placement to both the B-rep and display mesh. Reflection reverses display winding.
The complete Direct document is validated before publishing the mode change;
invalid or inconsistent empty carriers cannot leave a half-converted document.

Five focused regressions cover the real enclosure, reflected nonuniform placement,
non-symmetric rotation using the renderer's row-major convention, projective and
singular refusals, corrupt snapshots and stale empty displays, and π/e authored
dimensions surviving Float32 display conversion. Together with empty-consumer
checks, **11 tests pass**; Vue and MCP type checking and the production build pass.
In the production UI, the
enclosure retains B-rep controls and reports native V = 7619.8099 mm³ and
A = 3943.7429 mm². These are numerical surface integrals, not certified error
enclosures. The latest distribution verifies **66 artifacts / 4,512,502 bytes** under the expanded SVG
budgets. This frontend correction does not change the raw shared WASM above.

Independent review also exposed two Solid UI defects: retessellation edited a
history clone but committed a different fresh clone, and an async first mount
could miss a seed already supplied by Code-to-Solid. Retessellation now commits
the edited document. The seed is validated and applied once on first mount/open,
preserves Undo to the previous document, and cannot replay over later edits when
the workspace reopens. Twenty-one actual Vue-component tests pass, including
three new regressions for changed triangle counts, retained native surfaces,
Undo, cold mounting, deferred opening and subsequent independent seed imports.
Merely observing unchanged mass properties after a button click did not prove
that display retessellation occurred; the new mesh/history assertions do.
The final production browser check independently counted 3,124 rendered triangles
before retessellation and 5,388 at detail 8. Undo restored 3,124; Redo restored
5,388. Surface-derived volume and area remained 7619.8099 and 3943.7429 respectively.

The separate dependency-free `cad-predicates` crate implements `orient2d`,
`orient3d` and `compare_squared_distance`. Its authored-input arena admits finite
binary64 bit patterns and canonical i64/u64 rational constants. Private references
and immutable tolerance identities prevent accidental cross-context reuse;
they do not authenticate an arbitrary caller's claim that rounded data was authored.
There is no constructed-point admission API or geometric model importer.

Fast forward-error bounds and outward intervals escalate ambiguous signs to
bounded floating expansions, using independently written error-free sum/product
algorithms from the allowlisted mathematical paper. Exact zero is issued only
by the exact stage. Finite exponent limits can still refuse, as can the fixed
one-million work cap, 4096-term cap, deadline or cancellation. Opaque context-bound
distance residuals require an actual outward recipe and respect `max_entity_error`;
missing proof and tolerance gray bands remain indeterminate.

Native debug and release each pass **17 tests**. The independent BigRational
corpus contains **173 cases: 172 correct decided signs, 75 exact-expansion
decisions and one explicit exponent-range refusal**. Resource tests include
oversized u64 budgets and priority-preserving cancellation/deadline refusals.
The corpus generator is reproducible and its TypeScript checks pass. The crate
is not a geometry dependency and is not compiled into the shared WASM. Connecting
trusted source leaves, constructed-point recipes and topology decisions remains
required; current Boolean operations gain no certification from these tests.

## Attribution of remaining frozen legacy failures

A read-only historical reconstruction exactly restored both failing frozen
legacy fixture hashes. Commit `4e46a9f` changed cube construction from rectangle
extrusion to a direct eight-vertex cube; `383f9f9` changed the vector norm from
chained hypot to sqrt(dot). Both precede this B-rep work. Applying those changes
separately and together to the historical native probe reproduced the current
hashes exactly, including passage through today's scene/BVH adapter.

For those two concrete fixtures, independent bounds, volume, area, orientation
and edge-incidence checks preserve the shape and closed shells. Twelve actual
BVH probes preserve hit positions and outward normals. However, numeric IDs of
the -X, +X and +Y cube faces changed (3→5, 4→3 and 5→4 respectively), so saved
cross-version selections cannot assume compatibility. This is not general
kernel equivalence or a reason to replace frozen expected bytes.

The G0 test stops at its first assertion, but the complete artifact comparison
finds four drifts: package.json, THIRD_PARTY_NOTICES.md, crates/Cargo.toml and
crates/Cargo.lock. The package lock, .npmrc and Rust toolchain file still match;
part of Cargo.lock drift already existed in HEAD. Frozen fixtures and manifests
remain unchanged by this audit. Exact byte parity and production qualification
remain **failed**, with a future versioned qualification cycle still required.

## Exact line construction recipes — 2026-09-13

The native predicate candidate now retains a first actual constructed-point
recipe. `intersect_authored_lines2d` uses exact homogeneous cross products over
authored binary64/rational leaves, checks both source-line incidences and a
nonzero homogeneous weight, and returns an opaque unique point or explicit
parallel/coincident/degenerate/indeterminate classification. Coordinate and
source-line parameter intervals are rounded outward; the positional enclosure
diameter must fit the immutable entity-error cap. These are infinite lines,
so parameters outside [0,1] are retained and no segment-intersection claim is made.

`orient2d_points` accepts the resulting recipe beside authored points. A strict
sign can use its issued enclosure; an ambiguous sign re-evaluates the exact
construction and homogeneous determinant. The intersection at x=1/3 is therefore
on its source line while remaining strictly distinct from authored binary64
1.0/3.0. Neither a rounded center nor a caller-supplied enclosure can mint an
accepted constructed point. Source/tolerance identity and work/deadline/cancel
controls apply again to each subsequent predicate.

Six public-API regression tests add a separate bounded i128 rational oracle using
affine Cramer's rule and exact comparison against binary64 enclosure endpoints.
The 200-case matrix has 168 unique intersections, eight parallel, eight coincident
and 16 degenerate inputs. Additional cases cover endpoint order, line order,
reflection/shear/translation, orientation permutations involving three constructed
points, exact 1/3 incidence, near parallelism and error/resource/context refusals.
This is separately implemented arithmetic evidence, not an independent reviewer
or a completed construction qualification. Candidate implementation identity is
version 2; frozen inventories, oracles, manifests and admission gates are unchanged.

Final native checks pass in debug and release: **23 tests each** (seven predicate
unit tests, ten existing independent-oracle tests and six construction tests).
The 173-case predicate corpus keeps its earlier 172 correct decisions and one
explicit exponent refusal. Clippy over all crate targets passes with warnings
denied. `cargo tree -i cad-predicates` confirms there is still no runtime consumer;
no frontend/WASM behavior or production qualification is inferred from these checks.

The remaining scope is not reduced: recursive construction DAGs, constructed
line endpoints, 3D plane constructions, general CC/CS/SS roots, complete evidence
composition, production geometry integration and qualification remain required.
The new crate still has no B-rep/WASM consumer, so existing Boolean operations
remain numerical and unqualified by this change.

## Shared recursive 2D constructions — 2026-09-13

The candidate now accepts its own constructed points as line endpoints through
`intersect_lines2d`. Results own shared immutable recipe nodes with their source
dependencies. No rounded coordinate is re-admitted, intermediate handles can be
dropped, and exact orientation can re-evaluate the whole dependency graph.
Limits are 32 levels and 256 unique nodes per admitted root/combined query;
source/context validation precedes cancellation shortcuts. Query-local memoization
prevents repeated expansion of shared ancestors, and its retained expansion cache
is capped separately. Graph/resource refusals do not mutate input points.

A shared diamond initially exposed unnecessary common-factor growth: a simple
point on y=3x exhausted finite expansion products at exponents -709 and -716.
Lossless positive-GCD reduction of checked 120-bit-span homogeneous coefficients
now removes those factors before power-of-two normalization. Larger spans keep
the earlier exact expansion path. This changes representation, not the exact
geometric point, and never reconstructs a rational from a rounded center.

Six new graph regressions check 32-level ownership and depth refusal, shared
25-node/depth-17 evaluation, the exact 256/257-node boundary, combined oversized
roots, independent equal constructed endpoints, context/cancel/work refusal,
100 general nested intersections against the separate affine rational oracle,
and the unreduced 200-bit-span path. The shared diamond's exact orientation takes
18,867 declared work units in the observed run; that is not a CPU-time or general
performance qualification. Debug formatting is bounded rather than recursively
printing the shared graph.

Implementation identity is candidate version 3. General 3D constructions, curve
roots, complete evidence composition, production geometry consumers and the
qualification matrix remain open; this finite 2D graph does not close them.

Final debug and release checks each pass **29 tests**: seven unit, ten existing
independent predicate-oracle and twelve construction/graph tests. The original
173-case predicate outcomes remain unchanged. Clippy over all targets passes
with warnings denied. No production WASM, routing or qualification gate is
activated by this native-only candidate work.

### Candidate 4: retained 3D line/plane constructions

Added exact homogeneous line/plane intersection recipes with authored or
constructed line and plane inputs, outward point and parameter enclosures, and
recipe-aware `orient3d_points`. Rational affine regression checks cover more
than 200 fixtures, plane permutations, exact incidence, a rounded 1/3
counterexample, depth-32 nested constructions and typed degeneracies. Shared
projective normalization now supports both 2D and 3D coordinates.

This is a native numerical foundation only. It does not close general curved
intersection, trimming, Boolean topology, runtime integration or qualification
gates. Full B-rep completion remains open.

Candidate 4 validation: **32 tests pass in debug and release**; Clippy with
`--all-targets -- -D warnings` passes. No runtime or WASM integration is claimed.

### Candidate 5: exact segment/triangle domain classification

Unique retained 3D intersections now support exact location against their
original closed segment and triangle. Explicit endpoint, vertex and edge labels
come from rational parameter signs, including one-ULP boundary separation.
This advances the finite-intersection foundation without changing the runtime
or claiming coplanar overlap, arbitrary trim classification or a closed
qualification gate. General B-rep completion remains open.

Candidate 5 validation: **34 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

### Candidate 6: coplanar segment/triangle clipping

Exact barycentric clipping now handles coplanar segment/triangle overlap, with
distinct empty, singleton and nonzero parameter intervals. Input recipes and
context are retained. An affine boundary-enumeration oracle checks transformed
fixtures, including edge overlaps and vertex contacts; nested depth-32 input
recipes are covered. Noncoplanar and degenerate cases remain explicitly typed.
This closes this bounded query gap only: materialized overlap endpoint recipes,
general trim arrangements, solid topology and runtime integration remain open.

Candidate 6 validation: **36 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

### Candidate 7: reusable exact overlap endpoints

Coplanar overlap bounds now materialize as retained `ConstructedPoint3` recipes.
The graph evaluator replays clipping and homogeneous interpolation and accepts
both recipe kinds in subsequent operations. Publication shares the coordinate
and parameter enclosure checks. The transformed affine oracle checks endpoint
coordinates and exact plane incidence; depth-32 chains retain 4/3 without
rounding, and singleton bounds yield an exactly degenerate line.

The bounded endpoint construction item from candidate 6 is implemented. This
does not establish general trim topology, runtime integration or full B-rep
completion. Qualification gates remain open.

Candidate 7 validation: **38 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

### Candidate 8: unified finite segment/triangle query

A single query now composes exact transverse and coplanar cases into empty,
point or nonzero segment results with retained constructions and feature
locations. Both endpoints must finish within the shared budget and combined
graph-node cap before an overlap is published. Regression checks cover boundary
cases, input reversal, explicit degeneracy, cancellation/provenance precedence
and failure partway through the aggregate work.

This provides a bounded native query entry point. General curved intersections,
trim topology, runtime integration and independent qualification remain open.

Candidate 8 validation: **40 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes.

### Exact 3D coordinate ordering

`compare_points3d` compares authored and retained constructed points
lexicographically with exact homogeneous cross-products. Exact equality
does not depend on recipe identity or rounded coordinate centers. This supplies
a numerical primitive for future intersection deduplication, without assigning
persistent topology identity or tolerance-based coincidence. Admission, graph
limits and cancellation apply even when comparing the same handle. Regression
checks distinguish exact 1/3 from binary64 rounding, independent equivalent
recipes, later-coordinate ordering and context/cancellation precedence.

Coordinate-ordering validation: **41 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes. Runtime integration remains open.

### Noncoplanar triangle pair intersection

`intersect_triangles3d` assembles the six exact edge/triangle queries, removes
algebraically equal endpoints and returns empty, a point, or the common segment
in exact lexicographic order. Returned endpoints retain their constructions.
Coplanar triangles and degenerate inputs are explicit separate results;
coplanar polygon overlap is not yet assembled. Query work is shared and the
combined output graph is checked before publication.

Regression checks cover transverse overlap, vertex contact, separated planes,
coplanarity, degeneracy, triangle exchange and winding reversal. The native
suite passes **42 tests in debug and release**, and Clippy with all targets and
`-D warnings` passes. This bounded query does not close general trim topology,
curved intersections, runtime integration or full B-rep qualification.

### Candidate 9: coplanar triangle intersection contours

`intersect_triangles3d` now assembles coplanar area intersections into an
implicitly closed convex contour of three to six retained vertices. Exact
projected homogeneous orientation builds the hull and removes redundant
collinear points. The first nonsingular coordinate pair defines positive
(CCW) winding; the first vertex is the exact lexicographic minimum. The result
reports the projection axes and does not repeat the closing vertex.

Point and edge contacts still reduce to point/segment results. Checks cover
a six-vertex overlap in XY, XZ, YZ and a sheared reflected plane, operand swaps,
winding reversal, exact vertex coordinates/incidence and strict convexity.
This replaces the earlier explicit coplanar deferral for triangle pairs.
Arbitrary curved trims, persistent topology and runtime integration remain open.

Candidate 9 validation: **44 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes. Full B-rep completion remains open.

### Separate rational polygon oracle

A fixed-seed matrix now compares 172 nondegenerate triangle pairs (180 attempts,
eight degenerate pairs excluded) against separate rational sequential polygon
clipping. This oracle does not use the production edge-enumeration/convex-hull
path. Checks cover output cardinality and every exact rational vertex enclosure,
including more than 100 fractional coordinates, over 80 area overlaps and
multiple empty intersections. It is regression evidence, not independent
qualification or proof of complete B-rep behavior.

The full candidate suite passes **45 tests in debug and release**; Clippy
`--all-targets -- -D warnings` passes. No runtime behavior changed in this step.

### Exact point location on an arbitrary triangle

`classify_point_triangle3d` accepts authored or retained constructed points and
an arbitrary oriented triangle, with exact vertex/edge/interior/outside
classification. Off-plane results retain their exact side sign; degenerate
triangles and resource failures stay explicit. Modelling tolerance does not
snap a nearby point to the plane. The original recipe-domain classifier shares
the same feature-sign classification helper.

Checks cover all triangle features and positive/negative 1e-12 plane offsets.
Every output vertex in the 172-pair rational polygon matrix is additionally
classified on both original triangles. **46 tests pass in debug and release**;
Clippy `--all-targets -- -D warnings` passes. B-rep topology and runtime
integration remain unfinished.

### Shared topology retains exact vertex geometry

The shared indexed model now has a defaulted vertex-geometry parameter. Its
owner-supplied vertex validator runs alongside existing incidence checks, while
default binary64 validation and codec behavior remain intact. A native test
uses actual retained predicate constructions as vertices of a six-edge open
face, checks exact plane membership, clone retention and rejection of corrupted
incidence. Candidate predicates enter this crate only as a dev-dependency.

This removes the mandatory binary64 conversion at this topology boundary;
production source admission, recipe persistence, solid transactions, geometric
edge/face correspondence and qualified runtime integration remain open.

Validation for generic vertex storage: 106 tests pass across brep-core and
brep-topology in debug; five topology tests pass in release; the entire Rust
workspace passes `cargo check --offline --locked --workspace`. Clippy for
brep-topology with `--no-deps --all-targets -- -D warnings` passes. The broader
Clippy invocation fails on needless-range-loop diagnostics in math-core, outside
this step's changes; it is not reported as a clean dependency-wide lint run.

### Checked immutable topology snapshots

`TopologySnapshot` provides shared immutable indexed state, a retained vertex
admission callback and checked edits on an isolated model copy. The integration
test exercises successful sibling edits, failed edit rollback, corrupted
incidence, foreign-source point rejection and exact-coordinate retention after
clone/drop. Five topology tests pass in debug and release; module-only Clippy
passes with warnings denied.

This is a component transaction boundary. Mutable geometry payloads and external
callback side effects are outside its contract. Production transaction routing,
revision/CAS and stale/ABA handling, recipe persistence and solid certification
remain open.

### In-memory topology revision store

`TopologyStore` adds revision-checked publication of prepared immutable snapshot
edits. Checkout/transaction fields are private; commit verifies retained store
identity and base revision before changing state. Revisions cannot wrap, and
no-op commits advance them. Native tests prove sibling/stale rejection, ABA
coordinate restoration rejection, foreign-store rejection, failed preparation
and revision-exhaustion rollback. Six topology tests pass in debug/release;
module-only Clippy with warnings denied passes.

This addresses the shared component's single-writer stale/ABA behavior only.
Production model/undo routing, persistent recipes and revisions, concurrency
policy across workers, geometric solid validation and full qualification are
still open.

### Bounded revision-safe topology history

`TopologyStore` now supports Undo/Redo with at most 32 retained history entries.
Restoration always advances revision; branching clears redo. Tests cover
capacity eviction, empty-history behavior, redo invalidation, stale transactions
after undo/redo, exact vertex retention and revision-exhaustion rollback.
Seven topology tests pass in debug/release; module-only Clippy passes.

This component history does not yet replace production UI history or provide
persistent transactions. Application-owned payloads/external snapshot retention
need their own memory accounting. Full B-rep completion remains open.

### Whole-model validation in snapshot preparation

Snapshots can now retain a whole-model owner validator in addition to vertex
admission. The brep-core integration test uses the real rational cuboid and
Model::validate, rejecting invalid NURBS weights, NaN control points and a
detached curve endpoint before publication. Existing model validity and revision
are preserved, and the owner policy survives a successful commit/Undo/Redo.

The existing kernel validator retains its sampled/not-certified geometric
limitations. This is a native integration path for admission, not production
transaction routing or a new solid certification claim.

Validation: seven topology tests and one real-kernel transaction integration
test pass in both debug and release; topology module-only Clippy passes.

### Final publication admission

The topology store exposes a final caller check after identity/revision checks
and before publication. Cancellation after preparation preserves both history
stacks and the current model/revision. Real-kernel tests also verify that stale
transactions never invoke the final callback. Two integration tests pass in
debug/release; seven topology tests pass in debug; module-only Clippy passes.
This hook does not itself implement hard cancellation, worker routing or
production UI integration. Those wider requirements remain open.

### Native complete-model transactions

The shared revision/history engine is generalized over admitted immutable state.
`brep_core::transactions::ModelSnapshot` validates and retains the full Model,
including identity and lineage tables. Its ModelStore supports replacing the
whole model in an edit, rather than pinning old identity tables in a callback.
A real Boolean through-hole test verifies changed topology, coupled identity
restoration via Undo/Redo, stale rejection and invalid identity rollback.

This advances native kernel integration. UI/WASM transaction routing, persisted
transactions and exact construction recipes, cross-worker resource policy and
qualified geometric certification remain open.

Validation: 111 debug tests across brep-core/topology pass; three kernel
transaction and seven topology tests pass in release. Full Rust workspace
`cargo check --offline --locked` and topology module-only Clippy pass.

### Versioned full-model snapshot encoding

Native ModelSnapshot now supports a strict versioned JSON envelope with an
8 MiB encoded size limit and required identity tables. Decode revalidates the
full kernel model. The shared JSON codec gains opt-in duplicate-key rejection,
leaving the legacy decoder behavior unchanged. Tests cover escaped/nested
duplicate keys, corruption, oversize input, full Boolean-model roundtrip and
subsequent transaction/Undo behavior. Four transaction tests pass in debug and
release; value-codec tests pass in debug.

This persists the current full model, not pending transactions, history, store
identity or exact predicate construction recipes. Production save/load routing
and full B-rep qualification remain open.

### Validated model/history archives

Native history archives now retain current geometry/identities and both bounded
history stacks. Every embedded snapshot is revalidated before publication into
a fresh store identity. Tests prove Undo/Redo order, deterministic roundtrip,
foreign rejection of pre-load transactions, all-or-nothing corrupted-history
rejection and over-capacity refusal. Typed restoration also enforces the same
admission policy across historical states. Limits are 32 history entries,
8 MiB per model snapshot and 32 MiB per archive. Five integration tests pass in
debug/release; eight topology tests pass in debug; module-only Clippy passes.

Pending transactions and exact predicate recipe DAGs remain unpersisted.
Production save/load routing and full B-rep completion are still open.

### Admission continuity during preparation

The shared checkout now checks that the edited state reports the same admission
policy before creating a transaction. A deliberately faulty owner implementation
that substitutes its policy is rejected without changing geometry, revision or
history. This is a consistency guard over the owner contract, not authentication
of arbitrary trait implementations. Nine topology tests pass in debug/release;
five real-kernel transaction tests pass in debug; module-only Clippy passes.
Production integration and full B-rep certification remain open.

### Analytic model history roundtrip

Transaction persistence now has an analytic sphere-to-torus history regression.
It checks the sphere's two explicit pole vertices, complete geometry/identity
codec values, non-unit rational weight bits, validation after restoration and
Undo/Redo across both models. Six kernel transaction integration tests pass in
debug and release. This expands persistence evidence beyond planar Boolean
models; it does not certify general curved operations or persist predicate DAGs.

### Canonical empty across native transaction persistence

A full-cycle regression subtracts a model from itself, commits the canonical
empty, checks empty geometry/identity tables, archives/reloads history and
restores the original through Undo/Redo. It then unions a new body into the
restored empty state and undoes back to empty. Seven kernel transaction
integration tests pass in debug/release. This verifies the native persistence
path; production UI/WASM routing and the full B-rep requirements remain open.

### Portable 3D recipe records

`export_point3d` emits a bounded dependency-first graph with source label and
revision, authored leaf indices and exact bits/rationals, construction kinds,
root index, implementation identifier and tolerance specification. Shared
recipe dependencies appear once. No pointer identity, cached expansion or
rounded construction center is exported as source data.

These are portable typed records, not trusted admission tokens or a complete
file format/importer. A future importer must match the authored source and
replay validated operations. Checks cover shared dependencies after handle
drop, rational source preservation, ordering, repeatability and context/cancel
failures. **47 predicate tests pass in debug/release**; all-target Clippy passes.
Safe graph reconstruction and project persistence remain open.

### Source-checked 3D graph replay

`replay_point3d` checks schema/implementation, source label/revision, exact
source leaf values, tolerance bits, dependency ordering, reachability and graph
limits before rebuilding every operation. It binds the result to the supplied
already admitted source/context; it never creates an authored arena from the
record's embedded values. Line/plane and coplanar endpoint recipes are replayed
through existing checked constructors, with shared dependencies preserved.

Tests cover exact 1/3 and 4/3, rebinding to a separately admitted matching source,
shared DAG replay, modified values/revision/tolerance, forward references,
unreachable nodes, invalid operation semantics and cancellation. This is typed
record replay, not an authenticated file importer; callers still own source
admission. Byte encoding, full project integration and general construction
coverage remain open.

Replay validation: **49 tests pass in debug/release**; all-target Clippy passes.

### Bounded binary 3D recipe records

`encode_recipe_graph3` / `decode_recipe_graph3` implement BRG3 version 1:
little-endian integers, raw binary64 bits, signed rational numerators and
unsigned denominators, metadata and dependency-first operation records. Input
size is limited to 256 KiB, strings to 4096 bytes and nodes to 256 before their
allocations. Dependencies must precede their node; unknown tags/version,
truncation and trailing bytes are rejected. Decoding still produces untrusted
records requiring source-checked replay. No runtime dependency was added.

A roundtrip test preserves negative rational inputs and exact -1/9 replay,
checks byte stability, every truncated prefix, extra bytes, version corruption,
length overflow and oversize input. **50 predicate tests pass in debug/release**;
all-target Clippy passes. Project-level source admission and integration remain
open; this codec is not a complete B-rep file format or authenticity proof.

### Candidate 10: bounded one-pass exact replay

A new depth-32 binary roundtrip regression exposed ResourceLimit during replay:
the earlier implementation repeatedly reevaluated ancestor recipes through the
public constructors. Replay now evaluates each node once in dependency order,
retaining exact homogeneous results in a per-query cache capped at 65,536
expansion terms. Copy/arithmetic work remains charged; source admission and
coordinate/parameter publication checks remain in place.

The depth-32 overlap chain now restores under default limits with its exact
4/3 coordinate and shared recipe structure preserved. **50 tests pass in debug
and release**; all-target Clippy passes. The implementation identity advances
to candidate-10; replay continues to reject mismatching implementation records
explicitly. General project integration and full B-rep certification remain open.

### Exact vertex recipes through the indexed topology codec

The shared Vertex/Model codec now supports application-owned vertex payloads
through explicit value-codec trait bounds. The default binary64 wire shape is
unchanged; ConstructedPoint3 still has no implicit rounded serialization.
A native integration test exports all six exact intersection vertices to bounded
binary recipes, serializes the indexed contour as JSON, strictly decodes it,
replays against the admitted source, and revalidates incidence, exact equality
and membership in both source triangles. Curve/surface payloads in this test
are unit placeholders, so this demonstrates retained exact vertices and incidence,
not a certified trimmed surface or solid archive.

Validation: 121 tests across brep-core, brep-topology and value-codec pass in
debug; all nine topology tests pass in release. Module-only topology Clippy
with warnings denied and the native workspace check pass. Project UI/WASM
persistence integration and complete B-rep certification remain open.

### Atomic bounded vertex batch replay

`replay_point3d_batch` now decodes and restores an ordered set of binary vertex
recipes against one admitted source/context and one cumulative work budget.
Limits are 256 roots, 256 KiB per record and 8 MiB total encoded input; excessive
root count is rejected before scanning records. Decoding bytes consume work.
Malformed records, foreign source metadata and any indeterminate replay discard
the whole result; consumed work is not rolled back. This is atomic result
publication, not a topology/solid certificate or hard real-time cancellation.

The indexed six-vertex contour roundtrip now uses this batch path. Regressions
cover exact rational recovery, cumulative budget exhaustion after a successful
first vertex, corruption and foreign source in later records, count/per-record/
total byte limits, empty batches and cancellation. All 60 predicate/topology
tests pass in debug and release; module-only all-target Clippy passes with
warnings denied. Production project persistence and general curved B-rep
operations remain incomplete.

### Bounded batch export and publication checks

`export_point3d_batch` completes the save side of the vertex archive path:
ordered roots share one predicate work budget and the existing 256-root/8-MiB
batch limits. Individual recipes retain exact authored values and construction
DAGs. Failed exports return no partial archive and retain consumed work. Both
export and replay check cooperative cancellation/deadline immediately before
returning a successful batch; this does not prevent cancellation after return.
The six-vertex indexed topology integration now uses batch export and replay.

Regression evidence covers single-versus-double cumulative export cost,
second-root budget exhaustion, root count bounds, and cancellation/deadline on
empty batches. 61 predicate/topology tests pass in debug and release;
module-only all-target Clippy with denied warnings passes. This remains native
candidate infrastructure; UI project persistence and general curved B-rep
certification are still open.

### Ruled surface sections in either parameter direction

Surface/plane queries now admit ruled rational surfaces linear in U with matching
row weights, in addition to the existing linear-V case. The numerical algorithm
runs on the transposed retained surface and maps parameter boxes, unresolved
regions, samples and procedural traces back to original UV coordinates.
The new serialized `ruled_u` trace retains the source surface and V interval;
the TypeScript trace union includes this representation.

Tests cover horizontal/oblique cylinder sections and generator lines with shifted,
scaled knot domains, serialized trace evaluation, original-surface residuals and
budget-exhausted UV regions. All 10 intersection unit tests pass in debug and
release; native workspace check passes. Transposed floating-point evaluation can
differ in the last bit, so geometry is checked numerically, not claimed exact.
Coverage remains numerical_uncertified with topology changes forbidden. Browser
WASM packaging has not been rebuilt in this checkpoint; general surface/surface
intersection and curved Boolean certification remain open.

### Linear-U sections reach the packaged WASM adapter

The geometry WASM was rebuilt with linear-U sections. Four TypeScript/WASM
intersection tests pass, including JSON roundtrip of ruled_u traces, shifted
UV domains, cylinder section residuals, generator lines, invalid fractions and
budget-exhausted parameter regions. vue-tsc --noEmit passes. The packed geometry
literal was decoded through verifyPackedWasmChunk and matched the raw module
byte-for-byte (6,446,961 bytes). This verifies the generated adapter artifact;
a fresh production dist/browser interaction was not exercised here. General
curved Boolean and certification work remains open.

### Whole-domain admission of retained section traces

Trace evaluation now validates both line endpoints or both ruled interval ends
against the retained surface domain before interpolation. A valid requested
fraction cannot hide a malformed unused endpoint. Finite, in-domain reversed
intervals remain admitted for oriented traversal. Linear-U traces inherit the
same checks through parameter transposition.

All 11 native intersection tests pass in debug/release, including nonfinite and
out-of-domain endpoints and reverse traversal. Geometry WASM was rebuilt; all
four adapter tests pass with malformed serialized U/line traces rejected even
at fraction zero. vue-tsc --noEmit passes. These are parameter admission checks,
not a geometric completeness certificate; general curved Boolean remains open.

### Rational rulings with unequal endpoint weights

Surface/plane sections now admit linear-U/V rational rulings with unequal
positive endpoint weights. Trace evaluation computes both boundary homogeneous
weights using one normalization and solves the weighted plane equation for the
ruling parameter. Control-coefficient exclusion and unresolved-region handling
remain in place; no tessellation or fitted curve is introduced.

Native tests verify both parameter directions and serialized traces against the
analytic half-height parameter 1/4 for a 1:3 boundary weight ratio. WASM tests also
vary the ratio along the surface and compare UV against an independent quadratic
Bernstein formula. All 12 native intersection tests pass in debug/release; after
WASM rebuild all five adapter tests pass, and vue-tsc --noEmit passes. Query
coverage remains numerical_uncertified and cannot authorize topology changes.
General curved surface/surface Boolean remains incomplete.

### Knot-refined linear-U surfaces

Surface/plane orientation dispatch now uses degree one in U/V rather than
requiring exactly two global control rows/columns. Knot-refined linear-U
surfaces are transposed and decomposed into retained knot-span patches before
sectioning, so geometry-preserving knot insertion no longer disables support.
Tests insert a midpoint knot in a cylinder's linear direction and verify
sections on both sides, original UV, patch parameter boxes and cylinder residuals.

All 13 native intersection tests pass in debug/release. Rebuilt WASM passes all
six intersection adapter tests; vue-tsc --noEmit passes. This does not establish
sewing/completeness at knot-boundary contacts or authorize topology changes;
full curved Boolean and B-rep certification remain incomplete.

### Sections coincident with a ruling-span boundary

When one entire boundary coefficient row lies numerically on the plane and the
opposite row has a strict Bernstein side, sectioning emits the retained UV
isocurve instead of repeatedly subdividing an unresolved boundary band. A
continuous interior knot is owned by the preceding span to avoid duplicate
curves. This uses the query's existing numerical zero convention, not a new
exact certificate. Disconnected full-multiplicity knots remain rejected by
NURBS admission; they require separate geometry nodes.

All 14 native intersection tests pass in debug/release, including a section
exactly through an inserted knot and disconnected-input rejection. Rebuilt WASM
passes six adapter tests, now covering both exterior boundaries and the shared
knot. General branch sewing, curved Boolean and full certification remain open.

### Regression checkpoint after ruled-section expansion

The complete debug suites for brep-core, brep-topology and nurbs-core pass:
130 tests, including kernel operations, identities, archives, mass integration,
incidence and intersection queries. Six related TypeScript/WASM test files pass
43 tests covering analytic primitives, mass, regularized/stepped Boolean,
ModelGraph and intersections. These results establish regression evidence for
the current tested envelope, not the full qualification matrix.

The main requirements table now reflects candidate-10 recipe persistence,
versioned native history archives and the expanded ruled-section matrix.
General CC/CS/SS coverage, UV arrangements, production transaction routing,
STEP/IGES and certified curved Boolean are still missing; no completion status
or frozen qualification evidence was promoted by this checkpoint.

### Generator sections with proportionally weighted boundaries

The generator reduction now also recognizes a finite positive common ratio
between boundary weights when corresponding control points have equal plane
distances. In this admitted numerical case both boundary plane equations share
the same roots, so unequal weights no longer force an unresolved subdivision
band. The retained surface still controls rational traversal along the ruling.

All 15 native intersection tests pass in debug/release. Analytic cylinder tests
cover U/V orientation and z(t)=12t/(1+2t) for a 1:3 weight ratio. Rebuilt WASM
passes seven intersection adapter tests; vue-tsc --noEmit passes. The common
ratio/equality checks are numerical and do not establish certified completeness
or authorize topology changes. General curve/surface and surface/surface work
remains open.

### Nonproportional-weight section oracle

A regression now varies upper boundary weights by 2, 3 and 5 and cuts the
result with a vertical plane. It verifies that nonempty retained ruled branches
are not mislabeled as whole generator lines. Samples are checked against an
independent quadratic rational Bernstein formula for X and Z. With 128 boxes,
endpoint branch bands remain explicitly unresolved and coverage is incomplete;
this is an observed remaining solver limitation, not a complete section claim.

All 16 native intersection tests pass in debug/release, and eight tests pass
through the current WASM adapter. No runtime algorithm or WASM artifact changed
in this checkpoint. Complete endpoint isolation/branch joining and general
surface intersection remain required for the full B-rep objective.

### Fair bounded traversal across surface spans

A failing regression demonstrated that depth-first section traversal exhausted
eight boxes refining a difficult second span before processing a simple first
span. Surface subdivision now uses a FIFO queue: original spans are visited
before child refinement, then subdivision proceeds by depth. Existing box and
component budgets and explicit unresolved regions remain. Component traversal
order may change; components have no persistent topology identity.

The regression now retains the entire simple span within the same eight-box
budget. All 17 native intersection tests pass in debug/release, rebuilt WASM
passes nine adapter tests, and vue-tsc --noEmit passes. Fair scheduling improves
partial results; it does not resolve endpoint bands or certify completeness.
General intersection/branch joining and full B-rep remain open.

### Fair curve/plane isolation across knot spans

A regression reproduced starvation of an exact endpoint in a later curve span
while four boxes were consumed refining earlier roots. Curve/plane isolation
now uses one FIFO queue across source spans and their subdivisions. Source-span
coefficients are computed lazily after the box-budget check. Existing root
sorting, endpoint admission, initial-span overlap detection and explicit
unresolved outcomes remain in place.

The later endpoint is now retained within the same four-box budget. All 18
native intersection tests pass in debug/release. Rebuilt WASM passes ten adapter
tests; vue-tsc --noEmit passes. The query remains numerically uncertified;
fair scheduling does not establish full root/branch completeness or B-rep
solid certification.

### Native/WASM NURBS curve versus finite segment

Added curve_segment and intersectNurbsCurveSegment, extending the finite CC
query matrix beyond plane targets. Two supporting plane queries share the box
budget. Points retain source curve intervals, segment parameters and geometric
line residuals. Positive-weight control hulls handle finite segment admission;
whole coincident intervals are retained, while partial coincidence and uncertain
boundary bands remain explicit unresolved regions. No polyline approximation
or topology-changing certificate is introduced.

Native tests cover transverse hits, skew/disjoint cases, finite exclusion,
whole/partial overlap, shared-budget exhaustion, degenerate segment rejection,
and a rational circle arc with reversed segment correspondence. All 20 native
intersection tests pass in debug/release; native workspace check passes.
After bridge integration and WASM rebuild, eleven adapter tests pass and
vue-tsc --noEmit passes. General curve/curve pairs, coincident clipping,
certified root correspondence and full curved Boolean remain incomplete.

### Degree-one rational coincidence clipping

Curve/segment queries now clip partial degree-one coincident intervals by
inverting rational parameterization with normalized endpoint weights. Both
segment directions preserve ascending source-curve parameter intervals;
endpoint-only contact is returned as a point. A finite overlap whose parameter
interval collapses numerically remains unresolved. Higher-degree partial
coincidence remains CoincidentTrim rather than being approximated.

For weights 1:3, the independent analytic fixture maps geometric [1/4,3/4]
to source parameters [1/10,1/2]. Tests cover reversal and endpoint-only contact.
All 21 native intersection tests pass in debug/release; rebuilt WASM passes 12
adapter tests and vue-tsc --noEmit passes. This extends the numerical finite
query matrix, not certified B-rep topology operations; general coincidence and
curved Boolean remain open.

### Shared-knot contact uniqueness after rational clipping

A failing regression returned two identical parameter events for a curve that
touches a segment endpoint at its internal knot. Clipped singleton contacts now
use the known segment boundary parameter and reject duplicate source-parameter
events. Deduplication does not compare spatial coordinates: a second regression
keeps two visits to the same point at distinct curve parameters 1/4 and 3/4.

All 22 native intersection tests pass in debug/release; the extended repeated-
visit regression also passes in both profiles. Rebuilt WASM passes 13 adapter
tests and vue-tsc --noEmit passes. This fixes query event identity within a
single curve, not persistent topology naming or certified contact semantics.
Full B-rep completion remains unproved.

### Higher-degree coincident clipping with explicit root bands

Partial curve/segment coincidences of higher degree now isolate roots against
both segment boundary planes under the existing shared box budget. Source
parameter cells are classified using positive-weight control hulls; admitted
interior cells retain their original curve intervals. Root uncertainty and
unresolved solver regions remain explicit bands, never collapsed to guessed
trim parameters. Exact numerical boundary events are retained when not already
covered by an overlap interval. Cell classification also consumes box budget.

The independent x(t)=t² fixture maps segment [1/4,9/16] to [1/2,3/4], including
reversed segment direction. Nondyadic roots and low budgets remain incomplete.
All 23 native intersection tests pass in debug/release; rebuilt WASM passes
14 adapter tests and vue-tsc --noEmit passes. These numerical query results
still cannot authorize B-rep topology edits. General certified coincidence,
curve/curve and surface/surface completeness remain open.

### Backtracking coincidence and authoritative trim endpoints

An independent x(t)=4t(1-t) regression exposed a false CoincidentTrim band for
one of two visits to the same segment: knot insertion rounded the trimmed
control endpoint outside the boundary. Cell hull classification now evaluates
its two endpoints on the authoritative source curve at the retained parameters,
while retaining interior trimmed controls. No tolerance snapping was added.

The two visits are retained separately as [1/8,1/4] and [3/4,7/8]. Nondyadic
fixtures independently verify all four analytic boundary roots are covered by
unresolved bands and accepted intervals remain inside the segment. All 24
native intersection tests pass in debug/release; rebuilt WASM passes 15 adapter
tests and vue-tsc --noEmit passes. These remain numerical checks, not certified
trim or solid completeness; the full B-rep goal remains open.

### Coplanar NURBS curve clipping to affine surface domains

Curve/surface queries now clip coplanar curves to finite affine rectangles by
isolating roots against all four boundary planes, using the frame's dual vectors
and a shared query budget. Retained source-parameter cells are classified by
positive-weight control hulls with authoritative endpoint evaluation. Curves
along rectangle edges remain admissible; unresolved root bands remain explicit.
Isolated admitted boundary contacts are retained when no overlap covers them.

The x=t,y=t² fixture is clipped to [1/4,3/4] with shifted/scaled surface knot
domains; edge coincidence and two-box refusal are also tested. All 25 native
intersection tests pass in debug/release. Rebuilt WASM passes 16 adapter tests;
vue-tsc --noEmit passes. This is finite affine-domain numerical clipping, not
general lifted UV arrangements, curved-face trimming or certified Boolean.

### Affine clipping shear and corner evidence

Additional native and WASM regressions verify the same source interval after
shearing both the curve and rectangle, and a single parameter/UV event when a
curve touches the common corner of two boundaries. This exercises the dual-frame
projection and boundary-event deduplication rather than only axis-aligned cuts.

All 26 native intersection tests pass in debug/release; 17 tests pass through
the existing WASM adapter, and vue-tsc --noEmit passes. Runtime implementation
and WASM bytes did not change in this checkpoint. The main matrix now also
records finite-segment and affine-domain coincidence clipping; general lifted
UV arrangements and certified B-rep completion remain open.

### Finite affine surface/surface query in Rust and WASM

Added surface_surface / intersectNurbsSurfaceSurface for two finite affine
rectangular patches. Transverse segments retain a trace on each source surface;
the same fraction evaluates corresponding UV/3D points. Finite clipping also
returns isolated contact points and disjoint results. Coplanar area overlap and
curved pairs remain explicitly unresolved. Pair unresolved boxes concatenate
both original UV domains. Correspondence residuals are sampled and numerical;
reports continue to forbid topology changes.

All 27 native intersection tests pass in debug/release, including operand swap
and shared budget. Rebuilt WASM passes 18 adapter tests, covering paired trace
JSON replay, finite endpoint contact, bounded disjointness, parallel separation,
coplanar refusal and unsupported curved pairs; vue-tsc --noEmit passes. General
surface intersection, coplanar areas and certified curved Boolean remain open.

### Coplanar affine surface overlap with paired UV boundaries

Surface/surface queries now clip a coplanar affine rectangle against the four
boundary halfspaces of the second patch. Area results retain an implicitly
closed convex polygon in both UV domains and source-evaluated 3D vertices;
edge and point contacts retain their lower-dimensional result types. Each
clipping pass consumes the shared box budget. Numerically degenerate area or
failed correspondence remains unresolved rather than becoming a solid face.

The square/diamond oracle produces eight vertices and area 3.5. Self-overlap,
shared edge, corner, disjointness and budget limits are covered. All 28 native
intersection tests pass in debug/release; rebuilt WASM passes 19 adapter tests
and vue-tsc --noEmit passes. Release/WASM compilation was slow but verified
live and completed without restart. General curved surface pairs, UV/DCEL
assembly and certified B-rep operations remain incomplete.

### Coplanar pair invariance and UV correspondence matrix

Eight combinations of operand swap, U reversal and UV transposition now verify
the same square/diamond intersection. Source knot domains are shifted/scaled
and one surface's weights are uniformly scaled by 32. Each case checks eight
vertices, physical area 3.5, containment in both analytic polygons, and equality
of each stored 3D vertex with evaluations in both returned UV systems. Boundary
ordering may follow the first UV orientation; geometry is not compared by array
order or guessed identity.

All 29 native intersection tests pass in debug/release; 20 tests pass through
the existing WASM adapter and vue-tsc --noEmit passes. Runtime code and WASM
bytes did not change. The main completion matrix records the finite affine SS
capability; general curved pairs and certified solid completion remain open.

### Algebraic section-trace conversion to NURBS curves

SurfaceTrace::to_curve and intersectionTraceToNurbsCurve now convert supported
traces to retained rational curves without fitting samples. Affine traces and
isoparametric lines use native curve extraction; single-Bezier ruled sections
form homogeneous Bernstein products of the two boundaries and plane equations.
The resulting degree is twice the curved direction's degree, bounded to 24;
a positive-weight representation and normal NURBS validation are required.
Unsupported multi-span/degree/weight representations fail explicitly.

Tests compare converted curves to retained procedural traces in U/V orientations
with unequal boundary weights, oblique/horizontal cuts and generators. Independent
cylinder/plane residual checks use the query's 1e-9 tolerance; generator roots
already carry numerical isolation error and are not claimed exact plane points.
All 30 native intersection tests pass in debug/release, rebuilt WASM passes
21 adapter tests, and vue-tsc --noEmit passes. Floating-point algebra is not a
certified construction, and general curved B-rep completeness remains open.

### Reversed section conversion and ambiguous-ruling regression

Conversion regressions now cover reversed trace fractions on shifted U/V knot
domains for both surface orientations and generator/isoparametric sections.
Converted NURBS points agree with the original trace at the reversed fraction
within 1e-10. A ruling family whose endpoint evaluations succeed but whose
homogeneous conversion has mixed-sign weights is explicitly rejected. This is
a conservative representation refusal, not a claim that endpoint sampling
certifies the interior.

The debug and release intersection suites each pass 32 tests; the existing WASM artifact passes
23 adapter tests, and vue-tsc --noEmit passes. These changes add regression tests
only and require no new runtime artifact. General curved intersections and
certified topology-changing operations remain incomplete.

### Multi-span ruled-trace conversion

Ruled trace conversion now decomposes the requested interval at source knots,
converts each Bezier span algebraically, and assembles one C0 rational curve
with the original parameter domain. Adjacent spans rescale their homogeneous
weights to share an endpoint weight. A shared endpoint is admitted only when
its Cartesian control coordinates are numerically identical; no proximity
welding is performed. Nonmatching endpoints, invalid weight conditioning and
results exceeding 256 control points are explicitly rejected. Reverse traversal
is applied to the assembled curve. This extends retained section geometry,
without certifying intersections or publishing new B-rep topology.

Validation: 33 native intersection tests pass in debug and release, including a valid
degree-12 surface whose converted representation exceeds the output budget.
The rebuilt WASM passes 24 adapter tests; TypeScript checking and packed-WASM
byte verification pass. Multi-span point comparisons include interior knot
locations and reversed traversal.

### Varying-weight multi-span regression

Native and WASM tests additionally cover independently varying weights on both
ruled boundaries with an oblique cutting plane, transposed surface orientation
and reversed traversal. Source-trace agreement and an independent plane residual
are checked to 1e-10, including the interior knot. The intersection suite passes
34 tests in release; the WASM adapter passes 25 tests and TypeScript checking
passes. This regression adds no runtime code and uses the previously rebuilt
artifact.

The full debug command `cargo test --offline --locked -p brep-core
-p brep-topology -p nurbs-core --manifest-path crates/Cargo.toml` completes with
150 passing tests and no failures. This covers current constructor, supported
Boolean, mass-property, identity, transaction and NURBS regressions, in addition
to the query tests; it does not prove the open completion-matrix requirements.

### Tensor-patch UV diagonal lifting

`SurfaceTrace::Line` now converts non-isoparametric UV segments on a single
tensor Bezier patch to a rational curve of degree p+q (maximum 25). The source
is trimmed to the segment's UV bounding rectangle; homogeneous Bernstein
products restrict its tensor polynomial to the diagonal, accounting for both
UV directions. Output parameter fraction is [0,1]. No sampled fitting is used.
Multi-span diagonals remain an explicit refusal; iso-parametric and affine
paths retain their existing behavior. A zero-length UV segment produces a
constant degree-one curve.

This operation lifts the supplied UV path. It does not establish that a
caller-supplied path lies in the trace's plane; evaluated plane residuals remain
separate numerical evidence. Sphere tests independently verify the radius,
source agreement, partial shifted domains, all four UV directions, degree limits
and multi-span refusal. General curved intersection and trim certification
remain open.

Validation: 35 native intersection tests pass in debug and release. The rebuilt WASM passes
26 adapter tests; TypeScript checking and packed-byte verification pass.

## Expanded objective: move the B-rep implementation to Rust

The user expanded the objective to include moving everything to Rust. Existing
Rust kernels and WASM wrappers alone do not satisfy that requirement. The
following current TypeScript implementations are concrete remaining migration
work, verified from source after the objective update:

- `src/services/brepSemanticBackend.ts`: polygon/circle profile construction,
  profile affine transformation and admission, semantic node execution, leases
  and retained-payload state remain implemented in TypeScript.
- `src/services/directModeling.ts`: sketch winding decisions, B-rep workplane
  placement by rewriting vertices/edge controls/surface controls, point
  transforms, document parsing and history remain implemented in TypeScript.
- `src/services/brepSemanticScene.ts` and `brepDiagnosticExecutor.ts`: scene
  orchestration and host execution state require a migration-boundary audit.

This inventory is a starting point, not an exhaustive completion claim. Native
Rust semantic execution, shared transactions, profile authoring/transforms and
authoritative validation must replace duplicated TypeScript behavior with
verified native/WASM parity. Browser/MCP bindings and remaining application
modules must also be inventoried against the expanded objective before claiming
that all work has moved to Rust.

### Rust migration: affine B-rep and workplane placement

`crates/brep-core/src/transform.rs` now owns affine-matrix admission, normalized
determinant classification, retained vertex/edge/surface placement, reflected
shell orientation and input/output model validation. Workplane basis and local
offset composition also execute in Rust. `brep_nurbs_transform` and
`brep_nurbs_workplane` expose those operations through the shared bridge.
The former TypeScript transform implementation and the direct sketch's manual
carrier-rewriting loops were removed. Existing exported functions now transport
arguments to the Rust implementation.

This completes only this migration slice. Sketch construction/winding, profile
construction and transforms, semantic execution, history and the rest of the
expanded inventory still require migration; bindings alone do not satisfy the
full Rust objective.

Migration validation: both native transform tests pass in debug/release. The
rebuilt WASM passes 41 existing integration tests across analytic solids, mass
properties, regularized/stepped Boolean operations and the semantic backend,
with `--maxWorkers=1` and the unchanged 5-second per-test limit. An initial
parallel run had five timeout failures and 36 passes; the serial rerun passes
all assertions. Binding type checking and packed-WASM byte verification pass.

### Rust migration: sketch extrusion authoring

`crates/brep-core/src/sketch.rs` now owns closed-sketch and height admission,
circle versus polygon authoring, clockwise polygon reversal, signed-height
placement and workplane composition. The bridge parses the sketch and invokes
that native API. `extrudeSketchBrep` no longer implements those decisions or
geometry computations in TypeScript. Circular sketches retain analytic cylinder
surfaces, including off-origin circles on custom workplanes.

This does not migrate direct document/history state, display-mesh authoring,
semantic profile execution or the remaining application logic. Those remain
part of the expanded Rust objective.

Sketch migration verification checkpoint: two native tests pass in debug and
release. Shared WASM rebuilding is currently incomplete: concurrent G-code
work changed slicer-core to require GcodeBounds, COORDINATE_RESOLUTION_MM and
MAX_COORDINATE_MM before gcode-core exports them. The active task “Продолжить
работу с G-code” owns that ongoing implementation. A binding type-check also
reported TS2698 in the independently edited svgWorkerClient.ts. Neither
unrelated workstream was overwritten. The new sketch operation is therefore
not yet verified or available in the previously generated WASM artifact; the
new integration test must run after a successful rebuild. Do not count this
as an integrated, complete migration until that evidence exists.

### Rust migration: semantic profile authoring

Native `sketch::polygon_wire` and `sketch::circle_wire` now construct retained
line and rational quadratic contours. `brep_profile_author` owns rectangle
placement, circle authoring and polygon even-odd orientation/validation. The
semantic backend no longer constructs these curve controls or rectangle corner
coordinates in TypeScript. Three native sketch/profile tests pass, including
independent circle-radius and polygon-area checks. Full WASM integration is
being rebuilt after the concurrent G-code dependency symbols became available.
Profile transforms and semantic session execution remain migration work.

Integration follow-up: shared WASM rebuild now succeeds after the concurrent
G-code definitions landed. The new sketch-extrusion and profile-authoring
operations are present in the rebuilt artifact. All 31 tests across
`brepAnalytic`, `brepProfile` and `brepSemanticBackend` pass with one worker;
all three native sketch/profile tests pass in debug/release and packed WASM
bytes match the binary. The previously recorded artifact blocker is resolved.
The last global type-check still reported only the unrelated SVG worker TS2698
error; it is not reported as a passing global check.

### Rust migration: planar profile transforms

`transform::profile` now owns column-major affine admission, XY-plane
preservation, normalized determinant classification, circular-profile similarity
restriction, control-point transformation, reflected contour reversal and
pre/post material-left validation. The old TypeScript `transformProfile`
implementation has been removed. `brep_profile_transform` returns a newly
validated retained profile, preserving the authored tolerance. Unsupported
transform errors originate in Rust and retain semantic unsupported mapping.

Native tests independently verify reflected circle radius/area, input
immutability, polygon shear area and elliptical/out-of-plane refusal. General
elliptical trims remain unsupported; this migration does not claim to complete
that geometric capability or the remaining semantic/session/history migration.

Profile-transform migration verification: four native transform tests pass in
debug/release. Rebuilt WASM passes all 31 existing profile, semantic-backend
and analytic-solid integration tests with one worker, including unsupported
transform error mapping. Global interface type checking and packed-WASM byte
verification pass. The earlier unrelated SVG type-check failure is no longer
present in this run.

### Rust migration: reduced semantic geometry execution

`geometry-bridge/src/brep_semantic.rs` now executes the admitted reduced
geometry-node matrix in Rust: centered boxes/frusta, spheres, profile authoring,
2D/3D transforms, ordered Boolean folds and straight profile extrusion with
explicit twist/scale refusal. The TypeScript geometry-operation switch and
Boolean fold loops were removed; the session transports validated owned input
geometry to the Rust operation. Native tests cover a profile-transform-extrusion
chain and noncommutative Boolean operand ordering.

This is a reduced geometry-node executor, not a complete native SemanticProgram
implementation. DAG admission/reduction, identity, leases, cancellation transport,
session lifetime and retained-state management still require migration. The
permanent provider and full B-rep completion claims remain open.

Reduced-executor verification: native geometry-bridge compilation and both
composition/order tests pass. The rebuilt release WASM passes all 41 tests
across semantic backend, semantic scene, diagnostic executor and retained
profiles with one worker. Global type checking and packed-byte verification
pass. The reduced native endpoint explicitly caps operand count at 512; this
is a bounded geometry endpoint, not proof of complete native program admission.

### Native reduced-node admission

The Rust semantic geometry endpoint now requires the operation's explicit d2/d3
result space, rejects operands supplied to primitives, enforces unary arity and
checks solid/profile operand kinds before evaluation. Missing spaces no longer
default to 3D and primitive inputs are no longer silently ignored. Native
negative tests cover these cases alongside working composition/order tests.
This closes concrete endpoint admission gaps; complete typed DAG validation,
carrier identity and native session ownership remain open.

Admission verification: all three native semantic tests pass. Rebuilt release
WASM passes 42 integration tests across semantic backend, scene, diagnostic
executor and profiles. A direct WASM test bypasses the TypeScript semantic
validator and proves native refusal of extra primitive operands, wrong result
space and missing Boolean space. Packed-byte verification passes.

### Singleton Boolean admission correction

Rust now validates Boolean operation names before folding operands. Previously
a singleton request could bypass the underlying binary operator and return its
operand for an unknown operation name. Both profile and solid requests now
refuse that case explicitly. Native tests also verify that all four supported
operations preserve the sole operand. Four native semantic tests pass.

Singleton correction verification: rebuilt release WASM passes all 43 semantic
backend/scene/diagnostic/profile integration tests. Direct calls with unknown
operation names and one operand fail in Rust for both d2 and d3; valid singleton
union results preserve their operand. Packed artifact verification passes.

### Native reduced-session ownership implementation

`geometry-bridge/src/brep_session.rs` adds a native session owner for reduced
geometry execution. Opaque leases retain owner identity through Arc, preventing
foreign-session acceptance and identity reuse while a lease survives. Node IDs
are admitted once; input leases must match backward source references. Geometry
results are immutable Arc<Value> entries with an aggregate encoded-byte budget.
Evaluation errors poison the session without publishing a partial result.
Release is idempotent for previously issued leases; commit admits a unique set
of live owned leases atomically and transfers their immutable values to an
owned bundle. Abort/failed commit clear retained entries; bundle lifetime is
independent of the closed session.

This is an implemented native ownership layer, not yet the production session
route. It does not yet handle typed-empty reduction, complete program-context
admission, asynchronous cancellation, a bounded peak allocation certificate or
serialized ABI handles. The TypeScript session has not yet been replaced; that
integration remains required. No WASM entry point or artifact changed for this
native-only ownership step.

Native ownership verification: all three session tests pass in debug and
release. They cover foreign release, idempotent release, immutable committed
results, closed-session refusal, budget failure without publication, duplicate
node refusal, input-reference composition, released-lease commit refusal and
atomic duplicate-commit cleanup. Production ABI/session integration remains open.

### Native session snapshot and ownership regressions

A session can now lend an immutable snapshot only through a live owned lease
while the session is open. Foreign/released/closed access is refused. Additional
native regressions exercise foreign leases at evaluation, released inputs,
node-reference mismatches and aggregate-budget failure after an existing result.
Failed receiving sessions preserve the unrelated source session; failed commit
clears retained memory without publishing a bundle. These changes remain within
the native ownership layer; production session replacement and WASM handles
are still outstanding.

Snapshot/ownership verification: all five native session tests pass in debug
and release. No production WASM endpoint was changed in this native-only step.

### Native session typed-empty outcomes

Native session entries now retain their declared value type and distinguish
canonical empty geometry from ordinary results. `outcome` lends a typed empty
or a typed geometry view under the same owner/lease checks. Committed bundles
retain this metadata and expose typed outcomes after the session closes. The
retained-byte budget now includes the encoded type metadata. Canonical empty
geometry remains available internally for subsequent native operations.

A native test exercises A-minus-A, reuse of the empty result in union with A,
and commit of both empty and nonempty results for d2 profiles and d3 solids.
This preserves declared type data, not a complete carrier/evidence validation
certificate. Six debug session tests pass; WASM handle transport and production
session replacement remain outstanding.

Typed-empty verification: all six native session tests also pass in release.
No production WASM entry point changed in this native ownership-layer step.

### Rust session wire interface

`brep_session` exposes begin/evaluate/snapshot/release/commit/dispose through a
Rust registry. Active sessions retain native leases; committed slots retain
owned bundles. Handles are decimal strings with monotonically increasing IDs
and are not reused within the registry lifetime. Foreign handles cannot select
another session's result. A failed evaluation aborts the receiving session.
Commit retains only selected result handles; dispose drops the owner and its
reserved budget. Registry limits are 64 slots and 64 MiB of reserved retained
geometry budgets (including committed slots), not a peak-process-memory claim.

Handles are scoped to one WASM instance/native thread registry, not portable
capabilities across workers or process restarts. Host epoch/cancellation/context
integration and replacement of the production TypeScript session remain open.

Session wire verification: eight native session/registry tests pass. The rebuilt
release WASM passes 33 tests across the new direct session ABI, semantic backend
and diagnostic executor. Direct tests cover commit/dispose revocation, foreign
lease isolation and typed-empty commit with unretained-handle refusal. Interface
type checking and packed-byte verification pass. Existing production sessions
still use their old owner until the remaining host integration is performed.

### Application backend connected to native session ownership

The semantic backend now lazily opens a Rust session and evaluates by native
lease references, rather than re-sending input geometry to each operation.
Snapshot reads obtain typed native outcomes; empty results are released after
host empty propagation. Release, commit and result disposal call the native
registry. Failures dispose the native owner. The old geometry emptiness test
was removed from TypeScript. Reduced Boolean references preserve the executor's
admitted source order. Native unsupported-node admission was corrected so
unsupported operations retain their semantic refusal code before reference
processing.

This integrates native ownership, but does not finish the migration. Host
protocol/context checks, branded lease mapping, immutable display snapshots and
the legacy character-count budget remain in the adapter. The native owner has
an independent byte budget; the existing character-count contract is retained
until compatible native accounting is implemented. Complete DAG/context
admission, instance-epoch handling and removal of remaining duplicated state
checks are still required.

Native-owner integration verification: all eight native session/registry tests
pass; the rebuilt WASM passes all 42 tests across semantic backend, scene,
diagnostic executor and direct native sessions. This includes the unchanged
legacy budget/release tests and unsupported-operation error contracts. Interface
type checking and packed-artifact verification pass. Application B-rep execution
now actually uses the native owner; remaining host checks/accounting are not
claimed migrated.

### Native legacy character-budget accounting

The aggregate legacy snapshot-character budget now executes in Rust. Session
entries record payload character counts; release/abort updates them and
over-budget evaluation refuses publication with BREP_SEMANTIC_BUDGET. The
TypeScript JSON.stringify counter and accumulator were removed. The configured
character limit is passed at native session creation alongside the independent
encoded-byte bound. Empty outcomes retain the prior zero-payload-character
semantics.

`brep_json_size` counts UTF-16 JSON characters, including string escapes, -0
normalization and browser fixed/scientific number notation boundaries. A checked
in deterministic JavaScript-generated oracle covers 2048 finite binary64 values;
native comparisons pass, as do explicit number/Unicode boundary checks. This
is tested compatibility evidence, not a universal numeric-format certificate.

Native character-budget integration verification: rebuilt WASM passes all 42
semantic backend/scene/diagnostic/native-session tests, including the unchanged
exact single-snapshot character limit, release-and-reuse, aggregate overflow
and one-character-below-limit refusal tests. The independent 2048-number
oracle and explicit formatting tests pass natively. Interface type checking
and packed-byte verification pass. The former TypeScript snapshot-character
accumulator is removed; this part of the remaining migration is now native.
The fixture generator is scripts/generate-brep-json-number-lengths.mjs.

### Native node identity admission

Rust Session now validates nonnegative integer node identities, the configured
node range, and identity reuse (including after release), reporting
BREP_SEMANTIC_CONTRACT. The TypeScript adapter no longer keeps an evaluated-node
set or duplicates the identity range check; it translates the native error to
the existing host diagnostic. Context matching and host payload checks still
remain in TypeScript and are not counted as migrated.

All nine native session/ABI tests pass with offline locked dependencies,
including malformed identities and replay after release. This checkpoint does
not establish complete B-rep functionality or complete Rust migration.
The rebuilt WASM passes all 42 backend/scene/diagnostic/native-session integration
tests; interface type checking and packed-byte verification also pass.

### Native result admission before publication

The native Session now checks nonempty result dimension, body presence, closed
shells, single-body solid cardinality, and Model::validate before publishing a
lease or charging retained geometry. The equivalent TypeScript conditions and
the extra inspectNurbsBrep bridge call were removed. Empty result semantics are
unchanged. These checks admit topology; they do not certify solid correctness.

Ten native session/ABI tests pass, including damaged topology, missing bodies,
open shells and dimension mismatch. Rebuilt WASM passes 43 integration tests;
the direct native-session test accepts disconnected union as solid-set, rejects
it as solid, and refuses commit after that rejection. Type checking and packed
WASM verification pass. Full semantic context ownership and complete B-rep
functionality remain open requirements.

### Native semantic carrier admission

Session evaluation now requires an explicit supported geometryKind/space pair,
analytic-brep representation and representation-preserving evidence. Missing
metadata, mesh representation, incompatible dimension/kind and certificate
claims cannot enter the native owner through the session ABI. Failure poisons
the session and prevents commit. The TypeScript carrier/evidence admission
branch was removed; its remaining carrier-key cast follows successful native
evaluation. Direct reduced geometry helpers remain lower-level operations,
not semantic sessions or evidence issuers.

Eleven native session/ABI tests pass, including unsupported carrier/evidence
refusal without result publication. This is a native admission checkpoint;
full program/context ownership and complete B-rep qualification remain open.
The rebuilt WASM passes all 44 backend/scene/diagnostic/native-session tests,
including direct ABI rejection of incomplete carriers, mesh representation and
certificate evidence. Interface type checking and packed-byte verification pass.

### Native reduced operand ordering

The adapter now forwards the original semantic node and reduced input indices
separately. Rust checks that Boolean reductions preserve authored order and
multiplicity; other operations require the original references unchanged.
Lease identity and backward-reference validation then apply to the selected
indices. The host subsequence loop and Boolean node rewriting were removed.
The native direct evaluate API remains the unreduced path.

Twelve native session/ABI tests pass, including valid ordered reduction,
duplicate multiplicity refusal and reordering refusal. This does not prove
that every omitted operand was empty: empty reduction and full program
ownership still belong to the host executor and remain migration work.
Rebuilt WASM passes 45 backend/scene/diagnostic/native-session tests, including
direct ABI rejection of reordered difference operands and subsequent commit.
Type checking and packed-byte verification pass.

### Native SemanticResult selection

brep_result now admits empty/single/multi result shapes, exact output fields,
bounded references, increasing identity-occurrence order, RGBA range, producer
node matching, shared occurrence root/branch and transform-preserved identity
lineage. Parent and transform traversal reject invalid forward/cyclic links.
Full occurrence identity hashing and operation admission remain separate.

Graph requests can provide result plus occurrences instead of raw output indices.
Native selection retains each geometry root once and returns ordered resultItems
with reference metadata and outcomeIndex into unique outcomes. Repeated output
slots therefore do not require repeated geometry computation or serialization.
Providing both selection forms is rejected.

Thirty-four native B-rep tests pass, including duplicate-root output slots,
invalid branch/lineage/order/color and empty-result selection. Main application
executor integration and complete program admission remain outstanding.
Rebuilt WASM passes 50 integration tests. A lowered two-output OpenSCAD program
preserves its exact result references and outcome mapping through native
selection; reversed output order is rejected before execution. Type checking
and packed-byte verification pass.

### Cooperative native graph runner

brep_graph_runner::Runner owns graph order, pending nodes, leases, use counts and
the Session across bounded advance calls. abort revokes geometry and closes the
runner; consuming commit refuses incomplete or failed execution. Existing
one-shot graph execution now drives this runner, avoiding a second executor.
Advancement boundaries are between geometry nodes, not inside a kernel call.

Thirty-six native B-rep tests pass. New runner tests compare chunk sizes 1, 2
and 65,536, reject partial commit, verify repeated abort and zero retained bytes,
and preserve failure location/completed-prefix count after a failed step. The
public native stepping API is available; stepwise WASM ownership/transport and
host cancellation integration remain to be connected.
The rebuilt WASM passes all 50 integration tests and packed-byte verification.

### Stepwise graph WASM ownership

The shared brep_session registry now supports graph-begin, graph-advance and
graph-commit with the existing dispose action for cancellation. Graphs and
ordinary sessions share the same 64-slot/64-MiB reservation limits and monotonic
instance-local handle allocator. Preparation, semantic selection and report
serialization are shared with the one-shot graph path.

Graph advancement reports completed/total nodes and progress/ready status.
Commit consumes the graph handle and releases its reservation on success or
failure; incomplete commit publishes no outcomes. Dispose revokes the graph
between steps. A failed advance closes its runner; callers still dispose or
commit-consume the handle to release the slot reservation.

Thirty-eight native B-rep tests pass, including mixed-session budget exhaustion,
step completion, consuming commit, incomplete commit, cancellation and handle
non-reuse. Application-level progress/cancellation wiring remains pending;
individual synchronous geometry calls cannot be interrupted by this API.
Rebuilt WASM passes 52 integration tests, including stepwise execution of an
authored OpenSCAD graph, exact result metadata, consumed-handle refusal and
cancellation between steps. Type checking and packed-byte verification pass.

### Application scene uses the native graph executor

buildBrepSemanticScene now calls executeBrepNativeProgram, which transports
graph-begin/advance/commit to Rust. The scene path no longer uses the generic
TypeScript node executor or its empty-input reduction. Native resultItems supply
the output references; the scene no longer independently selects result slots.
The old backend remains for compatibility and regression comparisons.

The host adapter checks trusted lowering, cancellation/deadlines and node limits,
forwards progress, yields periodically to the event loop and disposes the native
handle on failure. Rust owns DAG decisions, geometry, liveness and commit.
Committed display snapshots are frozen and released through an idempotent host
disposer. Cleanup errors do not replace the primary execution error. These host
mechanics and source/envelope admission are not claimed to have moved to Rust.

Verification: 55 B-rep integration tests passed, including actual diagnostic
workers, cancellation, callback failure, immutable snapshots and recovery after
failure; 168 shared SemanticProgram/oracle tests passed. After final reference
mapping and cleanup adjustments, all 22 affected executor/scene/worker tests
and type checking passed again. No WASM rebuild was needed for this host wiring.
The permanent qualified provider, complete program admission and the remaining
geometric requirements in the audit matrix are still open.

### Native display-buffer preparation

brep_nurbs_display tessellates and prepares flat-normal Float32 display buffers
in Rust. It computes mesh surface-area estimates, rejects distinct f64 positions
that merge after Float32 conversion, rejects non-finite coordinates and refuses
triangles that collapse or reverse after rounding. The scene's TypeScript
normal/area/rounding loops were removed; it wraps the returned arrays for the UI.
Topology face IDs and the original tessellation report are preserved.

Two native display tests pass (normals/area and Float32 merge/collapse/overflow).
All 56 B-rep integration tests pass on rebuilt WASM, including the existing
far-origin collapse/contact regressions and a box's exact display area and
flat normals. Type checking and packed-byte verification pass. Vite production
build and verify-dist pass: 68 artifacts, 4,699,211 bytes, within the existing
4,700,000-byte budget. This display conversion does not certify surface geometry
or replace the remaining full B-rep audit requirements.

Native display transport follow-up: removed unused original mesh positions and
indices from the response, including their native JSON allocation. The host uses
displayIndices for triangle counts. Face IDs, display buffers, report and area
remain present. Measured UTF-8 JSON responses at eight segments decrease from
1,326 to 1,180 bytes (box), 398,107 to 362,055 (sphere), and 171,400 to 153,872
(cylinder). Tests explicitly refuse reintroduction of the redundant fields.

The new runtime is bound by
docs/qualification/brep-display-transport-v1.json. Native display tests, 56 B-rep
integration tests, refreshed SVG and 218 own-CAD tests pass. The full application
run passed 2,635 tests; seven HTTP tests were blocked by sandbox listen EPERM,
then all seven passed with local-port permission. Five optional runtime tests
remain skipped. Both type checks and packed-byte verification pass; production
dist is 4,699,595 bytes within the unchanged 4,700,000-byte limit. Source and
WASM hashes match the refreshed SVG evidence. Previous browser evidence remains
bound to its original snapshot; no new browser verification is claimed.

### Native empty-set regression correction

Differential execution found five regressions after the scene switched to the
native graph: an empty translated 2D result outside XY, twisted extrusion of
an empty profile, rotate-extrude, projection and offset of empty inputs failed
instead of yielding typed empties. Rust now applies empty algebra after node
kind, operand arity/kind and result-dimension admission, before nonempty feature
construction. All-empty hull is handled likewise. Reference extraction recognizes
these node kinds without prematurely treating them as unavailable geometry.
Nonempty unsupported feature implementations remain unsupported.

All seven audited source programs now match the old executor's empty outcomes;
the regression suite also covers a two-operand empty hull. Verification is bound
in docs/qualification/brep-empty-algebra-v1.json: 41 native B-rep tests, 57 B-rep
integration tests, 19 artifact/worker tests, refreshed SVG/218 own-CAD checks,
both type checks and production dist (4,699,439 bytes) pass. No complete native
program-admission or general feature qualification claim follows from these
empty-input cases.

### Native empty Boolean and extrusion execution

Native semantic execution now constructs typed empty geometry for supported
zero-operand Booleans and for straight extrusion of an empty profile. Unknown
Boolean operation names are checked before the empty fast path. Native Session
can retain, reuse and commit those outcomes without a host-produced empty value.

Twenty-five native B-rep bridge tests pass. The new session test exercises all
four Boolean operations with an empty operand on either side in both dimensions,
zero-operand construction and empty-profile extrusion. The shared TypeScript
executor still performs its own empty reduction; removing that layer requires
wiring full native program execution, and is not claimed complete here.
Rebuilt WASM passes 46 backend/scene/diagnostic/native-session tests, including
the direct empty-profile construction/extrusion/commit chain. Type checking
and packed-byte verification pass.

### Native ordered geometry graph execution

brep_graph executes an ordered geometry DAG through one native Session and
returns only an atomically committed bundle. It preflights contiguous identities,
backward references, per-node operand bounds and unique in-range outputs. Native
use counts preserve repeated references and release unretained geometry after
the last use. Outputs preserve requested order, including typed empty outcomes.
The brep_graph bridge operation exposes the same execution over WASM.

Twenty-eight native B-rep bridge tests pass. New graph tests cover empty reuse,
output ordering, malformed references/outputs, failure after a valid prefix,
and eight nodes executing under a one-result retained-byte budget through
liveness release. Retaining two results under that budget correctly fails.
This graph API does not yet admit the complete SemanticProgram envelope,
capability manifest, context identity or cancellation protocol, and the main
application executor still needs integration. It is not full Rust migration
or complete B-rep qualification.
The rebuilt WASM passes all 47 backend/scene/diagnostic/native-session tests,
including direct graph execution with ordered value/empty outputs and failure
after a valid prefix. Type checking and packed-byte verification pass.

Graph planner follow-up: removed the retained duplicate of every operand list.
After the validation/counting pass each bounded list is decoded when its node
executes, so additional planner storage is O(nodes + maximum node arity), not
O(nodes + total graph edges). The original decoded graph and geometry working
allocations remain outside this statement; it is not a peak-memory certificate.
All four native graph tests pass, including 512 repeated references and refusal
at 513. The rebuilt WASM passes 47 integration tests and packed-byte verification.

### Native graph failure provenance

execute_detailed returns GraphFailure with the original native error, active
node and completed-node count. The additive brep_graph_report ABI returns either
a committed set of outcomes or a failed diagnostic with no outcomes. Preflight
failures have zero completed nodes; geometry failures identify the active node
after the completed prefix. The existing brep_graph throwing API is preserved.

Five native graph tests pass, including malformed request, preflight reference
failure, geometry failure after a valid prefix and successful reporting. These
fields provide terminal execution diagnostics, not in-flight callbacks or
cancellation. Full SemanticProgram admission and application integration are
still outstanding.
Rebuilt WASM passes 48 integration tests, including structured graph failure
without partial outputs and a subsequent successful graph report. Type checking
and packed-byte verification pass.

### Native B-rep execution-plan admission

brep_execution_plan validates the B-rep v1.2 plan component: exact fields,
semantic-execution-v2 version, at most 25,000 nodes and complete authored order.
It refuses discarded legacy effects and non-null language terminals, as required
by the existing brep-1 contract. execute_plan admits this component before
running any graph node; the graph ABI accepts it through the execution field.
The lower-level graph API remains available without this semantic plan.

Thirty-two native B-rep bridge tests pass, including empty plans and refusal of
missing, duplicate, reordered, invalid and legacy plan entries. This admits
only the execution-plan component, not source identity, occurrences, capability
closure, the full envelope or the application executor replacement.
Rebuilt WASM passes 49 integration tests. An actual lowered OpenSCAD difference
program executes through its authored native plan and produces the same model
as the existing executor; reversed evaluation order is refused before execution.
Type checking and packed-byte verification pass.

### Native envelope component admission

brep_envelope now admits the SemanticProgram envelope components natively on
the graph/session path: the source identity descriptor (exact keys, lowercase
SHA-256, bounded UTF-8/UTF-16 lengths), the core literals (semantic-program-core
schema, identity version, exactly semantic.execution-v2, both language contracts
with their pinned semantics revisions, the capability graph version and the
fixed units sextet), declared capabilities and the capability closure. Schema
versions 1.0 and 1.1 refuse with the relowering-style message. The closure check
runs an exact Rust port of deriveSemanticCapabilityClosure — per-kind
capabilities, geometry kind/space/representation/evidence entries, transitions
and semantic.result.<tag> with dedup and raw UTF-8 byte sort — and requires
string-for-string equality. Admission is optional-but-strict: requests without
these fields execute unchanged, and every refusal precedes all node execution.
executeBrepNativeProgram transports the components through graph-begin without
adding host-side validation.

Seven native envelope tests pass, including a box + transform + Boolean union
closure matched string-for-string and every wrong-by-one closure variant; 156
native bridge tests pass in total. Rebuilt WASM passes 69 integration tests
across five files, including 17 executor-level admission tests that tamper one
field at a time and are refused natively before any geometry executes.
[Scoped evidence](../qualification/brep-native-envelope-admission-v1.json)
Type checking and packed-byte verification pass with 68 dist artifacts
(4,639,607 bytes). Occurrence-ID digest hashing, operations structural identity,
provenance/tessellationIntents/diagnostics admission, the full envelope as one
object and the application executor replacement remain open; this is scoped
evidence, not full completion.

### Native identity admission

brep_identity moved operation and occurrence identity verification behind the
WASM ABI. It ports identityDigest byte-exact — SHA-256 over big-endian
length-prefixed UTF-8 domain plus canonical payload bytes — together with
stableJson (raw-UTF-8 key order via BTreeMap iteration, JSON.stringify string
escaping, ECMAScript Number::toString replicated from the shortest round-trip
digits, including the 1e21 integer boundary, the 0.000001/1e-7 exponential
boundary and subnormals) and all four derivers (opv1:, ambv1:, occv1:,
entity:v2:). validate_operations admits the full host rules: positional IDs,
deterministic parent-before-child preorder with stack discipline, exact-once
structural path extension, re-derived operation IDs, unique paths and IDs,
dense-from-zero sibling child ordinals and ambiguity-group membership with
re-derived group IDs. validate_occurrence_identity admits the host identity
subset: positional IDs, canonical-first-row parent/staticParent references,
static operation instantiation, re-derived occurrence IDs, rows sharing an ID
describing the same logical evaluation, per-ID output rows dense from zero with
re-derived scene entity IDs and dynamic-slot duplicate-ordinal discipline. The
admission runs inside the existing envelope admission in prepare, after
execution-plan validation and before Runner::new; it stays optional-but-strict,
so requests without operations execute unchanged and every refusal precedes all
node execution. executeBrepNativeProgram transports core.operations through
graph-begin with zero new host-side validation logic; Cargo.lock gained sha2
0.11 (already pinned via rbench) and previously untracked geometry-bridge
dependencies, and cargo build --locked passes.

Seven native identity tests pass, with every expected digest pinned against the
TS derivers run in node and wrong-by-one mutations refused (bad parent order,
non-dense ordinals, wrong structural paths, tampered occurrenceId and
sceneEntityId, duplicate ordinals, ambiguity membership errors, unknown opv2:/
occv2:/entity:v3: prefixes); 163 native bridge tests pass in total. Rebuilt WASM
passes 79 integration tests across six files, including the new
brepIdentityAdmission suite where a real lowered difference program executes
and nine per-field tampers are refused natively with E_BREP_SEMANTIC_CONTRACT
before any node runs.
[Scoped evidence](../qualification/brep-native-identity-admission-v1.json)
Type checking and packed-byte verification pass with 68 dist artifacts
(4,657,872 bytes). Occurrence-production replay, $expansion continuation and
module-activation anchoring, provenance/tessellationIntents/diagnostics
admission, the full envelope as one object and the application executor
replacement remain open; this is scoped evidence, not full completion.

### Native provenance and diagnostics admission

brep_provenance moved the remaining transported envelope components behind the
WASM ABI. It ports the envelope-component subsets of validateSemanticProgramV1
and validateDiagnosticTemplates: provenance admits exact record keys, exactly
one source record per static operation in operation order, non-empty half-open
UTF-16 spans bounded by the source descriptor utf16CodeUnitLength and labels
bounded at 512 code units; tessellationIntents admits exact intent keys,
unique strictly ascending occurrence references in range of the transported
occurrences and targeting a producing (non-null node) occurrence,
positive-or-null chord and angular tolerances, segment counts null or integers
in [3, 1,000,000] with maxSegments never below minSegments, and refuses the
component entirely under the legacy/current contract; diagnostics admits
exactly one source-bound presentation per diagnostic template in template
order, messages bounded at 1,000,000 UTF-16 code units and nullable, possibly
empty, bounded spans; diagnosticTemplates admits positional IDs, the
^[A-Z][A-Z0-9_]{0,63}$ code pattern, the info/warning/error severity enum, the
v1.2 frozen LEGACY_LANGUAGE_ERROR error rule, in-range operation references
and argument discipline (≤128 entries, unique bounded names, full
validateIdentityValue checking reusing the identity port). Cross-field
consistency is enforced natively: provenance requires transported operations
and the source descriptor, non-empty intents require the language contract and
occurrences, and diagnostics require the transported templates. Admission runs
inside the existing envelope validation in prepare, after execution-plan
validation and before Runner::new, and stays optional-but-strict, so requests
without these components execute unchanged and every refusal precedes all node
execution. executeBrepNativeProgram transports program.provenance,
program.tessellationIntents and program.diagnostics through graph-begin with
zero new host-side validation logic; cargo build --locked passes unchanged.

Six native provenance tests pass, covering the full valid component set and
wrong-by-one mutations for every rule (provenance count/order/empty/reversed/
out-of-range spans and missing context, intent ordering/duplicates/out-of-range
and non-producing occurrences/non-positive tolerances/segment bounds/max-below-min/
legacy-contract refusal, diagnostics count/order/overlong messages/bad spans/
missing context, template IDs/code shapes/severities/the v1.2 error rule/
operation references/argument discipline); 169 native bridge tests pass in
total. Rebuilt WASM passes 90 integration tests across seven files, including
the new brepProvenanceAdmission suite where a real lowered difference program
executes with all envelope components transported and ten tampers refuse
natively with E_BREP_SEMANTIC_CONTRACT before any node runs.
[Scoped evidence](../qualification/brep-native-provenance-admission-v1.json)
Type checking and packed-byte verification pass with 68 dist artifacts
(4,661,642 bytes). The UTF-16 span surrogate-pair endpoint check stays
host-side because the source text never crosses the ABI; occurrence-production
replay, $expansion continuation and module-activation anchoring, the full
envelope as one object (envelope exact keys and the semantic-program-envelope
schema literal remain host-side) and the application executor replacement
remain open; this is scoped evidence, not full completion.

### Native envelope object and source attestation

brep_attestation closed the last transported-shape gaps of envelope
admission behind the WASM ABI. A present graph-begin envelope field is now
admitted natively as the complete SemanticProgramEnvelopeV1 object: exactly
the seven envelope keys, the semantic-program-envelope schema literal, a
supported 1.2 schemaVersion, and a core with exactly the fourteen validateCore
keys under the semantic-program-core literal. The chosen consistency semantics
is duplicates-must-match-exactly: every component carried both as a top-level
graph-begin field and inside the envelope must be deep-equal, components
carried only inside the envelope are adopted into admission, and execution
keeps consuming the top-level fields so the envelope can never silently
supersede what runs. Source attestation moved native as well: when the
request transports sourceText, the exact UTF-8 bytes are SHA-256-hashed and
measured against the admitted source descriptor (utf8ByteLength and the
UTF-16 code-unit length computed from the decoded string, both bounded by the
250,000-unit and 4,000,000-byte source limits), the routing header is parsed
by a faithful port of parseGeometrySourceRoutingHeader (CR/LF/CRLF line
splitting, block-comment/string masking with escape tracking, the JavaScript
\s whitespace set, header-placement and duplication rules, capability
identifier syntax and the 32-capability budget) and matched against
language.contract and declaredCapabilities, and the surrogate-pair
span-endpoint check that previously stayed host-side now refuses any
provenance or diagnostics span endpoint falling between a high and a low
surrogate while admitting pair-edge and text-end boundaries. Without
sourceText the behavior is unchanged: descriptor-only admission, surrogate
check skipped, optional-but-strict. All of it runs inside
brep_envelope::validate in prepare, after execution-plan validation and
before Runner::new, so every refusal precedes all node execution.
SemanticLoweringSuccess now retains the exact lowerer input as sourceText and
executeBrepNativeProgram transports program as the envelope object plus
sourceText through graph-begin with zero added host-side validation logic;
cargo build --locked passes unchanged.

Nine native attestation tests pass, covering a valid unified envelope with a
real source text (digest computed from the actual bytes), envelope key/literal/
version/core-key mutations, valid-but-different envelope components, envelope
nodes mismatches, an adopted envelope-only forged provenance row, one-flipped-
hex-character digests, utf8/utf16 lengths off by one in both directions,
missing attestation context, mid-surrogate provenance and diagnostics span
endpoints (empty diagnostic spans included) versus pair edges and the text-end
boundary, and routing-header contract/capability/unsupported/duplicated/
engine/malformed/after-body mutations plus @requires deduplication; 178
native bridge tests pass in total. Rebuilt WASM passes 95 integration tests
across eight files, including the new brepSourceAttestation suite where a real
lowered difference program executes with the envelope object and source text
transported, span endpoints at emoji pair edges admit and execute, and
tampered source text, tampered descriptors and a mid-surrogate provenance span
refuse with E_BREP_SEMANTIC_CONTRACT before any node runs.
[Scoped evidence](../qualification/brep-native-source-attestation-v1.json)
Type checking and packed-byte verification pass with 68 dist artifacts
(4,667,137 bytes). Occurrence-production replay, $expansion continuation and
module-activation anchoring, and the application executor replacement remain
open; this is scoped evidence, not full completion.

### Native occurrence-production replay

The last host-only admission logic of validateOccurrences/validateCore moved
behind the WASM ABI. brep_production.rs ports validateOccurrenceProduction
(semanticProgramValidator.ts l.1381) for the brep-1 reachable path: logical
occurrences must be exactly one zero/frame row or dense output rows, runtime
groups follow parent-before-child order, compiler-frontier frames expand with
directChild discipline under the snapshot-value budget, and every occurrence
group closes its frozen v1 production rule — primitive kind tables,
transform/projection/offset maps with single-input frontier consumption and
field-equal common parameters (rotate bucketed per value space),
preserving-alias, transparent, boolean/hull one-item aliasing and n-ary
ordered-frontier reductions, difference with compiler $body buckets exactly
partitioning the ordered frontier plus canonical base/cutter union reducers
and the bucket-rebuilt schedule, and linear/rotate extrusion with the
canonical internal union reduction. Every DAG node must have exactly one
recomputed materializer group, the authenticated group/node schedule replay
must equal the node IDs exactly, the synthetic program frontier must equal
the transported result row-by-row, and every node-bearing group must be
reachable from a root production path. brep_identity.rs now also ports the
staticParent runtime ancestor-chain walk, moduleActivationAnchor (l.600) and
isChildrenExpansionContinuation (l.669) with the shared one-million-step
proof budget, and same_dynamic_slots was aligned to key on slot.value exactly
as the host does. The replay runs in brep_envelope::validate after
identity/provenance admission and before Runner::new whenever non-empty
occurrences are transported (the result and execution fields are required
context); TS adds zero validation logic.

Thirteen new native tests replay seven real fixtures dumped from the TS
lowerer and admitted by the host validator (difference, union-with-transform,
hull, children()/$expansion, module, nested transforms, multi-output union):
all admit, and wrong-by-one mutations refuse — zero/two materializer groups
per node, broken transform-map chains, bad boolean operand structure, hull
reduction mismatches, misplaced output ordinals, surgical re-digested
$expansion-without-anchor and anchored-row-missing-continuation rows,
structural expansion tampering, result-frontier mismatches and defensive
refusal of interrupted terminal/discarded-effects artifacts; 191 native
bridge tests pass in total. Rebuilt WASM passes 108 integration tests across
nine files including the new brepProductionAdmission suite, where real
lowered difference, union-with-transform and children() programs execute
natively, a real lowered hull program passes admission and is refused only
downstream by the pre-existing hull evaluation gap
(E_BREP_SEMANTIC_UNSUPPORTED), and nine production tampers refuse with
E_BREP_SEMANTIC_CONTRACT before any node runs.
[Scoped evidence](../qualification/brep-native-production-replay-v1.json)
Type checking and packed-byte verification pass with 68 dist artifacts
(4,684,492 bytes; the kernel chunk budget was raised to 2,400,000 for the
~16 kB packed replay/anchoring addition). The interrupted-evaluation replay
branches (active difference reconstruction, partial maps, completeReduction,
terminal prefix equality) remain host-side because execution-plan admission
refuses non-null terminals and non-empty discardedEffects before the replay,
and the application executor replacement remains open; this is scoped
evidence, not full completion.
