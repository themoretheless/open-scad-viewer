# brep-intersect

Analytic pair intersections on retained B-rep models (layer 4, above
`brep-core`): plane, sphere, cylinder, cone and torus against each other.
Each `intersect_<a>_<b>(&Model, &Model, Options)` recognises both operands
exactly through `brep_core::intersections::recognize_*`, computes the
intersection in closed form, and reports it through the `nurbs-intersect`
report schema with lifted UV traces on the canonical patches. Reports never
authorise a topology change.

The sphere–sphere pair stays in `brep-core` (the analytic Booleans call it
directly) and is re-exported here so `geometry-bridge` sees one API.
