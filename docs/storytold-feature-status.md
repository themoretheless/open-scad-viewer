# Storytold features — implementation status

The current compact interface is retained. Geometry runs in Rust; TypeScript handles document identities, worker transport, controls and export layout.

## Available

- Source-linked grid XY, radial and mirror body patterns. Changes to source geometry regenerate placements and retain B-rep. Generated copies have stable IDs. Editing individual pattern members requires detaching the complete pattern. Limit: 256 placements per source and 1000 linked instances per document, within document memory limits.
- Image Trace: PNG/JPEG/WebP, dark-ink/alpha silhouette or 2–16 color palette, physical width, threshold, resolution and simplification tolerance. Color mode uses native RGB quantization and per-color boundaries; near-white background suppression and minimum region area are configurable. Traced sketch colors survive document save and are inherited by a single-color extrusion. Native SVG rasterization and contour extraction return editable boundaries. Hole boundaries are separate clockwise contours; the import is one Undo step. Main viewport sketch tools retain authored paths; Solid workspace exposes polygon nodes.
- Drawing sheets: A5/A4/A3/A2/Letter, landscape or portrait, explicit scale, title/author, overall projected dimensions, assembly sheet or one sheet per body, sheet preview, individual SVG downloads and multipage vector PDF. Native SVG projection retains line, polynomial cubic and elliptic-conic paths from B-rep; other admitted curves use a checked 0.01 mm tessellation. Hidden edges can be shown, hidden or dashed; visibility uses the display mesh with a 0.02 mm uncertainty band, not exact surface occlusion. Dimensions are projected mesh bounds, not constraint-linked engineering dimensions. PDF embeds the bundled Noto Sans font and Unicode mappings: labels are searchable and selectable. SVG conics become cubic paths in the PDF parser. Limits: 40 sheets, 100000 mesh triangles, 20000 projected fragments and two million visibility tests per view. Native PDF caps combined SVG input at 16 MiB, output at 32 MiB and path segments at one million; unsupported paint effects are refused.
- Atomic action batches in UI and MCP. UI commits the final validated result once; failures publish nothing. Recording stores one initial document checkpoint followed by entity edit commands with source preconditions, including creation, modification, deletion, metadata and list order. This records document operations, rather than pointer gestures or the original parameterized modelling command for every tool. Records can be saved, loaded and replayed. Limits: 80 actions and 16 MiB per script. MCP `openscad_direct_transaction` executes in a disposable worker, honours cancellation and a 60-second deadline, and returns a document; it does not mutate the live UI or catalog automatically.
- Assembly/camera keyframes: relative translations, X/Y/Z Euler rotation, uniform scaling and orbit yaw/pitch. Native shortest-angle interpolation; up to 16 independent body tracks, 64 frames, 120 seconds. Preview preserves the source document. JSON export/import includes animation data; WebM records the WebGPU view in real time, with a short final-frame hold and maximum output dimension 1280 pixels. Playback updates geometry at approximately 8 Hz; video capture targets 15 fps. A compatible browser with active WebGPU and MediaRecorder is required for WebM. Offline export samples a uniform timeline including both endpoints (effective FPS recorded in the manifest), uses native opaque mesh projection with deterministic painter ordering, and writes 960 × 640 PNG frames plus timestamps to ZIP. All frames use shared projected bounds; it is not a WebGPU shaded render. Limits: 300 frames, 200 bodies and 50000 mesh triangles per frame, 32 MiB projected SVG and 64 MiB raster archive. Intersecting/transparent geometry has painter-rendering limitations; export supports cancellation.
- Contour cleanup: open paths preserve endpoints; closed source paths use native simplification and even-odd arrangement normalization. Already retained profiles use native bounded curve tessellation and simplification, each spending half the requested deviation, followed by closed region normalization. The original sketch/curves are preserved and cleanup creates a new result. Cleaned retained profiles intentionally become polylines at the requested tolerance. Native profile limits still apply.

## Exact profiles and resource limits

