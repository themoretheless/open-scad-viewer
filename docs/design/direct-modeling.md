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

Remaining differences from the demo: analytic arcs, Fillet/DogEar, revolve tools, arbitrary face-attached workplanes and multi-selection are not yet supported. 3D preview still uses projected SVG triangles with painter ordering, not a depth-buffer renderer; intersecting surfaces may have occlusion artifacts. Undo history is still session-local.

Additional verification: `tests/directModelingTools.test.ts` checks camera inverse projection, snap precedence, signed extrusion, confirmed add/cut volumes/topology and undo, invalid target handling, and circular copy spacing/isolation. Browser checks cover E, height-handle dragging, Escape without history edits, circular copy confirmation + one undo, and orbit changing screen geometry without history edits.
