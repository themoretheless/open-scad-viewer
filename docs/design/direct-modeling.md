# Direct modeling — first supported workflow

Open **Прямое моделирование / Direct modeling** in the editor toolbar. The workspace fills the available screen: 2D sketches remain on the left and 3D bodies on the right. Drag the divider to resize (keyboard arrows or Home also work). Each pane has independent zoom, pan and Fit; wheel zooms and Shift/right/middle-drag pans. Contextual editing controls sit below both panes, with object lists below their respective canvases. There are no 2D/3D tabs or modal dialog.

- Draw rectangles and 64-vertex polygonal circles by dragging; create open or closed polylines by clicking and Finish / Close contour.
- Select and drag a contour or its vertices. Apply XY translation, Z-axis rotation and uniform scaling without a constraint solver.
- Extrude a closed contour into a baked triangle mesh. The source sketch remains a separate object: subsequent edits/deletion do not alter the body, and body edits do not alter the sketch.
- Select bodies in the list or isometric preview; drag in XY, enter a Z offset, rotate about Z and scale uniformly. Every completed gesture/action is one undo step. Ctrl/Cmd+Z, Shift+Ctrl/Cmd+Z and Ctrl/Cmd+Y operate within this mode; Escape cancels an unfinished drawing/drag.
- Save/reopen the independent document as JSON. Current geometry autosaves separately from the source editor in `scad-direct-modeler-v1`; undo history is session-local (80 snapshots, approximately 16 MB retention budget). A detected concurrent tab write stops autosaving and asks for a JSON backup. localStorage writes are not a transactional cross-tab lock.
- Export selected bodies as STL, all bodies as SCAD, or append baked polyhedra to an OpenSCAD scene. ModelGraph sources use file export, not SCAD append. Returning to direct mode preserves its independent geometry.

This first implementation uses an SVG isometric preview and the existing Rust polygon kernel. It is a direct sketch/extrusion/body-transform workflow, not face push/pull, general sculpting or a replacement for the main viewport. Circles are sampled, not analytic arcs. Sketches have no inferred constraints or references to bodies. Current limits: 200 total objects, 512 points per sketch, 4 MB JSON documents; invalid or self-intersecting extrusion profiles are rejected by the kernel.

Verification: `tests/directModeling.test.ts` covers concave closed extrusion and volume, sketch/body independence, history branching and snapshot isolation, transforms, invalid input and a baked SCAD round trip into the main geometry pipeline. Manual Edge verification covers rectangle drawing, extrusion, Z translation, undo/redo, deleting the source sketch and reloading the saved body.

