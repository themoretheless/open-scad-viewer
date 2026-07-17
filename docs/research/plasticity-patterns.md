# Plasticity interaction patterns

Snapshot: **2026-07-14**. This follow-up uses the official Plasticity manual
for the 2026.1 generation of the product. The goal is not to imitate
Plasticity's Parasolid/NURBS editing tools; this project renders triangulated
Manifold results and cannot promise stable B-Rep faces or edges.

## Patterns selected for the viewer

- [Command Palette](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/command-palette):
  one searchable registry for rendering, files, camera, projection and display
  commands, with keyboard navigation.
- [View Cube](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/view-cube):
  visible orientation and direct access to ISO and the six standard views.
- [Selection Mode](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/selection-mode):
  point, face and body filters with BVH preselection. Face selection maps to a
  source feature through Manifold provenance without claiming persistent B-Rep
  naming across source revisions.
- [Focus and Isolate](https://doc.plasticity.xyz/plasticity-essentials/focus-and-isolate-objects):
  frame the selected object and temporarily hide the rest of the result.
- [Shader Mode](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/shader-mode)
  and [View Mode](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/view-mode):
  shaded, mesh-edge and x-ray inspection modes.
- [Viewport navigation](https://doc.plasticity.xyz/plasticity-essentials/operating-the-3d-viewport):
  viewport-scoped shortcuts that do not interfere with editing OpenSCAD text.
- [Outliner](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/outliner):
  searchable mesh/source hierarchy with selection, visibility, focus and
  isolation.
- [Measure](https://doc.plasticity.xyz/tool/measure): face-hit two-point
  dimensions with world coordinates and XYZ deltas.
- [Section Analysis](https://doc.plasticity.xyz/common/section-analysis):
  non-destructive X/Y/Z clip plane with distance and flip controls.

## Good candidates for later passes

- exact section contours/caps and interference coloring;
- endpoint/midpoint/center/perpendicular snap families;
- source-cursor → result reverse highlighting and depth cycling;
- face-direction and draft-angle diagnostic shaders;
- adaptive grid spacing and display units;
- radial viewport menu and technical hidden-line export.

Direct face push/pull, edge loops, exact fillets and G0–G3 surface analysis are
deliberately excluded: those operations require persistent B-Rep/NURBS
topology, while the current kernel boundary is a manifold triangle mesh.
