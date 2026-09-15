# QualificationPlan — step-interchange/1

Status: **AnalyticComplete walking slice** — evidence in `docs/qualification/step-interchange-1-evidence-v1.json`.

## Scope

- Analytic ADVANCED_FACE / MANIFOLD_SOLID_BREP export/import with topology + units + placement notes
- Roundtrip AABB recovery evidence for planar solids

## Kill criteria

STL/OBJ/mesh BREP labeled as STEP B-rep; silent healing on import; FACETED_BREP-only payloads claiming analytic B-rep.

## Dependency

Unblocked: analytic Boolean (`analytic-boolean/1`) is Qualified. Full general NURBS STEP remains open.
