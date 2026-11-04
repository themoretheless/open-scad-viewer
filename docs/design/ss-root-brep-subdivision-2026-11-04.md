# Applying exact SS edge partitions to an existing root BRep

`QualifiedRootBrepEdgeSubdivision` applies qualified original-edge partitions to
an immutable `RootBrep` snapshot. Each request names an existing edge and must
match its literal original graph, endpoint selectors, traversal and phases.
Duplicate/foreign/out-of-range requests are refused.

The first child keeps the parent edge slot; other children and interior vertices
are appended in deterministic ORIGINAL edge-index order, independent of request
order. Every original vertex remains at its original index with its original
selector. Source endpoints are reused. Interior cut aliases are resolved by the
native partition's vertex map. This is pure subdivision: no implicit weld is
made to another edge or old vertex merely because world images coincide.

Every coedge use in EVERY loop is expanded. Reversed coedges use reverse child
order. Original support roles, pcurve/coedge reversal and world-edge references
are preserved or retargeted to the exact child edge. Existing loop indices,
face outer/hole references, original surface definitions, shell/body membership,
face-use orientation and tolerance remain unchanged. The output passes ordinary
root BRep incidence and original pcurve binding validation.

The certificate retains source/output snapshot owners, parent-edge child maps,
parent-coedge child-position maps and input-cut-to-global-vertex maps. New entity
ranges and coedge growth are explicit. These are SNAPSHOT-LOCAL indices, not a
complete persistent naming/history solution. Vertex/edge/coedge/request budgets
abort before a partial model is returned. Replay rebuilds source/partition proofs,
all topology maps, original selectors and resulting BRep binding.

Qualification applies two irrational cuts to an existing eight-edge paired BRep,
updating both face loops to ten coedges with original vertices/surfaces/shells/
bodies retained. A reversed world edge and reversed coedge uses exercise child
order in both faces. Endpoint aliases are no-ops; multiple requests yield the
same child numbering under permutation. Foreign/duplicate/out-of-range requests
and deficient growth/request budgets are rejected. Closed incidence and Euler
characteristic are preserved in the fixtures; no new volume/material admission
is inferred from this topology operation.

Remaining work includes cross-source world/UV junction bindings, canonical global
vertex sewing and mixed-pair face arrangements, actual SURFACE region splitting,
material selection and intersecting Boolean/healing. General algebraic UV and
boundary/periodic/pole proofs, persistent split/merge lineage/history, actual
bridge/WASM calls and the wider quad/offset/fillet/G2 scope remain open.
