# Source-bound partial annular preview

`analytic_features::build_partial_annular_preview` now constructs the prototype
from a selected source edge. It validates and audits the source, recognizes the
complete annular support nets, checks the selected degree-2 quarter-circle
control points, weights, axial position and radius, then authors the local
transition body and places it in the source axial frame.

The focused native test enumerates every source edge: exactly four outer rim
arcs on the recognized top cap are admitted. Other rims, seam edges and an
out-of-range index are refused. Every admitted result is topology-closed with
one body and complete persistent names; the source remains unchanged.

`inherit_topology_ids` uses the existing geometric naming mechanism. Complete
names alone do not prove that every unchanged source identity is retained:
the reconstructed sector representation differs from the original annular
caps. Explicit retained-entity ownership and ChangeSet qualification remain
open, along with independent placed-result STEP tests and whole-boundary proof.

This API returns a Model, not an AuditedFeatureResult or complete feature
certificate. It is not connected to the user command dispatcher. The existing
qualified full-rim feature still refuses partial arcs.