- Authored Bézier extrusion preserves cubic control points in B-rep edges and cap trims. Native admission supports up to 32 closed mixed boundaries, each with 3–128 line/cubic/circular rational quadratic spans, within the shared 1024-face budget. Each retained span must have a proved injective coordinate. Source normalization can split stationary points and resolve self-nodes before this admission. Adjacent joins use either separated control hulls or a complete algebraic census proving only the exact shared endpoint; nonadjacent spans require a complete contact census with no intersections. Adjacent hull separation includes adaptive candidate axes checked with outward-rounded bounds. Holes and nested islands use convex containment where available, or native interval separation and bounded winding decisions for concave boundaries; separate curves with overlapping bounds can be admitted when their boundaries are proved disjoint. Self-intersections with a complete arrangement can be normalized; unresolved or tolerance-ambiguous relationships are refused. Display tessellation does not replace retained curves. Native polynomial Bézier union/intersection/difference now resolve transverse curve intersections, trim original curves and assemble validated material-left boundaries. Identical spans are deduplicated. Mixed cubic/circular rational operands retain both original carriers. Supporting-plane endpoint tangencies admit separate external components; holes touching their containing boundary are refused. Partial coincidence is admitted only when an intersection overlap has an independently matching retained Bernstein carrier; straight and curved partial overlap, including projective parameter maps, have native regression coverage. Self-intersections can be normalized with even-odd material: native exact stationary roots and intersections split source spans, side probes require normal-coordinate monotonicity and separation from all other complete curve hulls, and angular traversal produces validated closed components. Tolerance-ambiguous contacts, unproved overlap identities, unresolved events, nonmanifold XOR contacts and unresolved singular ordering are refused. This is bounded numerical geometry, not an unlimited certified general Boolean implementation.
- Outline Stroke no longer has the old 256-span UI/profile barrier. Native planar profiles admit up to 1022 spans, subject to 2048 arrangement fragments and the existing 1024-face solid budget (including caps). These are bounded resource limits, not an unlimited-contour promise.

## Action script example

```json
{
  "version": 1,
  "actions": [
    {"kind":"primitive","options":{"kind":"box","id":"part","name":"Part","size":10,"topRadius":1,"innerRadius":1,"round":"exact","segments":4}},
    {"kind":"pattern","sourceId":"part","pattern":{"kind":"grid","rows":2,"columns":3,"spacing":[20,20]}}
  ]
}
```

Supported action kinds: `primitive`, `sceneEdit`, `pattern`, `sketchCleanup`, `outlineStroke`, `extrusion`, `documentEdit`, `checkpoint`. The caller supplies the source document separately. A checkpoint contains its serialized direct document as a string. A cleanup action provides `sourceId`, `createdId` and `tolerance` in millimetres. An `outlineStroke` action provides `sourceId`, `createdId` and `width`; an `extrusion` action provides the same `options` accepted by direct extrusion. Both participate in atomic validation and rollback.

Previous milestone verification (2026-10-08, before the six expansions above): the complete WASM build succeeded, including optimized geometry output. Production bundling and artifact verification passed: 8,024,916 asset bytes plus 14,270,887 raw WASM bytes. Six optional feature panels load separately; explicit size budgets were updated to the measured integrated feature set with bounded headroom. The 14 feature/worker/MCP suites passed 118 checks; focused native compound Bézier, prism and vector PDF checks passed. GPU fallback tests passed 7 checks, including preservation of a healthy device when transparent preview sorting exhausts its budget. Such a preview is temporarily shown opaque with a visible status; a new scene upload restores transparency. The native compound proof produced two solids, 16 faces, 24 retained cubic edges and a closed display mesh. A two-page A4 PDF was checked with zero embedded images and both rendered pages inspected, including Cyrillic labels. Browser checks cover compound extrusion and the native PDF worker. The fresh full Solid UI regression run passed 258 checks; Vue and MCP type checking and whitespace checks passed. Earlier baseline verification passed 840 B-rep checks (3 ignored); that native count describes the preceding milestone, not a fresh full native run of this continuation. The extrusion shortcut consumes its key before focusing the height field. In-app browser Blob downloads are unsupported; actual PDF file generation and rendering were verified independently through the same native exporter.


Current expansion verification (2026-10-08): the complete WASM build succeeded; geometry optimized from 12,808,029 to 11,350,574 bytes. Production bundling and artifact verification passed for 152 files: 8,069,328 asset bytes plus 14,352,357 raw WASM bytes. Measured feature budgets retain bounded headroom (geometry chunk 3,829,162 / 3,840,000 bytes; CAD worker within 148,000; aggregate within 8,080,000).

