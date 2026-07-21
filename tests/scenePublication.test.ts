import { describe, expect, it } from 'vitest'
import type { MeshData, SceneEntityId } from '../src/core/mesh'
import { planScenePublication } from '../src/services/scenePublication'

const IDENTITY = Float32Array.from([
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
])

function mesh(entityId: SceneEntityId): MeshData {
  return {
    entityId,
    vertices: new Float32Array(),
    indices: new Uint32Array(),
    bvh: {
      version: 1,
      vertexStride: 6,
      leafSize: 1,
      nodeCount: 0,
      bounds: new Float32Array(),
      nodes: new Uint32Array(),
      triangles: new Uint32Array(),
    },
    edgeIndices: new Uint32Array(),
    color: [0.5, 0.6, 0.7, 1],
    transform: IDENTITY.slice(),
    faceIds: new Uint32Array(),
    provenance: [],
    topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

describe('scene publication', () => {
  it('preserves state from preview to full for the exact same snapshot', () => {
    const previousMeshes = [
      mesh('entity:root/cube%230'),
      mesh('entity:root/cube%231'),
    ]
    const nextMeshes = [
      mesh('entity:root/cube%230'),
      mesh('entity:root/cube%231'),
    ]

    expect(planScenePublication({
      previousMeshes,
      previousVisibility: [false, true],
      previousSelectedIndex: 1,
      previousIsolated: true,
      nextMeshes,
      sameSourceSnapshot: true,
    })).toEqual({
      previousIndexByNextIndex: [0, 1],
      nextVisibility: [false, true],
      nextSelectedIndex: 1,
      nextIsolated: true,
      measurementMayBePreserved: true,
    })
  })

  it('follows distinct operation identities through a cross-revision reorder', () => {
    const cube = mesh('entity:root/cube%230')
    const sphere = mesh('entity:root/sphere%230')

    expect(planScenePublication({
      previousMeshes: [cube, sphere],
      previousVisibility: [false, true],
      previousSelectedIndex: 1,
      previousIsolated: true,
      nextMeshes: [sphere, cube],
      sameSourceSnapshot: false,
    })).toEqual({
      previousIndexByNextIndex: [1, 0],
      nextVisibility: [true, false],
      nextSelectedIndex: 0,
      nextIsolated: true,
      measurementMayBePreserved: false,
    })
  })

  it('fails closed for ambiguous same-name edits across revisions', () => {
    const first = mesh('entity:root/cube%230')
    const second = mesh('entity:root/cube%231')

    expect(planScenePublication({
      previousMeshes: [first, second],
      previousVisibility: [false, false],
      previousSelectedIndex: 0,
      previousIsolated: true,
      // Positional IDs can stay equal even when the two cube statements swap.
      nextMeshes: [mesh(first.entityId!), mesh(second.entityId!)],
      sameSourceSnapshot: false,
    })).toEqual({
      previousIndexByNextIndex: [-1, -1],
      nextVisibility: [true, true],
      nextSelectedIndex: null,
      nextIsolated: false,
      measurementMayBePreserved: false,
    })
  })

  it('keeps surviving visibility while dropping a hidden selection and isolation', () => {
    const cube = mesh('entity:root/cube%230')
    const sphere = mesh('entity:root/sphere%230')
    const cylinder = mesh('entity:root/cylinder%230')
    const cone = mesh('entity:root/cone%230')

    expect(planScenePublication({
      previousMeshes: [cube, sphere, cylinder],
      previousVisibility: [true, true, false],
      previousSelectedIndex: 2,
      previousIsolated: true,
      nextMeshes: [cone, cylinder],
      sameSourceSnapshot: false,
    })).toEqual({
      previousIndexByNextIndex: [-1, 2],
      nextVisibility: [true, false],
      nextSelectedIndex: null,
      nextIsolated: false,
      measurementMayBePreserved: false,
    })
  })

  it('keeps a visible surviving selection while new objects use safe defaults', () => {
    const cube = mesh('entity:root/cube%230')
    const sphere = mesh('entity:root/sphere%230')
    const cylinder = mesh('entity:root/cylinder%230')

    expect(planScenePublication({
      previousMeshes: [cube, sphere],
      previousVisibility: [true, true],
      previousSelectedIndex: 1,
      previousIsolated: true,
      nextMeshes: [cylinder, sphere],
      sameSourceSnapshot: false,
    })).toMatchObject({
      previousIndexByNextIndex: [-1, 1],
      nextVisibility: [true, true],
      nextSelectedIndex: 1,
      nextIsolated: true,
    })
  })

  it('drops selection and isolation when the selected object was removed', () => {
    const cube = mesh('entity:root/cube%230')
    const sphere = mesh('entity:root/sphere%230')

    expect(planScenePublication({
      previousMeshes: [cube, sphere],
      previousVisibility: [false, true],
      previousSelectedIndex: 0,
      previousIsolated: true,
      nextMeshes: [sphere],
      sameSourceSnapshot: false,
    })).toMatchObject({
      previousIndexByNextIndex: [1],
      nextVisibility: [true],
      nextSelectedIndex: null,
      nextIsolated: false,
    })
  })
})
