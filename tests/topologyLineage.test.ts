import { describe, expect, it } from 'vitest'
import { createNativeGeometryArtifact } from '../src/core/nativeGeometry'
import {
  TopologyLineage,
  assertNativeEdgeCurrent,
  nativeControlPointReference,
  nativeEdgeReference,
  topoIdFromParts,
} from '../src/core/topologyLineage'

describe('native topology lineage', () => {
  it('transfers edge selection across split/merge and rejects stale snapshots', () => {
    const parent = topoIdFromParts(1, 1)
    const left = topoIdFromParts(1, 2)
    const right = topoIdFromParts(1, 3)
    const merged = topoIdFromParts(1, 4)
    const lineage = new TopologyLineage()
    lineage.introduce(parent, 'edge')
    lineage.split(parent, [left, right])
    expect(lineage.transfer(parent)).toEqual({ status: 'ambiguous', ids: [left, right] })
    lineage.merge([left, right], merged)
    expect(lineage.transfer(left)).toEqual({ status: 'followed', id: merged })

    const snapshot = lineage.snapshot(merged)
    expect(snapshot.schema).toBe(1)
    expect(TopologyLineage.restore(snapshot).transfer(merged)).toEqual({
      status: 'persistent',
      id: merged,
    })

    const artifact = createNativeGeometryArtifact('body', 'mesh', { faces: 6 }, { source: 'cube' })
    const edge = nativeEdgeReference(artifact, merged)
    const point = nativeControlPointReference(artifact, parent)
    assertNativeEdgeCurrent(artifact, edge)
    expect(point.controlPointId).toBe(parent)
    const stale = createNativeGeometryArtifact('body', 'mesh', { faces: 7 }, { source: 'cube' })
    expect(() => assertNativeEdgeCurrent(stale, edge)).toThrow(/stale/)
  })
})
