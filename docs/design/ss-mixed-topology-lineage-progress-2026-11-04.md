# Lossless mixed topology transfer and snapshot-bound lineage

`MixedBrep::concatenate` transfers complete mixed topologies through one checked
implementation. It shifts original vertex/edge/loop/face/shell/body references,
including the separate root-pcurve world-edge binding. It preserves original
root selectors, source graphs, coedge traversal, rational curves, UV curves,
closed-shell ownership, existing cavity shells and exact trim/lift metadata.
The result passes the original mixed topology/binding validator. This is a
storage operation and never grants material, geometric separation or nesting.

`MixedTopologyMap` maps every source entity, coedge and shell face use into the
child snapshot. It records additional shell orientation reversals and supports
many-to-one body ownership when a source solid becomes a cavity. Invalid source
indices return None. Total index ranges are checked before shifts. Exact source
identity and indexed transfer are retained independently of world coordinates.

Maps own opaque source and target snapshot identities. Immutable model clones
retain the same identity; reconstructing a model from equal geometry creates a
new one. The identities stay alive while maps reference them, preventing token
reuse. `then` composes maps only when the intermediate snapshots are identical.
It composes every entity/body map and XORs shell orientation changes. Equal
counts or equal geometry alone cannot authorize composition. `same_mapping`
compares only index/orientation policy and explicitly is not an identity proof.
These are process-local identities, not persistent history IDs across restarts.

The admitted cavity assembler now uses this common transfer. It changes the
appended body into an explicit inner shell, reverses its material shell uses,
and retains both parent maps to the single result body. It owns the original
host mixed snapshot as well as the admitted root source; `lineage_sources`
exposes those exact source snapshots. Its maps are rebound only to the validated
final result snapshot. Replay checks the live source/target identities and,
after fresh geometry/material proofs, compares the reproduced mapping policy.

Tests cover mixed root/rational transfer, exact root-pcurve binding, immutable
source selectors, rational subranges and periodic-lift metadata, existing cavity
shells and multiple bodies, repeated map composition, rejection of a foreign
intermediate snapshot, and invalid indices. Synthetic trim/lift metadata in a
storage test is not presented as periodic geometry admission. Native cavity
tests additionally compose root-to-cavity lineage through a further prefix
transfer, preserving body ancestry and inward orientation after index changes.

This supplies faithful unsplit topology transfer for later mixed shell forests.
It does not establish complete mixed shell nesting or intersecting Boolean.
Splitting/merging edge subranges through Boolean and patterns, persistent IDs,
branch-history reconciliation, arbitrary SS singular/coincident and periodic/
pole strata, full finite-face trim, healing and actual bridge/WASM delivery
remain in the full original goal. Broader offset/fillet/quad/G2 parity is not
qualified by these storage and cavity tests.

Frozen qualification: 20 tests passed, including both cavity genera, original
U reflection and fresh native material replay. Architecture remains at 54 pinned
oversized files and zero pinned layer violations. Logs and exact tested source
hashes are in `docs/qualification/ss-mixed-topology-lineage-2026-11-04/`.