Design reference: [Conversation](https://james-bern.github.io/conversation.html), whose public page describes direct sketching, direct modeling and separate 2D/3D objects. This implementation does not depend on Conversation software.

## Video-driven interaction pass

Reviewed the embedded 69-second `conversation.mp4` on the reference page in the browser. Visible examples include on-canvas operation feedback, highlighted contours, Fillet/DogEar, repeated circular profiles, ExtrudeCut, RevolveAdd and orbiting the result. These are observations of the demonstration, not a claim that all shown operations are implemented here.

Added:

- Orbit controls with view-dependent face shading and a projected floor grid. Left drag or right drag orbits; G switches left drag to XY body movement. Middle/Shift drag pans. Camera movement does not enter document history.
- E opens a non-destructive extrusion tool preview. Drag its height handle or enter signed height and base Z. New/Add/Cut commits baked geometry; Add/Cut requires an explicit target. Enter confirms once; Escape discards the tool. The translucent volume shows the operation tool, not the final boolean result.
- Circular copies about an explicit XY center, with count/sweep preview. Confirming all copies is one undo step; source geometry stays independent. Ctrl/Cmd+D makes a translated independent copy.
- Vertex/grid snapping with a visible vertex marker; Alt bypasses snapping. Live size feedback during rectangle/circle drawing. Polylines can close by clicking the first point.
- Hover highlight and compact on-canvas operation cards. Exact numeric transforms are tucked into a contextual popover. V/R/C/L/E/F/G and ? are documented in the help overlay.

The advanced pass below adds analytic circle/arc objects, planar face workplanes and multiple selection. 3D preview still uses projected SVG triangles with painter ordering, not a depth-buffer renderer; intersecting surfaces may have occlusion artifacts. Undo history is still session-local.

Additional verification: `tests/directModelingTools.test.ts` checks camera inverse projection, snap precedence, signed extrusion, confirmed add/cut volumes/topology and undo, invalid target handling, and circular copy spacing/isolation. Browser checks cover E, height-handle dragging, Escape without history edits, circular copy confirmation + one undo, and orbit changing screen geometry without history edits.

## Corner tools and profile rotation

- Select a closed sketch, then **Fillet / Скруглить** or **DogEar**. Click a vertex (or use the numbered vertex selector), set the radius and confirm with Enter. The green contour is a preview; Escape discards it. Fillet supports convex and concave corners. DogEar creates a semicircular relief at a right-angle corner, with radius equal to the cutter radius. Other angles are rejected explicitly. Oversized radii, degenerate edges, intersections and the 512-point document limit are checked before committing.
- Arcs are baked as polyline samples: Fillet uses at most 5 degrees per segment, DogEar at most 2.5 degrees. This is sketch corner editing, not a fillet on 3D solid edges. Each confirmed corner is one undo step; existing bodies remain independent.
- **Revolve / Вращение** rotates the sketch around an X or Y axis in its own XY plane. The dashed line shows the axis and its editable offset. Signed sweeps from 0.1 to 360 degrees and 8–128 segments are supported; partial rotations have closed end caps. Profiles may touch the axis or lie entirely on either side, but cannot cross it. The kernel currently accepts at most 128 profile points and 20,000 generated side triangles.
- New/Add/Cut uses the same explicit target-body interaction as extrusion. The translucent preview is the revolved tool; confirming bakes the new or boolean result in one history step. The original sketch stays separate.

Verification: `tests/directProfileTools.test.ts` checks fillet radius/tangent endpoints/area with both windings, concave corners, DogEar relief area and closed extrusion, invalid corners/radii, full/partial/negative revolutions, both axes and sides, shifted axes, and add/cut volumes with undo/redo and source independence. Manual Edge checks cover applying Fillet, undoing it, selecting a different DogEar corner and undoing it, and creating a capped 180-degree revolved body.

DogEar geometry reference: [Vectric Dogbone documentation](https://gadgets.vectric.com/v9/dogbone).

## Advanced direct editing (ten-tool pass)

1. **Push / Pull:** switch to Faces and select a planar surface. Drag the selected face again or its yellow handle, or enter a signed distance in the operation card. This moves the face support plane while preserving a convex solid; it is not a blind triangle displacement.
2. **3D chamfer / fillet:** switch to Edges and select a feature edge (triangulation diagonals are excluded). Set a chamfer distance or fillet radius; inspect the replacement preview, then confirm. Fillets are tessellated with 16 angular intervals. Edges are selectable as an overlay, including occluded edges.
3. **Sketch on face:** select a planar face and press Sketch on face. The left pane switches to the face's local coordinates and shows its boundary; new sketches store an orthonormal world-space basis. Extrude/Add/Cut follows that plane's normal. XY returns to the global drawing plane. Workplanes are independent snapshots, not live references to a body.
4. **Analytic circles and arcs:** new circles/arcs store center, radius, start and sweep, with sampled points as a rendering/meshing cache. Edit the numeric curve card or drag center/radius/arc endpoint handles. Tangents are shown at the angular endpoints. JSON round trips, uniform transforms, circular copies and offsets preserve the analytic representation. Corner edits and Trim explicitly bake affected curves to polylines; there is no general mixed line/arc wire representation yet.
5. **Multiple selection:** Shift-click objects or their list buttons; Box select encloses all vertices/projected body vertices in a rectangular selection. Shared transforms, dragging, duplicate and delete commit once and undo together.
6. **Gizmo:** world X/Y/Z translation handles, rotation rings and uniform scale handles act around the selection's shared bounds center. The Transform selection card exposes exact translation, axis, angle and scale. Camera movement stays outside document history.
7. **Trim / Extend:** click the portion of a contour to remove between nearest intersections; both surviving chains are retained. Extend an open polyline's chosen endpoint to the nearest intersecting boundary on the same workplane. Curves are sampled for Trim; Extend requires a polyline.
8. **Offset:** signed miter offset for closed polylines; analytic circles/arcs change radius. Positive distances expand the contour. Collapsing and self-intersecting results are rejected; concave contours that would need topology changes are not silently repaired.
9. **Shell:** Faces mode, Ctrl/Cmd-click one or more openings, then enter wall thickness. Inward support-plane offsets produce the cavity and boolean subtraction removes it through the selected openings.
10. **Plane split:** choose X/Y/Z normal and signed plane position, or drag its yellow handle. Both closed halves are previewed in distinct colors and retained as separate bodies. The service also supports arbitrary normals, tested with an oblique plane; the UI currently offers the three world axes.

**Current solid-operation envelope:** Push/Pull, Shell and edge chamfer/fillet require a closed convex solid with at most 64 planar support faces. Unsupported nonconvex bodies, collapsed dimensions and invalid results are rejected without modifying the document. This is a bounded mesh implementation, not general B-rep filleting or shelling of curved/nonconvex surfaces. Sketch workplanes and splitting do not have that convex-only restriction. The document retains the existing 200-object/4 MB limits. SVG painter ordering remains the preview renderer and can show occlusion artifacts.

**Verification:** `directAdvancedTools.test.ts` checks merged face/edge topology, signed Push/Pull volumes, chamfer and fillet volumes, Shell wall thickness, axis-aligned/oblique split volume conservation, vertical face extrusion, analytic curve round trips/transforms/offsets, Trim/Extend boundaries and shared-pivot history. `directModelerUi.test.ts` mounts the actual Vue component with a deterministic custom renderer and invokes rendered controls/pointer handlers to check selection, previews, confirmation, cancellation, undo, face workplanes, curve parameters, Trim/Extend, box selection and gizmo commits. These component tests do not simulate browser layout or prove visual picking accuracy. Browser checks additionally exercised drawing/extrusion and exposed the new selection modes and operation panels. A local stable preview avoids hot updates from parallel workspace work during inspection.


### Main editor tools

The main WebGPU viewport has a persistent **Primitives** toolbar with Box,
Cylinder, Cone and Sphere. Clicking a primitive appends an ordinary OpenSCAD
command, rebuilds the existing scene and selects the new body. It never opens
DirectModeler or replaces the viewport. The separate direct workspace remains
available through its explicit button above the source editor.

Selecting a body in the native viewport or scene tree exposes Push/Pull, fillet,
chamfer, Shell, split, face profile, move, rotate, scale, duplicate and delete.
Face operations use the native triangle pick. Edge treatment offers an edge
selector. Face profiles are dimensioned rectangles/circles at the picked point,
with union/cut depth; this is not the standalone freehand sketch canvas.
Preview publishes temporary meshes to the same WebGPU renderer. Cancel restores
the original meshes and visibility; preview picks cannot replace the source
selection. Apply bakes the edited scene to SCAD polyhedra and rebuilds it.
Source undo/redo restores the original source, and stops when unrelated edits
make its snapshot stale. Histories are bounded by count and retained text size.
Operations require a current completed build; the existing convex/planar limits
for Push/Pull, edge treatment and Shell still apply.

### Native viewport interaction pass

- Main viewport tools now project their handles with the renderer's actual camera.
  Move/rotate/scale gestures preview in WebGPU and commit once on pointer release.
  Push/Pull has a face handle; edge operations draw selectable, highlighted edges.
- Shift-click adds/removes bodies; Box select encloses projected vertices. Group
  transforms share a world-space pivot; duplicate/delete use the entire selection.
- Split accepts arbitrary XYZ normals and a draggable plane. Shell lets users
  click multiple face openings. Overlaid edges/faces currently include occluded
  geometry; use the highlighted result and preview to disambiguate.
- Face profile opens a drawing layer over the main viewport, not DirectModeler.
  XY drawing is also available. Rectangles, polylines, analytic circles/arcs,
  vertex/curve handles, Trim, either-end Extend, Offset, extrusion and cut share
  the existing geometry services. Sketch histories are local; sketches persist
  by workplane in browser storage. Mixed line/analytic-arc wires remain unsupported.
- Solid extensions recognize straight prisms before applying profile-based Shell
  and longitudinal edge fillet/chamfer, including nonconvex profiles. Cylinder
  shells use tessellated profiles. Spherical shells construct a concentric inner
  skin and bridge the chosen openings. Arbitrary curved/nonconvex B-rep offsets
  and general surface fillets are still outside the supported geometry envelope.
- Added checks cover group pivots, oblique splits, nonconvex prism/sphere shells,
  gesture commit count and sketch drawing/extrusion through mounted components.

### General mesh Shell and local edge blends

Unsupported specialized Shell cases now fall back to a sampled signed-distance
construction in Rust. The material is the source interior within the requested
thickness of retained (non-opening) faces. A BVH accelerates nearest-triangle and
ray-parity queries. Extraction validates its final closed, positive-volume mesh.
The requested grid step must resolve at least three samples across the wall;
limits are 64 cells per axis and 30,000 source triangles. Openings too small to
resolve are rejected. This reconstruction can change sub-cell detail and is not
an analytically exact surface offset or certified Hausdorff tolerance.

Unsupported specialized edge blends use a local circular cutter/filler bounded
by the selected straight mesh edge and its two incident faces. Convex edges cut
material; concave edges add material. Radius limits protect adjacent face extents
and endpoints. This is a tessellated local blend, with planar end termination,
not a general analytic rolling-ball B-rep fillet across a curved edge chain.

Shell/fillet/chamfer calculations run in a cancellable worker. New requests and
parameter/source changes terminate obsolete work; stale responses cannot apply
geometry. The UI labels the mesh approximation and exposes Shell grid spacing.
Tests cover a tapered nonconvex shell and edge blend, resolution rejection,
BVH/reference signed-distance agreement, and worker supersession/error cleanup.

## Main viewport CAD workbench — next 20 features (2026-09-09)

Entry points: **Операции CAD**, **Размеры**, **Эскиз XY** and the existing selected-body toolbar in the main WebGPU viewport. Heavy geometry runs in a cancellable worker. Preview is temporary; Apply publishes one source edit with Undo/Redo.

| # | Implemented behavior | Current boundary |
|---|---|---|
| 1 | Replace only top-level source calls owning changed meshes; preserve other primitives, variables, modules and comments | Changed calls become polyhedra. Ambiguous provenance is rejected; this is not a parametric feature-tree editor |
| 2 | Persist source Undo/Redo per filename; validate matching source and chain on reload | Browser storage, 2 MB / 80 persisted entries; manual source edits start a new modeling chain |
| 3 | Face sketches use native body/face identity; stored planes refresh after parameter rebuilds | Identity must survive. Changing operation type/topology can require reattachment; derived extrusions do not rebuild automatically |
| 4 | Snap sketch points to coplanar vertices, feature-edge midpoints and body centers; axis movement snaps its center to collinear targets | 10 CSS-pixel threshold, 5000 candidate budget; Alt bypasses movement snapping |
| 5 | Editable X/Y/Z dimensions drawn on selected model/group | Axis-aligned group bounds, not arbitrary annotation constraints |
| 6 | Selected-body union, ordered difference, intersection | First selected body is the base; empty results are rejected |
| 7 | Loft between ordered closed sketch sections | Polygonal sections, resampled to matching vertex counts; no analytic loft surface |
| 8 | Sweep closed profile along open sketch path | Polygonal transport; self-intersecting or collapsed results may be rejected |
| 9 | Shift-select multiple edges and specify start/end blend radii | Local straight-edge cutters; no rolling-ball corner patches or general curved edge chains |
| 10 | Axis-directed body draft deformation | Radial mesh deformation, not a general neutral-plane face-draft solver |
| 11 | Mirror selected bodies across arbitrary plane, copy or replace | Reverses triangle orientation to preserve outward solids |
| 12 | Linear or arc-length path arrays of body groups | 2–100 instances; orientation stays fixed along path |
| 13 | Align min/center/max to first body and evenly distribute | World axes; distribution spaces reference positions, not necessarily equal gaps |
| 14 | Plain, counterbored and countersunk holes from picked face axis/origin | Explicit diameters/depths; polygonal circular cutters |
| 15 | Internal/external helical thread geometry placed on selected body | Diameter/pitch/depth inputs; external adds a threaded rod, internal subtracts one; no automatic cylindrical face inference |
| 16 | Hinge/slider reference, absolute position and limits saved in source comment | Two components per joint; changed component geometry invalidates reference. No linked assembly constraint propagation |
| 17 | Pairwise intersection volume and triangle-surface minimum gap | At most two million triangle pairs; mesh result, not analytic clearance |
| 18 | SVG/PDF vector drawing: three orthographic views, dimensions, horizontal section | All feature edges visible; no hidden-line removal or drawing standards certification |
| 19 | STEP import/export with millimetre faceted BREP | Import accepts convex planar poly-loops, uniform m/mm. Rejects analytic BREP, void shells, mapped assemblies and converted units; interoperability beyond local round-trip is not yet verified |
| 20 | Adaptive sparse Shell tiles skip blocks excluding the surface and weld shared boundaries | Same requested spacing in active tiles; ≤256 cells/axis, four million samples and 100000 output triangles. Not variable surface LOD; sub-cell detail may be missed |

STEP topology follows [ISO 10303-42 faceted_brep](https://www.steptools.com/stds/smrl/data/resource_docs/geometric_and_topological_representation/sys/6_schema.htm) and [poly_loop / closed_shell](https://steptools.com/stds/smrl/data/resource_docs/geometric_and_topological_representation/sys/5_schema.htm). Export uses AP214 product/shape contexts and planar FACE_SURFACE entities. Unsupported input is rejected explicitly rather than partly imported.

Validation: CAD geometry tests cover boolean ordering, mirror orientation, group sizing/arrays, alignment/distribution, holes/thread, Loft/Sweep, clearance, drawing structure, STEP round-trip, persistent bounded joints after a real parser rebuild, source preservation and sketch support after parameter rebuild. Adaptive Shell tests exercise cross-tile closedness above the dense-grid axis limit. Browser smoke: main CAD panel, mirror preview/apply, unchanged original source calls, Undo and reload-persisted Redo. File-picker STEP import and external CAD interchange were not browser-tested.

## Surface texture generator

Use **Текстура** in the main viewport toolbar after selecting a body. Available patterns: ribs, recessed grooves, diamond knurl, deterministic fuzzy skin, dimples, and waves. Pitch, height/depth, angle, detail, inversion and fuzzy seed are editable. The picked-face option uses that face's local frame and tapers displacement to zero at its boundary; whole-body mode uses world XY for periodic patterns and 3D noise for fuzzy skin.

This generates actual mesh relief, retained by source edits and STL export. It is not a shader or slicer extrusion-path setting. Shared-edge uniform subdivision keeps the indexed surface connected, then vertex normals displace the refined mesh. The generator rejects excessive subdivision (40000 triangles), height above pitch/4, invalid faces, flipped triangles and invalid closed-solid results. General distant self-intersections and minimum wall thickness are not checked. Fine textures on large bodies can exceed the editor source limit; increase pitch or reduce detail. Preview is cancellable and does not edit source; Apply is undoable. CAD computations are worker-owned to avoid duplicating the geometry operations in the main bundle.

Validation: six-pattern closure, source immutability, fixed selected-face borders, deterministic/different seeds, bounds rejection, and a texture/source/parser round-trip preserving unrelated source. Browser smoke applied default ribs to a 20×15×10 box: 3072 triangles, 78650-character scene source, all other three bodies retained, no reported errors, Undo available.

## Solid lightening / skeletal walls

Two additional volumetric modes are available under **Облегчение → Структура**: **3D узлы и стержни** and **Кость — объёмная пористость**. Both connect nodes throughout X/Y/Z and clip the result to the source signed-distance field. Spatial mode has optional cell diagonals. Bone mode jitters internal nodes, varies strut radii and blends junctions; it is an organic graph approximation, not 3D Voronoi or mechanical topology optimization.

Controls include cell size, strut diameter, seed/jitter, outer skin and sampling step. Opening the +Z skin leaves the lattice struts in place. Limits are 125 nodes, 400 edges, 30000 input triangles and 64 sampling cells per axis; step must be at most diameter/2.5 and skin/2. Output is simplified toward 2600 triangles using local topology-preserving edge collapses with vertex-cluster radius bounded by 1.5 steps. Final checks reject open/degenerate meshes, increased volume and extra positive-volume components; cavity boundaries are allowed. Source budgets still apply. Minimum final thickness, global self-intersections, strength and support-free printing are not certified.

Six new tests cover XYZ connectivity, seeds, both modes, closed reduced-volume output, skin opening, source/parser round-trip and limits. All 49 related tests and build checks passed. Browser spatial preview/apply on a 20×15×10 mm box produced 2600 triangles and reduced volume from 3.00 to 0.77 cm³ (74.3%), retaining the other three objects.

The main viewport **Облегчение** button opens four structures: rectangular grid, diagonal triangular skeleton, honeycomb-like Voronoi cells and seeded irregular Voronoi web. Cells are inset by half the rib width, then their extruded volumes are subtracted from the original solid in one Boolean operation. All original geometry outside the cut volumes remains. Channel direction is X/Y/Z; this is an extruded wall lattice, not an arbitrary spatial truss or stress-based topology optimizer.

Controls include cell size, minimum rib width, bounding-box frame, bottom/top skins measured along the channel axis, seed and jitter. The print controls use the actual extrusion line width and requested line count to reject thinner ribs; rounding to a multiple of line width is available. Z channels avoid adding transverse bridges within the generated lattice when Z is also the print direction; a top skin introduces bridging. Original geometry may still need supports. The rectangular bounding frame is not a contour-offset frame on arbitrary curved bodies. Preserve a bottom skin or increase ribs/frame if clipping would produce detached pieces.

Validation rejects invalid dimensions, more than 144 sites, vanished material, non-closed output, extra disconnected components and export-scale degenerate triangles. Output volume must decrease. The panel reports percentage and cm³ before/after; no strength, stiffness, fatigue or print-time claim is made. This complements slicer infill by changing the actual exported geometry. Print preparation follows the distinction between solid geometry, wall thickness and overhangs in [Prusa's modeling guidance](https://help.prusa3d.com/article/modeling-with-3d-printing-in-mind_164135).

A Boolean fix treats coplanar retriangulation as optional: if its multi-hole bridge heuristic fails, the stitched triangles continue through unchanged topology, intersection and orientation audits. No validation is skipped. The distribution budget increases by 10 kB for the measured ~8 kB lightening generator and controls.

Verification: four patterns, repeatable/different seeds, all channel axes, complete retained bottom face, rejected sub-line ribs and generation budget, source/parser round-trip. 43 related Vitest tests and 10 native Boolean tests passed. Browser applied an irregular web to a 20×15×10 box: volume 3.00 → 1.97 cm³ (34.4% reduction), 84 triangles, other scene bodies retained and Undo available. Physical print and mechanical tests have not been performed.

### Skeletal walls

The volumetric modes support **Область сетки → Только стенки**. Wall depth restricts the lattice to a signed-distance band at all source surfaces, including floor and roof. The interior is optionally hollow (default) or a solid core. Outer skin remains independent: set it to zero to expose lattice openings. Existing thin-walled solids can also use whole-volume mode to lattice their existing material. This is a clipped spatial graph, not a surface-conforming remesher; disconnected results are rejected. Wall depth must span at least two sampling steps. FDM fitting respects this additional sampling bound.

Geometry tests cover hollow walls versus full-volume lattice, optional solid core, closed output and rejected undersampled wall depth. Nineteen related tests, type checking, geometry build and distribution checks passed.

### Strength-oriented structures

Three additional structures target high stiffness-to-weight rather than only volume reduction:

- **Октет-ферма / Octet truss** — stretch-dominated 3D graph with cube corners plus face centres, face-to-corner struts and octahedron edges between face centres. Prefer this for skeletal walls (`Только стенки`) and volumetric lightening when isotropic stiffness matters. Selecting it defaults the lattice region toward a wall band.
- **Изогрид / Isogrid** — extruded equilateral triangular openings (NASA-style isogrid). Best channel-mode choice for planar walls and panels; use Z channels for FDM when possible.
- **ОЦК / BCC** — body-centred cubic struts from each cell centre to its eight corners. Strong under compression with a simpler, lighter graph than octet.

The panel shows a short qualitative hint for every structure (stretch / mixed / bending / organic). Hints and rankings are geometric guidance only — not FEA, fatigue or print certification. Node/edge budgets remain 125 / 400; octet and BCC need larger cells on big bodies.

### Strength-of-materials estimate

**Облегчение** includes an analytical strength-of-materials block (material, load case, force, safety factor). After preview it appends a report based on:

- relative density from measured before/after volume (or a geometric estimate beforehand);
- Gibson–Ashby scaling \(E^\*/E \sim C\rho^n\) with pattern-dependent stretch/bending exponents;
- mean spatial-graph connectivity (Maxwell / Deshpande stretch vs bending);
- strut axial stress and Euler buckling ratio for spatial lattices;
- ranked **weak spots** with approximate XYZ: free ends, hinge nodes (Z=2), under-connected nodes, body/frame corners, acute cell corners, slender/buckling struts, long load-aligned spans, horizontal bridges and overly thin skeletal walls.

Materials are isotropic reference values (PLA/PETG/ABS/Nylon/Al 6061/steel). FDM anisotropy, contacts, residual stress, fatigue and local print defects are out of scope. Weak-spot screening is a strength-of-materials heuristic with coordinates for inspection, not an FEA hotspot map. Scores and viewport marker colours are **material-weighted**: brittle FDM (e.g. PLA) elevates sharp corners, bridges and thin walls via notch/layer factors; ductile nylon reduces corner criticality; metals shift emphasis toward Euler buckling of slender struts. Changing the material after preview refreshes the ranked list and coloured octahedron markers without re-running the lightening boolean. The UI states explicitly that this is not FEA or certification.
