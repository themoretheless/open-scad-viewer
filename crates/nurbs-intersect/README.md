# nurbs-intersect

Bounded, geometry-only intersection queries on rational curves and surfaces,
and the certificates that travel with their reports. Layer 2: it depends only
on `nurbs-core`, `cad-predicates` and `value-codec`, and knows nothing about
B-rep models.

| Module | Contents |
| --- | --- |
| crate root (`queries.rs`) | report schema (`Report`, `Coverage`, `Options`, `Plane`, `SurfaceTrace`, …) and the seven queries `curve_plane`, `surface_plane`, `curve_surface`, `surface_surface`, `curve_segment`, `curve_curve`, `curve_ruled_surface` |
| `nurbs_ss` | G6 narrow transverse surface–surface engine, multispan branch graphs and exact iso-intersection certificates |
| `analytic_ss` | analytic surface–surface pairs (plane, cylinder, sphere, …) as complete reports |
| `predicate_evidence` | composed predicate evidence under a tolerance context |
| `coverage_verifier` | independent coverage checks beside intersection reports |
| `RationalCurveDefinition` | bit-exact record of a rational curve |

`brep-core` consumes this crate for recognition, audits and Booleans;
`brep-intersect` adds the analytic pair intersections on `Model`s;
`geometry-bridge` dispatches the queries to the front end.