The B-rep regression run passed 833 tests, with 3 existing ignored diagnostics and 14 unrelated `spatial_rmf_qualification` tests excluded. All 155 planar-geometry tests passed. Focused native checks passed for original cubic/conic projection and hidden-line classification (2), searchable vector PDF (2), palette tracing (1), and retained-profile cleanup (1). Fifteen feature/worker/MCP suites passed 97 checks; the full Solid/main UI run passed 263 checks, followed by a fresh 8-check main UI run after the recording serializer fix. Vue and MCP type checks and scoped whitespace checks passed. The recorder regression verifies creation, edits, removal, ordering, source conflicts, rollback and typed mesh serialization.

Browser verification on the rebuilt artifact covers the two-color PNG import (red outer boundary, red hole and blue region, white removed), one-step Undo/Redo with the original B-rep body preserved, drawing preview/SVG/PDF preparation, and all-axis animation PNG archive preparation. The actual PNG preview decoded to 960 × 640. The native fixed frame proof has six frames at 4 FPS, exact timestamps 0–1.25 s, shared bounds, and a ZIP checked for CRC correctness. The two-page PDF contains no page images; its embedded CID TrueType font has Unicode mappings, Cyrillic text was extracted with `pdftotext`, and both rendered pages were inspected. Visibility subdivision merges equal-state spans back into source trims: an ordinary box exports 12 source edges per view rather than thousands of short fragments. In-app browser Blob downloads remain unsupported; generated PDF and native frame archive files were verified independently through the same export functions.

## Bézier and drawing continuation (2026-10-08)

- `brep_profile_normalize` exposes retained even-odd self-intersection splitting; authored Bézier profiles attempt this after simple-wire admission fails. It refuses uncertain cusp/injectivity, carrier-overlap and material-side decisions. Curve decisions stay native; source curves are trimmed rather than replaced by sampled polylines. A 100,000-cell side-separation budget bounds probe admission.
- Drawings offer explicit `mesh` and `brep` occlusion modes. B-rep mode uses affine planar retained surface charts and authored UV trim regions including holes, independent of display triangles. Exact homogeneous projection independence excludes edge-on curved walls. Unsupported curved surface charts are refused explicitly; there is no automatic mesh fallback in this mode. Projection clipping retains original edge trims and a 0.00001 mm unresolved transition band (uncertain terminal spans stay visible). This is bounded numerical surface occlusion, not complete exact-arithmetic hidden-line removal for arbitrary NURBS surfaces.
- Linked horizontal/vertical dimensions reference a body ID, two B-rep vertex indices, view and connectivity signature. Native projection recomputes their endpoints and values from the current model; missing bodies, collapsed projections or changed connectivity require reattachment. This guards indexed topology, not a general semantic topological-naming system. At most four linked dimensions per view and 64 definitions overall; extra margins preserve sheet fit. UI vertex numbers help choose references; definitions can be saved/loaded as JSON (1 MiB cap). References are separate export settings, not persistent sketch constraints inside the model document.
- Detail and assembly title blocks provide drawing number, revision, author, scale, sheet count and material/part count. Existing page sizes and orientations remain available; the engineering title block needs at least 196 mm page width (landscape A5 or A4 and larger). These are compact engineering layouts, without a claim of formal GOST/ISO compliance or a generated bill of materials.

## Surface occlusion continuation (2026-10-13)

The B-rep drawing mode now admits curved retained surface charts through native `surface_occlusion`. Uniform parametric Krawczyk inclusion and contraction enclose roots for the complete projected curve hull, with an independent strict depth gap and authored UV trim classification. Original-span interval jets retain rational weights and knot rectangles. Unresolved cells remain explicit and trigger curve subdivision; they do not become empty/visible proofs. The existing physical transition band and global work/fragment limits remain.

Full-multiplicity chart boundaries have a separate one-dimensional uniform root proof. Rational extrusion charts additionally admit closed-rectangle covers from exact coordinate independence, endpoint bracketing and positive-weight depth hulls; this avoids artificial subdivision seams and keeps ordinary cylinder drawings fast. Face-domain helpers establish membership in exact authored linear boundary segments or four-sided closed rectangles without holes. They never enlarge general trim tolerance bands. The affine fast path and explicit mesh mode remain available; curved B-rep mode does not silently use display triangles.

