# QualificationPlan — step-interchange/1

Status: **AnalyticComplete** — graph-only topology roundtrip + product seam evidence in `docs/qualification/step-interchange-1-evidence-v1.json`.

## Scope

- Analytic ADVANCED_FACE / MANIFOLD_SOLID_BREP export/import with linked VERTEX_POINT → EDGE_CURVE → EDGE_LOOP → FACE_OUTER_BOUND graph
- Cuboid: honest 8 vertices / 12 LINE edges / 6 planes
- Tube annular caps: `FACE_OUTER_BOUND` + `FACE_BOUND` hole; oriented (non-+Z) tube recognize
- Ring edges as `CIRCLE` (cylinder / frustum / tube / sphere equator / torus major); apex frustum keeps CIRCLE on the nonzero ring
- Real `AXIS2_PLACEMENT_3D` on PLANE / CYLINDRICAL / CONICAL / SPHERICAL / TOROIDAL
- Units: `SI_UNIT(.MILLI.,.METRE.)`
- **Graph-only import** + **CIRCLE gate** on curved solids — no proprietary `OSCAD_SOLID`, no AABB curved collapse, no silent height defaults
- Product seam: `brep_nurbs_export_step` / `brep_nurbs_import_step` + [`cadAnalyticStep.ts`](../../../src/services/cadAnalyticStep.ts) beside faceted [`cadStep.ts`](../../../src/services/cadStep.ts)
- Schema: `AUTOMOTIVE_DESIGN` + AP242

## Kill criteria

STL/OBJ/mesh BREP labeled as STEP B-rep; silent healing; FACETED_BREP-only as analytic; incomplete graphs; curved import without CIRCLE; proprietary descriptor success path.

## Out of scope

General freeform NURBS STEP; Parasolid parity; arbitrary third-party AP214; mixing analytic and faceted importers.

## Dependency

Unblocked: analytic Boolean (`analytic-boolean/1`) is Qualified.
