# Exact premises for planar profile transport

Rust profile certificates now recognize arbitrary authored planar control hulls. Positive rational weights keep the entire source curve in that plane. Three non-collinear controls supply an outward-rounded unit-normal enclosure; every original control is checked by the bounded exact `orient3d` predicate before this enclosure can be used.

At full-multiplicity Bezier joins and closed source endpoints, all three projected `orient2d` predicates must prove exact collinearity. Strict authored coordinate ordering proves a common nonzero signed tangent direction. No rounded vector differences, tolerance snapping, or inferred ideal curve enters this decision. Coordinate-axis cases retain their existing exact comparison fast path.

Anchor search and exact predicate work consume the existing certificate cell budget. An unfinished premise never enables the analytic planar Bishop frame. Closed source paths require exact endpoint agreement even when the retained surface is not marked periodic.

Local regression cases cover arbitrary-plane constant-normal oracles, oblique multi-span joins, closed tilted planar paths at six and ten stations, one-ulp changes, reversal, immutability and exhausted budgets. These extend planar-frame certification; they do not prove general nonplanar Bishop holonomy, smooth retained BRep station seams, global embedding of the ideal sweep family, or complete qualification across all constructors.