Verified native continuation: 4 surface-kernel tests, 7 drawing-projection tests and 5 face-domain/intersection tests passed. The real cylinder case checks all four vertical edges of a separate body behind the retained wall, rather than merely finding any hidden cylinder edge. The projection suite completed in 0.47 s. Vue type checking and scoped whitespace validation passed. The final WASM/browser/PDF and broader goal completion are not yet established.

The original goal remains open:
- Broaden real-body coverage of general curved charts (including sphere/trimmed charts) and verify the packaged WASM/browser path.
- Finish internal/higher-order Bézier contacts, unresolved complex self-intersections and retained coincidence handling.
- Verify the implemented durable topology bindings in the packaged browser/WASM path, including ambiguity handling.
- Verify the implemented engineering specification/parts-list pages through native PDF and browser rendering.
- Run final integration/type/build checks and visually inspect browser drawing/PDF results.

## Durable dimensions and specification sheets (2026-10-13)

New UI dimensions store opaque vertex IDs anchored to the document body ID. Native `VertexReferenceIndex` reads only the model's validated authoritative ChangeSet, follows unique live successors through modifications and merges, and detects ambiguity even when one split branch preserves the original ID. It never uses nearest geometry or an old index as a fallback. Removed bodies/vertices, missing history, multiple live successors and exhausted work require explicit reattachment. Legacy index/signature recipes retain their original connectivity guard. Native projection returns resolved indices, current IDs and reference statuses; the UI can remove/recreate an individual dangling dimension without clearing all other bindings. JSON export preserves the durable IDs.

Native verification: 4 reference-index tests and 9 drawing-projection tests passed, including a real profile edge subdivision, vertex-table permutation, scaling, split ambiguity and deletion. New host integration cases are saved for the final rebuilt WASM; they have not yet been run against that artifact.

Optional specification pages list position, designation, description, quantity, material and note. Only explicitly matching designations group occurrences; conflicting descriptions/materials/notes are rejected, and unnamed bodies stay distinct even when their display names match. Quantities count the selected authored bodies. Physical material and shape equivalence are not inferred from viewport meshes/materials. Metadata can be edited per body and saved/loaded separately as JSON. Long Unicode fields and explicit line breaks wrap into variable-height rows; all rows are paginated without dropping bodies, with a 40-sheet combined drawing/specification cap. Drawings and specification sheets share final page numbering and the existing vector/searchable PDF path. These remain generic engineering layouts rather than a formal GOST/ISO compliance claim.

Specification-layout verification: 4 host tests passed for grouping/quantity, conflicting and dangling metadata, multipage coverage/escaping, line breaks and bounded format/page errors. Vue type checking passed. Native PDF rendering and real-browser verification remain part of the open final gate.

## Scalar contacts and curved drawing qualification (2026-10-14)

The exact-expansion scalar root adapter is now connected to profile Boolean splitting, simple-wire validation, and even-odd self-intersection normalization. Its admitted reductions are line/carrier intersections and pairs with an exactly common monotone coordinate (including reversed and globally rescaled weight definitions). Complete scalar root counts retain multiplicity; indeterminate predicates request the general intersection driver instead of becoming disjoint reports. Third-order interior contacts retain the original cubic trims through union/intersection/difference, including finite-area cusp regions. Straight partial overlaps also deduplicate source carriers of different degrees. This does not establish general curved partial-coincidence identities, arbitrary two-parameter cubic tangencies, singular self-intersection ordering, or manifold results for every contact.

Curved B-rep hidden-line qualification now includes an actual eight-chart rational sphere and a curved chart with an authored UV hole. Overlapping uniform-root proof boxes remain within original knot rectangles and remove artificial subdivision seams. An outward-rounded homogeneous Bernstein ball-residual bound excludes front spherical carrier arcs without reconstructing a replacement surface. Retained boundary curves provide an additional sufficient coverage proof for collinear projected rays, with endpoint bracketing, positive-weight depth hulls and the global work budget. Distant original positive-weight control hulls are excluded before interval jets. These are sufficient bounded proofs; unresolved geometry still refines or refuses, and B-rep mode never switches to a display mesh.

Native checks passed: 3 exact scalar predicate tests, 3 scalar adapter tests, 16 focused profile tests, 5 surface-occlusion kernel tests, and 11 projection tests. The profile cases include original cubic/rational carriers, even-odd crossing normalization, third-order contact areas, and different-degree straight overlap. The sphere case checks separate hidden geometry in top view and complete front/right projections; after the additional boundary/front-branch proofs it completes in about 9 seconds in the native diagnostic run. The wider B-rep run passed 874 tests, with 3 existing ignored checks and 17 separately qualified spatial-RMF tests filtered out.

