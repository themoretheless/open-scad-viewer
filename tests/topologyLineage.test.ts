import { describe, expect, it } from 'vitest'
import { createNativeGeometryArtifact } from '../src/core/nativeGeometry'
import {
  TopologyLineage,
  assertNativeEdgeCurrent,
  nativeControlPointReference,
  nativeEdgeReference,
  topoIdFromParts,
} from '../src/core/topologyLineage'
import { createSelectionTransferService } from '../src/services/geometry/selectionTransfer'

describe('native topology lineage', () => {
  it('transfers edge selection across split/merge and rejects stale snapshots', () => {
    const parent = topoIdFromParts(1, 1, 'edge')
    const left = topoIdFromParts(1, 2, 'edge')
    const right = topoIdFromParts(1, 3, 'edge')
    const merged = topoIdFromParts(1, 4, 'edge')
    const lineage = new TopologyLineage()
    lineage.introduce(parent, 'edge')
    lineage.split(parent, [left, right])
    expect(lineage.transfer(parent)).toEqual({
      status: 'confirmation-required',
      ids: [left, right],
    })
    lineage.merge([left, right], merged)
    expect(lineage.transfer(left)).toEqual({
      status: 'confirmation-required',
      ids: [merged],
    })

    const snapshot = lineage.snapshot(merged)
    expect(snapshot.schema).toBe(2)
    expect(TopologyLineage.restore(snapshot).transfer(merged)).toEqual({
      status: 'persistent',
      id: merged,
    })

    const artifact = createNativeGeometryArtifact('body', 'mesh', { faces: 6 }, { source: 'cube' })
    const edge = nativeEdgeReference(artifact, merged)
    const pointId = topoIdFromParts(2, 1, 'control-point')
    const point = nativeControlPointReference(artifact, pointId)
    assertNativeEdgeCurrent(artifact, edge)
    expect(point.controlPointId).toBe(pointId)
    const stale = createNativeGeometryArtifact('body', 'mesh', { faces: 7 }, { source: 'cube' })
    expect(() => assertNativeEdgeCurrent(stale, edge)).toThrow(/stale/)
  })

  it('uses authoritative Rust changes across revisions and preserves anchors', () => {
    const a = topoIdFromParts(4, 1, 'face')
    const b = topoIdFromParts(4, 2, 'face')
    const c = topoIdFromParts(4, 3, 'face')
    const service = createSelectionTransferService()
    service.applyRustChangeSet({
      schema: 1,
      nodes: [{ id: a, kind: 'face' }, { id: b, kind: 'face' }],
      changes: [{
        kind: 'modified',
        topoKind: 'face',
        parents: [a],
        children: [b],
        provenance: { operation: 'push-face', operand: null, occurrence: 'occ-1' },
        role: 'primary',
        anchor: 'top-cap',
      }],
    })
    service.applyRustChangeSet({
      schema: 1,
      nodes: [{ id: b, kind: 'face' }, { id: c, kind: 'face' }],
      changes: [{
        kind: 'modified',
        topoKind: 'face',
        parents: [b],
        children: [c],
        provenance: { operation: 'resize', operand: null, occurrence: 'occ-2' },
        role: 'primary',
        anchor: 'top-cap',
      }],
    })
    expect(service.transfer(a)).toEqual({ status: 'followed', id: c })
    const snapshot = service.snapshot(c)
    const restored = createSelectionTransferService()
    restored.restore(snapshot)
    expect(restored.roleOf(c)).toBe('primary')
    expect(restored.anchorOf(c)?.parameterHint).toBe('top-cap')
    expect(restored.transfer(a)).toEqual({ status: 'followed', id: c })
  })
})
