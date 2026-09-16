# nurbs-step-bicubic-face/1

Walking-slice freeform NURBS STEP: one rectangular untrimmed positive-weight bicubic open face.

## In scope

- Uniform bicubic (`degree 3×3`, 4×4 net, weights ≡ 1, non-periodic)
- Topology: 1 face, 4 LINE edges, 4 verts, open shell (no body)
- Export/import: `B_SPLINE_SURFACE_WITH_KNOTS` + `ADVANCED_FACE` + `OPEN_SHELL` + `SHELL_BASED_SURFACE_MODEL`
- Product seam: `brep_nurbs_export_step_freeform` / `brep_nurbs_import_step_freeform`, `cadNurbsStep.ts`

## Out of scope

- Closed freeform solids / `MANIFOLD_SOLID_BREP`
- Trimmed faces / holes
- Rational weights ≠ 1, non-bicubic
- Constructor corpus (`step-interchange/1`)
- Parasolid parity / arbitrary AP214 soup

## Evidence

See [nurbs-step-bicubic-face-1-evidence-v1.json](../../qualification/nurbs-step-bicubic-face-1-evidence-v1.json).