Browser checks on the rebuilt first continuation artifact verified the current Solid interface, B-rep drawing preparation, durable vertex-dimension creation, specification preview and worker PDF preparation. A three-page vector PDF was independently rendered and all pages inspected: two detail sheets, a linked 20 mm dimension, one grouped specification row with quantity 2, Cyrillic searchable labels and shared page numbering. Final packaging/host checks of the subsequent sphere-performance corrections are recorded below when complete.

Final continuation artifact verification: the complete WASM build passed, followed by a geometry-only rebuild for the final curved-boundary performance fixes (13,345,506 -> 11,812,663 bytes after optimization). Six host/worker suites passed all 81 checks against that final artifact, including third-order profile Boolean areas, durable dimension reordering/scaling/ambiguity, real sphere drawings in all three views, specification quantities/page numbering, and native vector PDF. Browser drawing/dimension/specification/PDF preparation was repeated after reloading the final artifact. The three-page PDF was regenerated and visually inspected again from that final artifact.

Production bundling passed. The measured geometry JS chunk is 3,883,867 bytes; its explicit budget is now 3,900,000. Artifact verification is still not globally green: the independently changed renderer chunk measures 135,704 bytes against its existing 132,000-byte budget. That renderer budget was not changed in this continuation. Vue/MCP type checks and scoped whitespace checks passed; this is not a claim that every complex Bézier degeneracy or arbitrary trimmed NURBS drawing is solved.

## Curved coincidence and degenerate Bézier arrangements (2026-10-14)

Retained planar overlap admission now covers partial curved subspans, reversed traversal, degree elevation, positive homogeneous weight scaling and positive projective reparameterization. Candidate parameter maps are admitted by outward-rounded homogeneous Bernstein difference coefficients and positive denominator lower bounds, never by endpoint agreement or display samples alone. Coalescence is limited to `min(4096 * EPSILON * coordinate_scale, 0.001 * geometry_tolerance)` on the represented trims. This is numerical carrier agreement at roundoff scale, not an unlimited symbolic identity oracle. The public NURBS helper admits clamped positive-weight spans of degree 1–3; profile admission still supports its existing polynomial cubics, lines and circular rational quadratics.

Boolean splitting synchronizes cuts across admitted shared carriers before selecting or deduplicating pieces. Even-odd normalization groups matching fragments and cancels even multiplicities, including reversed/retraced subspans, threefold passes and a nonuniformly parameterized straight cubic against a degree-one line. Exact constant cubic passes have no material. A partial shared cubic boundary can now be processed both by union/intersection/difference and by even-odd arrangement normalization without replacing it with a polygon.

Exact-expansion stationary roots now split cusps using the current represented source definition. Multiple stationary roots retain their multiplicity; unresolved predicates cannot become empty cut lists. A shared cut uses a common source endpoint observation. Tiny endpoint-control coordinate noise is conformed only within `min(512 * EPSILON * scale, 0.001 * geometry_tolerance)` in outward-bounded L1 displacement per represented trim control, with original weights unchanged. Positive Bernstein weights bound the complete curve displacement by that control bound. This explicit roundoff conformance is needed for non-dyadic cusps; the authored input document remains unchanged and the geometry status remains `numerical_uncertified`.

Zero first derivatives use the first nonzero endpoint control jet. Equal tangent continuations are resolved by interval-enclosed first exits from a common local box, then strict interval cross/dot signs establish clockwise precedence. No sampled-angle tie breaker is used. Exact source-coordinate orientation predicates additionally admit oblique supporting-line endpoint contacts. Successor bijection, closed wires, component separation, manifold restrictions and explicit resource refusals remain enforced. The shared-cut pool is capped at 8192 observations, cut projection and departure ordering each at 100000 work units, with the existing 1022-source-span/2048-fragment limits.


The overlap adapter now also admits source spans that are injective on different coordinates. Endpoint ownership proposes the shared parameter ranges and the complete homogeneous coefficient bound admits them. Both complete spans must have an injective coordinate. All pairs of their outside pieces receive the general intersection audit, preserving additional isolated nodal contacts outside the shared carrier rather than reporting only the overlap. An unsplit non-injective source still requests the general intersection path.

