# Competitor Capability Gaps: 2026-09-21

This is a capability comparison, not a claim that the products use the same
geometry kernels or data contracts.

## OpenSCAD

OpenSCAD documents two deliberately different display modes: a fast,
approximate GPU preview and an exact CGAL render that produces a fully
tessellated mesh. The separation makes responsiveness predictable, but exact
render can take minutes or hours on larger designs.

Source: [OpenSCAD User Manual](https://files.openscad.org/documentation/manual/OpenSCAD_User_Manual.html)

Our stack already has the stronger foundation for a third option: retained
Rust solids, explicit refusal/budget contracts, and deterministic display
snapshots. The missing product layer is a visible progressive policy that can
start approximate, refine to exact, and explain which representation is being
shown.

## Onshape

Onshape's graphics display data uses unloaded/bounding-box, coarse, medium,
fine, and extra-fine levels. Parts are prioritized by screen size and camera
distance, refined incrementally, and constrained by a memory limit. It can
restrict automatic levels for complex Part Studios.

Sources: [Graphics Area Display Data](https://cad.onshape.com/help/Content/Home/graphics_area_display_data.htm),
[Render Studio tessellation settings](https://cad.onshape.com/help/Content/RenderStudio/render_studio_interface_scene_list.htm)

Onshape also treats imported meshes as view/reference data rather than editable
parametric geometry. This is a useful explicit distinction for our model
graph: imported mesh, exact B-rep, and sampled NURBS should remain different
capability states instead of being silently coerced into one solid type.

Source: [Working with Imported CAD](https://cad.onshape.com/help/Content/Document/working_with_imported_cad.htm)

## Gap and Proposed Extension

The highest-value missing feature is an adaptive display scheduler:

- a camera-aware target error and screen-space size estimate;
- bounded coarse-to-fine tessellation levels per retained solid;
- memory admission before refinement, with downgrade/unload priorities;
- exact-render promotion as an explicit state transition;
- cache keys containing solid identity, transform, tolerance, and display mode;
- deterministic fallback to the last admitted level when a budget is exhausted.

This should sit above `geometry-analysis` and below the viewport, while the
Rust kernel remains responsible for tessellation validity and budgets. It can
reuse the existing `RenderMesh`, BVH, face ids, and artifact identity rather
than introducing a second mesh representation.

## Rewrite-from-Zero Decision

If starting over, the display contract would be a typed `DisplayRepresentation`
with `BoundingBox`, `PreviewMesh(level)`, `ExactMesh`, and `Refused(reason)`
states. The current compatibility facade can adopt this incrementally: first
add the scheduler and cache key, then route existing `renderMeshInKernel`
requests through level selection. No competitor feature justifies weakening
the current exactness, ownership, or refusal invariants.
