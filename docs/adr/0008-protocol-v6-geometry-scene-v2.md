# ADR 0008: Protocol v6 / GeometrySceneV2 IDL freeze (G0.8)

- Status: accepted
- Date: 2026-09-12
- Accepted-by: repository-owner (human authorization in session)
- Contract: `protocol-v6-geometry-scene-v2`
- Fixtures: `docs/qualification/protocol-v6/`

## Context

Design prose historically called the current Worker wire “v5” and the B-rep
migration target “v6”. The shipped TypeScript constant
`GEOMETRY_WORKER_PROTOCOL_VERSION` is already **6**. G0 must freeze both:

1. the **current** Worker message IDL (as shipped today);
2. the **future** `GeometrySceneV2` publication envelope required before G4a.

G0.8 is contract-only. It must not change runtime routing or geometry results.

## Decision

### Naming

| Name | Meaning |
| --- | --- |
| `worker-protocol-current-v6` | Exact shipped Worker request/event shapes today |
| `geometry-scene-v2-idl` | Future publication payload for B-rep/Manifold-on-v6 migration |
| Design doc “v5” | Historical name for the pre-scene-v2 wire; maps to current-v6 |

### Current Worker protocol (frozen snapshot)

Normative fields already required on every job envelope:

`protocolVersion` (=6), `documentRevision`, `jobId`, `quality`, `sourceSha256`
(except cancel omits quality/sourceSha256).

Events: `accepted` | `started` | `progress` | exactly one terminal
(`succeeded` | `failed` | `cancelled` | `stale`).

Invariants preserved:

- latest-only publication;
- full never downgrades to preview;
- export only from current full;
- payload limits in `GEOMETRY_WORKER_PAYLOAD_LIMITS`.

Checked-in fixture:
`docs/qualification/protocol-v6/current-worker-protocol-v6.fixture.json`.

### Future GeometrySceneV2 (IDL only)

Publication payload is `GeometrySceneV2`, not a competing root `MeshData`.
Required future fields (G4a implementation):

- `workerEpoch`, `kernelKey`, `kernelFingerprint`;
- capability manifest/version;
- representation + typed evidence/certificates;
- `topologySnapshotId`, `meshAssetId`, packet-local `PacketTopoToken` tables;
- backend-computed `fullEquivalent`;
- occurrences, geometry assets, mesh buffer refs, diagnostics.

Schema:
`docs/qualification/protocol-v6/geometry-scene-v2.schema.json`.

Coordinator still sees one atomic request family with discriminant
`InteractiveBuild | ExportCurrentSnapshot`. Snapshot handles remain Worker-local.

### Migration matrix

`docs/qualification/protocol-v6/migration-matrix-v1.json` freezes field mapping
from design-v5 naming → current-v6 → scene-v2. Migration is one atomic protocol
bump after G4a qualification. Temporary adapters must **check**
`fullEquivalent === !reduced`, not store two independent truths.

### Explicit non-goals

- Changing `GEOMETRY_WORKER_PROTOCOL_VERSION` or runtime validators in this ADR.
- Shipping B-rep UI or SceneV2 publication.
- Claiming G1 qualifies protocol-v6 / scene-v2 (forbidden by G1 plan).

## Acceptance gates

G0.8 completes only when:

1. this ADR is accepted;
2. current-v6 fixture validates against the independent checker;
3. GeometrySceneV2 schema + one positive + one negative fixture exist;
4. migration matrix covers every current message type and every new SceneV2 field.