Degenerate branch ordering uses a common rectangle with several aspect ratios and either certified monotone coordinate for each branch. Strict prefix interval bounds establish the first exit. Exact supporting half-plane orientation also applies during final separation of components, including rotated zero-jet cusps. A genuine two-carrier curved lens is subdivided on its original carrier to satisfy the existing three-span cap convention; it is never replaced by a closing chord. Emitted arrangement spans remain capped at 2048.


Final component separation now routes reducible planar pairs through the same exact scalar adapter as arrangement splitting before falling back to the general two-parameter driver. This prevents a generic overlap report for two disjoint oblique line carriers from rejecting an otherwise valid rotated cusp arrangement. The 8192-pair separation budget includes these scalar decisions. A targeted diagnostic build of the current profile and planar adapters passed all 39 cases, including the rotated/translated cusp and the independent-axis curved hole with retained extrusion and a 400-point independent material oracle; final packaged verification is recorded separately.


Final native profile qualification passed all 29 cases. The 40 CAD predicate tests passed with the same unchanged owned predicate source; the diagnostic profile/planar-adapter build passed all 39 cases. The supplemental wider run used a temporary test-only `profile.test.package.nurbs-core.opt-level=0` override in an isolated target directory. It was stopped after 884 passes and two failures to release resources for packaging. `aag_features::tests::pocket_corpus_recall_and_hierarchy` subsequently passed alone; `nested_pockets_form_a_hierarchy` repeatedly returned `NURBS_RESOURCE_LIMIT` from its 60-second wall-clock pocket-recognition budget. No source limits or test deadlines were relaxed. This is not a complete green wider B-rep qualification.


The fresh Cargo-produced WASM was independently instantiated through the production binary value codec while Binaryen optimization was still running. Direct ABI smoke checks passed for the 1/2 and 1/3 interior cusps, the rotated zero-jet two-component contact, and union/intersection/difference with independent-axis curved coincidence and its two-carrier hole. This checks the newly compiled WASM itself; packaged-host qualification still uses the final optimized artifact.


Final packaged qualification: the standard geometry build and unchanged Binaryen 116 `-Oz` recipe completed successfully, reducing the fresh WASM from 13,582,576 to 12,022,663 bytes. All nine owned native source hashes remained unchanged through compilation and packaging. Seven application/worker suites passed all 87 tests against that final package, including six degeneracy cases covering rotated contacts and independently monotone coincidence. The Vue type check and scoped whitespace checks passed. Temporary optimizer experiments were stopped; no optimizer recipe change was made.


Production bundling passed. The final geometry JS chunk measures 3,953,958 bytes; its explicit budget is 3,970,000, retaining 16,042 bytes of headroom. Independent production checks passed for the packed geometry payload, the matching 12,022,663-byte streamed WASM, and uniqueness of packed artifacts. Generated, public and production geometry WASM bytes are identical. The global distribution size gate remains incomplete: it stops at `MainModelingTools` (107,258 > 107,000 bytes), and the independently changed renderer also exceeds its existing limit (137,576 > 132,000 bytes). Neither unrelated chunk budget was changed.

## Proper planar cubic singularities and curved joints (2026-10-14)

The planar adapter now has a general moving-line implicit reduction for proper degree-two/three rational carriers. An exact fraction-free matrix kernel constructs a linear rational inverse, checked against the original homogeneous source coefficients. A coprime exceptional fiber establishes that the inverse is valid away from the carrier's singular point. Substitution of the second carrier produces a polynomial of degree at most nine. The existing exact Sturm/GCD machinery counts all distinct roots and their multiplicities; exact algebraic sign queries filter parameters outside the finite source span and enclose both retained parameters. The exceptional fiber preserves both preimages of a node. A cusp with only one preimage is not reported as a second branch. Coincidence of implicit carriers alone does not authorize overlap: the existing finite-range, homogeneous-coefficient agreement admission still owns that decision.

Adjacent curved spans whose control hulls overlap can now be admitted by a complete algebraic census proving that their only contact is the exact shared endpoint. This includes same-side third-order cusp lenses with different nonlinear parameter maps. Difference can retain externally touching material components; hole/parent contact restrictions and the previous XOR refusal remain. No closing chord or polygon replacement is introduced.

