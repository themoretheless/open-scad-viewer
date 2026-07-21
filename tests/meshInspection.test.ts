import { describe, expect, it } from 'vitest'
import {
  computeHitNormal,
  computeHitWorldPoint,
  inspectMesh,
  matchMeshesByProvenance,
  resolveProvenance,
  summarizeMeasurement,
} from '../src/services/meshInspection'
import { identity, scale, translate, type Mat4 } from '../src/services/math3d'
import type {
  MeshData,
  MeshSourceReference,
  SceneEntityId,
  MeshTopologyDiagnostics,
} from '../src/core/mesh'

function makeMesh(options: {
  positions?: number[][]
  indices?: number[]
  transform?: Mat4
  topology?: MeshTopologyDiagnostics
  entityId?: SceneEntityId
} = {}): MeshData {
  const positions = options.positions ?? [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
  ]
  return {
    ...(options.entityId ? { entityId: options.entityId } : {}),
    vertices: Float32Array.from(positions.flatMap(([x, y, z]) => [x, y, z, 0, 0, 1])),
    indices: Uint32Array.from(options.indices ?? [0, 1, 2, 1, 3, 2]),
    color: [0.5, 0.6, 0.7, 1],
    transform: options.transform ?? identity(),
    faceIds: new Uint32Array(2),
    provenance: [],
    ...(options.topology ? { topology: options.topology } : {}),
  } as MeshData
}

describe('mesh inspection', () => {
  it('resolves provenance at half-open run boundaries', () => {
    const mesh = makeMesh()
    const cube: MeshSourceReference = {
      id: 2,
      originalId: 42,
      start: 2,
      end: 12,
      label: 'cube',
    }
    mesh.provenance = [
      { triangleStart: 0, triangleEnd: 1, source: cube, backside: false },
      { triangleStart: 1, triangleEnd: 2, source: null, backside: true },
    ]

    expect(resolveProvenance(mesh, 0)).toBe(mesh.provenance[0])
    expect(resolveProvenance(mesh, 1)).toBe(mesh.provenance[1])
    expect(resolveProvenance(mesh, -1)).toBeNull()
    expect(resolveProvenance(mesh, 1.5)).toBeNull()
    expect(resolveProvenance(mesh, 2)).toBeNull()
  })

  it('matches unique legacy provenance and rejects ambiguous duplicate keys', () => {
    const source = (id: number): MeshSourceReference => ({
      id,
      originalId: id + 100,
      start: id,
      end: id + 5,
      label: 'cube()',
    })
    const tagged = (id: number) => {
      const mesh = makeMesh()
      mesh.provenance = [{ triangleStart: 0, triangleEnd: 2, source: source(id), backside: false }]
      return mesh
    }
    const previous = [tagged(20), tagged(10), tagged(20), makeMesh()]
    const replacement = [tagged(10), tagged(20), tagged(20), makeMesh(), tagged(30)]

    expect(matchMeshesByProvenance(previous, replacement)).toEqual([1, -1, -1, 3, -1])
  })

  it('prefers stable entity identity when provenance is duplicated and scene order changes', () => {
    const sharedSource: MeshSourceReference = {
      id: 20,
      originalId: 120,
      start: 20,
      end: 25,
      label: 'cube()',
    }
    const tagged = (entityId: SceneEntityId) => {
      const mesh = makeMesh({ entityId })
      mesh.provenance = [{ triangleStart: 0, triangleEnd: 2, source: sharedSource, backside: false }]
      return mesh
    }
    const first = tagged('entity:first')
    const second = tagged('entity:second')

    expect(matchMeshesByProvenance([first, second], [second, first])).toEqual([1, 0])
  })

  it('does not fall back to shared provenance when stable entity identity changed', () => {
    const source: MeshSourceReference = {
      id: 20,
      originalId: 120,
      start: 20,
      end: 25,
      label: 'cube()',
    }
    const previous = makeMesh({ entityId: 'entity:old' })
    previous.provenance = [{ triangleStart: 0, triangleEnd: 2, source, backside: false }]
    const replacement = makeMesh({ entityId: 'entity:new' })
    replacement.provenance = [{ triangleStart: 0, triangleEnd: 2, source, backside: false }]

    expect(matchMeshesByProvenance([previous], [replacement])).toEqual([-1])
  })

  it('reports counts, transformed world bounds and topology', () => {
    const transform = translate(scale(identity(), [2, 3, 4]), [10, -2, 5])
    const topology = { boundary: 4, crease: 0, nonManifold: 0, degenerate: 1 }
    const mesh = makeMesh({
      positions: [
        [-1, 0, 2],
        [2, 0, 2],
        [-1, 4, -1],
        [2, 4, -1],
      ],
      transform,
      topology,
    })

    expect(inspectMesh(mesh, 7)).toEqual({
      index: 7,
      vertices: 4,
      triangles: 2,
      bounds: { min: [8, -2, 1], max: [14, 10, 13] },
      dimensions: [6, 12, 12],
      center: [11, 4, 7],
      topology,
    })
  })

  it('handles meshes without finite positions', () => {
    const mesh = makeMesh({
      positions: [[Number.NaN, 0, 0]],
      indices: [],
    })

    expect(inspectMesh(mesh, 0)).toMatchObject({
      vertices: 1,
      triangles: 0,
      bounds: null,
      dimensions: [0, 0, 0],
      center: null,
      topology: null,
    })
  })

  it('computes a barycentric world point and transformed face normal', () => {
    const transform = translate(scale(identity(), [2, 3, 4]), [10, -2, 5])
    const mesh = makeMesh({
      positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
      indices: [0, 1, 2],
      transform,
    })

    expect(computeHitWorldPoint(mesh, 0, [0.25, 0.25, 0.5])).toEqual([10.5, -0.5, 5])
    expect(computeHitNormal(mesh, 0)).toEqual([0, 0, 1])
    expect(computeHitWorldPoint(mesh, 1, [1, 0, 0])).toBeNull()
    expect(computeHitNormal(mesh, -1)).toBeNull()
  })

  it('rejects a normal for a degenerate transformed triangle', () => {
    const mesh = makeMesh({
      positions: [[0, 0, 0], [1, 0, 0], [2, 0, 0]],
      indices: [0, 1, 2],
    })
    expect(computeHitNormal(mesh, 0)).toBeNull()
  })

  it('summarizes signed axis deltas and Euclidean distance without aliasing inputs', () => {
    const start: [number, number, number] = [-1, 2, 5]
    const end: [number, number, number] = [2, -2, 17]
    const result = summarizeMeasurement(start, end)

    expect(result).toEqual({
      start: [-1, 2, 5],
      end: [2, -2, 17],
      delta: [3, -4, 12],
      deltaX: 3,
      deltaY: -4,
      deltaZ: 12,
      distance: 13,
    })
    expect(result.start).not.toBe(start)
    expect(result.end).not.toBe(end)
  })
})
