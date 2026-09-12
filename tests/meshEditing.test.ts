import { describe, expect, it } from 'vitest'
import {
  MeshHistory,
  booleanMeshObjects,
  createBoxMesh,
  deleteFaces,
  emptyMeshDocument,
  extrudeSelectedFaces,
  mergeByDistance,
  moveVertices,
  parseMeshDocument,
  separateFaces,
  subdivideFaces,
  symmetrizeMesh,
  transformMesh,
} from '../src/services/meshEditing'
import { isWorkspaceMode, WORKSPACE_MODES } from '../src/services/workspaceModes'
import { meshDocumentToSolidDocument, solidDocumentToMeshDocument } from '../src/services/solidBridge'
import { emptyDirectDocument } from '../src/services/directModeling'

describe('workspace modes', () => {
  it('exposes Code · Solid · Mesh', () => {
    expect(WORKSPACE_MODES).toEqual(['code', 'solid', 'mesh'])
    expect(isWorkspaceMode('solid')).toBe(true)
    expect(isWorkspaceMode('assembly')).toBe(false)
  })
})

describe('mesh editing', () => {
  it('transforms, subdivides, moves vertices and deletes faces', () => {
    const box = createBoxMesh([10, 10, 10])
    expect(box.indices.length / 3).toBeGreaterThanOrEqual(12)
    const moved = transformMesh(box, [1, 0, 0], 0, 1)
    expect(moved.positions[0]).toBeCloseTo(box.positions[0] + 1)
    const divided = subdivideFaces(box)
    expect(divided.indices.length).toBeGreaterThan(box.indices.length)
    const grabbed = moveVertices(box, [0], [0, 0, 2])
    expect(grabbed.positions[2]).toBeCloseTo(box.positions[2] + 2)
    const trimmed = deleteFaces(box, [0])
    expect(trimmed.indices.length / 3).toBe(box.indices.length / 3 - 1)
  })

  it('merges coincident vertices and supports history', () => {
    const mesh = {
      positions: [0, 0, 0, 1e-5, 0, 0, 0, 1, 0, 0, 0, 1],
      indices: [0, 2, 3, 1, 2, 3],
    }
    const merged = mergeByDistance(mesh, 1e-4)
    expect(merged.positions.length / 3).toBeLessThan(mesh.positions.length / 3)
    const history = new MeshHistory(emptyMeshDocument())
    const next = history.document
    next.objects.push({ id: 'a', name: 'Box', mesh: createBoxMesh(), visible: true })
    history.commit(next)
    expect(history.document.objects).toHaveLength(1)
    history.undo()
    expect(history.document.objects).toHaveLength(0)
  })

  it('boolean unions two boxes', () => {
    const a = transformMesh(createBoxMesh([10, 10, 10]), [0, 0, 0], 0, 1)
    const b = transformMesh(createBoxMesh([10, 10, 10]), [5, 0, 0], 0, 1)
    const united = booleanMeshObjects(a, b, 'union')
    expect(united.indices.length).toBeGreaterThan(0)
  })

  it('extrudes a face via the geometry kernel', () => {
    const box = createBoxMesh([10, 10, 10])
    const extruded = extrudeSelectedFaces(box, [0], 2)
    expect(extruded.indices.length).toBeGreaterThan(box.indices.length)
  })

  it('round-trips mesh ↔ solid documents', () => {
    const meshDoc = emptyMeshDocument()
    meshDoc.objects.push({ id: 'm1', name: 'Cube', mesh: createBoxMesh(), visible: true })
    const solid = meshDocumentToSolidDocument(meshDoc)
    expect(solid.bodies).toHaveLength(1)
    const back = solidDocumentToMeshDocument(solid)
    expect(back.objects[0].mesh.indices).toEqual(meshDoc.objects[0].mesh.indices)
    expect(parseMeshDocument(JSON.stringify(back)).version).toBe(1)
    expect(emptyDirectDocument().bodies).toEqual([])
  })

  it('separates faces and symmetrizes', () => {
    const box = createBoxMesh([10, 10, 10])
    const { kept, separated } = separateFaces(box, [0, 1])
    expect(kept.indices.length).toBeGreaterThan(0)
    expect(separated.indices.length).toBeGreaterThan(0)
    const sym = symmetrizeMesh(createBoxMesh([4, 4, 4]), 0)
    expect(sym.indices.length).toBeGreaterThan(createBoxMesh([4, 4, 4]).indices.length)
  })
})
