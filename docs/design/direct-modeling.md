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