Normalization keeps each stationary piece's authored parent and uses its original definition for contact and self-node decisions. If a source has an injective coordinate, only that coordinate's stationary parameters need splitting; unrelated coordinate extrema no longer perturb exact curved-contact identities. A source lacking an injective coordinate still splits at the complete coordinate stationary census. Endpoint/control conformance remains explicitly bounded as above.

Cut coalescence now measures an outward enclosure of the original source image, rather than an absolute parameter difference. Its L1 bound is `min(4096 * EPSILON * coordinate_scale, 0.001 * geometry_tolerance)` and both original domain endpoints are preserved. Coordinate inversion chooses the smallest geometric residual across usable axes. The scalar contact parameter enclosure is at most eight machine epsilons (or the stricter conditioning-derived bound). Normalization tangents are measured in a unit parameter domain; clamped reversal and scalar endpoint mapping preserve both domain endpoints exactly. Regressions include domains `1e-15`, `1` and `1e8`.

These remain bounded numerical profile operations, with the existing `numerical_uncertified` status. Nonprimitive/unsupported definitions, unseparated root boxes, excessive expansion growth, or exhausted work/precision limits return an explicit unresolved decision. A failed root census never becomes an empty contact list. Native and packaged qualification for this revision is recorded after the running builds finish.

Root transport now checks outward images of the original unit-domain carrier against the represented parameter observation. A coarse knot domain cannot silently turn a meaningful interior contact into an endpoint or collapse distinct interior events. Endpoint coalescence must fit the explicit roundoff geometry bound. The regression for a one-ULP knot domain refuses the operation, while the existing transverse cubic Boolean regression remains admitted.

## Cusp mesh and final qualification (2026-10-15)

Distinct curved cusp branches can share a display chord on equal parameter grids.
The mesher now refines their shared edge schedules using interval departure-order
bounds, including anisotropic cap coordinates, and propagates counts across ruled
faces. It preserves authored geometry and existing vertex/triangle budgets. The
source mesher diagnostic passed all 15 registry tests. A fresh standard WASM build
passed the real cusp extrusion/mesh smoke tests at all seven subdivision levels,
followed by all 95 profile/drawing/specification/worker tests on the final package.

Production bundling and the unchanged distribution verifier are green (171
artifacts, geometry JS 3,699,795 bytes, entry 234,016 bytes). Generated, public and
production geometry WASM hashes match: `6ce47a0cf18b241ba3a39d09f9fa0cabf8371a0228aca89e90534f0bdb347f0f`.
The production browser imported the retained cusp profile and its 37.8 mm³ BRep
body, displayed the actual triangle mesh with smooth preview disabled, and had
no warning/error console entries. Source/control definitions were not moved.
This qualifies the admitted bounded numerical operations described above; it does
not replace their explicit precision/resource/unsupported-definition refusals.
Evidence and the saved test project are in
`docs/qualification/storytold-bezier-2026-10-14/`.

Fresh Vue/MCP type checks and scoped formatting/whitespace validation passed after the final browser check.


## 2026-10-15 — checkable Rust profile certificates

General polynomial/rational quadratics and rational cubics now retain their
original controls through profile validation, Booleans and extrusion. Affine
ellipses are supported; rational Green integration uses outward error bounds.
Exact root reduction cancels verified common homogeneous polynomial factors.

A separate exact-rational Rust verifier checks source-bit-bound portable
certificates for closure, injectivity, the full pair census, orientation and
nesting, including algebraic same-side cusp joints. Native standalone and
WASM verification agree. Unknown, stale, incomplete or false proofs refuse.
This certifies represented profile topology, not construction history,
reported numerical area or arbitrary 3D solids. Global geometryStatus remains
numerical_uncertified; callers request and verify the scoped certificate.

Qualification: [2026-10-15 report](qualification/bezier-certificates-2026-10-15/README.md).
104 predicate tests, 49 targeted native profile/certificate/transform tests,
6 native bridge tests and 98 packaged host tests pass. The corrected compression
bootstrap passes 26 additional host checks. Production verification passes
173 artifacts without changing size budgets. Browser display/recovery of the
rational ellipse extrusion was inspected with smooth B-rep preview disabled,
at about 120 FPS and no console warnings/errors.

The broader native profile filter completed 80 tests; its one remaining
unrelated closed spatial RMF mode matrix was explicitly stopped. No full
workspace or full sweep-matrix pass is claimed.
