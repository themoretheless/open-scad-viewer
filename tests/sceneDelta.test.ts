import { describe, expect, it } from 'vitest'
import { carryInteraction, diffSceneEntities, previousSceneIntact, type DeltaMesh } from '../src/render/sceneDelta'
import type { PickHit } from '../src/render/rendererContracts'

const identity = () => new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
const vbA = {},
  vbB = {}
const mesh = (
  entityId: string | undefined,
  assetId: string | undefined,
  vb: object,
  transform = identity(),
): DeltaMesh => ({
  entityId,
  assetId,
  vb,
  transform,
  faceIds: new Uint32Array([1]),
})
const hit = (meshIndex: number): PickHit => ({
  meshIndex,
  triangleIndex: 0,
  faceId: 1,
  point: [0, 0, 0],
  normal: [0, 0, 1],
  barycentric: [1, 0, 0],
  source: null,
  backside: false,
  cycleIndex: 1,
  cycleCount: 3,
})

describe('scene delta', () => {
  it('classifies entities by id, geometry and transform', () => {
    const moved = identity()
    moved[3] = 5
    const previous = [
      mesh('entity:a', 'asset:a', vbA),
      mesh('entity:b', 'asset:b', vbB),
      mesh('entity:c', 'asset:a', vbA),
      mesh(undefined, undefined, vbA),
    ]
    const next = [
      mesh('entity:c', 'asset:a', {}, moved),
      mesh('entity:b', 'asset:x', {}),
      mesh('entity:a', 'asset:a', {}),
      mesh('entity:new', 'asset:a', vbA),
    ]
    const delta = diffSceneEntities(previous, next)
    expect([...delta.stable]).toEqual([[0, 2]])
    expect(delta.transformChanged).toEqual([0])
    expect(delta.geometryChanged).toEqual([1])
    expect(delta.added).toEqual([3])
    expect(delta.removed).toEqual([3])
    expect(previousSceneIntact(delta, previous.length)).toBe(false)
    expect(previousSceneIntact(diffSceneEntities(previous.slice(0, 1), next), 1)).toBe(true)
  })

  it('treats changed face ids or native snapshots as changed geometry', () => {
    const previous = [mesh('entity:a', 'asset:a', vbA)]
    expect(diffSceneEntities(previous, [{ ...previous[0], faceIds: new Uint32Array([2]) }]).geometryChanged).toEqual([
      0,
    ])
    const native = (geometryJson: string) =>
      ({
        version: 1,
        nodeId: 'n',
        kind: 'patch',
        revision: 1,
        geometryJson,
        documentJson: '{}',
        documentRevision: 1,
      }) as unknown as DeltaMesh['nativeGeometry']
    const withNative = [{ ...previous[0], nativeGeometry: native('{"a":1}') }]
    expect(diffSceneEntities(withNative, [{ ...previous[0], nativeGeometry: native('{"a":1}') }]).stable.size).toBe(1)
    expect(
      diffSceneEntities(withNative, [{ ...previous[0], nativeGeometry: native('{"a":2}') }]).geometryChanged,
    ).toEqual([0])
  })

  it('carries selection, hover and isolation to new indices of stable entities', () => {
    const previous = [mesh('entity:a', 'asset:a', vbA), mesh('entity:b', 'asset:b', vbB)]
    const next = [mesh('entity:b', 'asset:b', vbB), mesh('entity:a', 'asset:a', vbA)]
    const carried = carryInteraction(
      { selected: 0, selectedHit: hit(0), hovered: 1, hoveredHit: null, isolated: true },
      diffSceneEntities(previous, next),
    )
    expect(carried.selected).toBe(1)
    expect(carried.selectedHit).toMatchObject({ meshIndex: 1, faceId: 1 })
    // Depth-cycle metadata never survives a publication.
    expect(carried.selectedHit).not.toHaveProperty('cycleIndex')
    expect(carried.hovered).toBe(0)
    expect(carried.isolated).toBe(true)
  })

  it('clears targets whose entity changed or left, and everything for a null delta', () => {
    const previous = [mesh('entity:a', 'asset:a', vbA), mesh('entity:b', 'asset:b', vbB)]
    const state = { selected: 0, selectedHit: hit(0), hovered: 1, hoveredHit: hit(1), isolated: true }
    const changed = carryInteraction(state, diffSceneEntities(previous, [mesh('entity:a', 'asset:z', {})]))
    expect(changed).toEqual({ selected: null, selectedHit: null, hovered: null, hoveredHit: null, isolated: false })
    expect(carryInteraction(state, null)).toEqual(changed)
  })
})
