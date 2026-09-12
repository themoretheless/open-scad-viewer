# ADR 0008: Protocol v6 wire IDL and GeometrySceneV2 publication payload

- Status: accepted
- Date: 2026-09-12
- Accepted-by: repository-owner (human authorization in session)
- Reviewer: repository-owner (solo dual-role attestation; not organizationally independent)
- Contract: `protocol-v6-geometry-scene-v2`
- Artifacts: `docs/qualification/geometry-scene-v2/`

## Context

Design §15.2 requires a checked-in discriminated IDL for request / accepted /
started / progress / terminals and a publication payload named
`GeometrySceneV2` before G4a product activation.

The live Worker already uses `GEOMETRY_WORKER_PROTOCOL_VERSION = 6` with
`MeshData[]` success payloads and `GeometryScene` **version 1** in
`src/core/scene.ts`. That integer bump is **not** GeometrySceneV2. This ADR
freezes the *target* discriminated contract and the migration matrix from the
current MeshData / GeometryScene v1 envelopes. Product routing and publication
behavior must not change under this ADR alone.

## Decision

### Naming

| Name | Meaning |
| --- | --- |
| Live wire integer `6` | Current Worker envelope with MeshData success (transitional) |
| `GeometryScene` v1 | Current App scene: assets + entities + inspection |
| `GeometrySceneV2` | Future publication payload: occurrences, assets, mesh buffers, topology/provenance tables, diagnostics |
| `MeshDataV2` | Internal geometry-asset packet inside GeometrySceneV2; not a competing App root type |

### Envelope invariants (preserved from current protocol)

- `protocolVersion`, `documentRevision`, `jobId`, `quality`
- `accepted` / `started` / `progress`
- exactly one terminal per job
- latest-only publication
- full never demotes to preview
- export only from current full

### Required GeometrySceneV2 additions

Every terminal success must carry:

- `workerEpoch`
- `kernelKey` + kernel fingerprint
- capability manifest/version
- representation + typed evidence/certificates
- kernel / tessellation / packing timings
- `TopologySnapshotId`, `MeshAssetId`, packet-local `PacketTopoToken` tables
- structured diagnostics
- backend-computed `fullEquivalent` (adapter may only *check*
  `fullEquivalent === !reduced`, never store two independent truths)

### Request family

Coordinator continues to see one atomic `build` family with discriminant
`InteractiveBuild | ExportCurrentSnapshot`. Export carries value
`TopologySnapshotId` and policy, never a serializable capability handle.
Reusable handles stay Worker-local.

### Migration

Atomic protocol bump only. Checked-in matrix:
`docs/qualification/geometry-scene-v2/v5-migration-matrix-v1.json`.

Transitional adapters may project GeometrySceneV2 → GeometryScene v1 for
renderer compatibility during G4a qualification. Reverse silent widening of
v1 scenes into claimed TopologySnapshotId evidence is forbidden.

### Non-goals

- Activating GeometrySceneV2 in production Worker/MCP under this ADR
- Creating `cad-*` crates
- Changing engine routing (ADR 0002)

## Consequences

- G0.8 can advance to partial with IDL + fixtures + independent checker
- G4a remains the activation gate
- ADR 0005 shadow-adapter / Worker identity boundary work must cite this
  contract, not the live MeshData integer alone

## Acceptance gates

G0.8 engineering-complete when:

1. this ADR is human-accepted;
2. schema + wire fixtures + migration matrix validate under
   `tests/support/referenceGeometrySceneV2.ts`;
3. every migration row states source shape, target shape, and fail-closed
   negatives;
4. no production path claims GeometrySceneV2 publication before G4a.
