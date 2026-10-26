import { describe, expect, it } from 'vitest'
import { createBoxMesh, deleteFaces, flipFaces } from '../src/services/meshEditing'
import { projectMeshModelerFaces, projectMeshModelerObject } from '../src/services/meshModelerProjection'
import { defaultDirectCamera, projectDirectPoint } from '../src/services/directModelingTools'
import { cachedMeshManifoldReport } from '../src/services/meshManifoldDefects'

describe('Mesh viewport closed-shell visibility', () => {
  it('never paints the far side of a cube over the near side throughout an orbit', () => {
    const mesh = createBoxMesh()
    for (const pitch of [-1.2, -0.4, 0.2, 0.8, 1.4]) {
      for (let step = 0; step < 72; step++) {
        const yaw = (step + 0.37) * Math.PI / 36
        const direction = [Math.sin(yaw) * Math.cos(pitch), Math.cos(yaw) * Math.cos(pitch), Math.sin(pitch)]
        const faces = projectMeshModelerFaces(mesh, { yaw, pitch })
        expect(faces).toHaveLength(8)
        for (const { face } of faces) {
          const ids = Array.from(mesh.indices.slice(face * 3, face * 3 + 3))
          const plane = [0, 1, 2].find(axis => ids.every(id => mesh.positions[id * 3 + axis] === mesh.positions[ids[0] * 3 + axis]))!
          expect(mesh.positions[ids[0] * 3 + plane]).toBe(direction[plane] > 0 ? 10 : 0)
        }
      }
    }
  })

  it('keeps the same visible faces when the entire closed shell has reversed winding', () => {
    const mesh = createBoxMesh(), camera = defaultDirectCamera()
    expect(projectMeshModelerFaces(flipFaces(mesh), camera).map(f => f.face))
      .toEqual(projectMeshModelerFaces(mesh, camera).map(f => f.face))
  })

  it('keeps open meshes and inconsistent winding two-sided', () => {
    const box = createBoxMesh()
    for (const mesh of [deleteFaces(box, [0]), flipFaces(box, [0])]) {
      for (const pitch of [-0.7, 0.7]) {
        expect(projectMeshModelerFaces(mesh, { yaw: 0.3, pitch })).toHaveLength(mesh.indices.length / 3)
      }
    }
  })
})

describe('Mesh viewport manifold defect overlay', () => {
  const openTetra = {
    positions: Float64Array.from([0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2]),
    indices: Uint32Array.from([0, 2, 1, 0, 1, 3, 1, 2, 3, 0, 3, 2]),
  }

  it('projects kernel boundary edges with the scene camera', () => {
    const camera = {yaw: 0.4, pitch: 0.2}
    const open = deleteFaces(openTetra, [3])
    const report = cachedMeshManifoldReport(open)
    const result = projectMeshModelerObject({mesh: open, visible: true}, camera, 'object', () => [], report)
    expect(result.defects.boundaryEdges).toHaveLength(3)
    expect(result.defects.nonManifoldEdges).toHaveLength(0)
    const a = report.boundaryEdges[0]
    const expected = projectDirectPoint(Array.from(open.positions.slice(a * 3, a * 3 + 3)), camera)
    expect(result.defects.boundaryEdges[0].x1).toBeCloseTo(expected[0])
    expect(result.defects.boundaryEdges[0].y1).toBeCloseTo(-expected[1])
    const closed = createBoxMesh()
    const clean = projectMeshModelerObject({mesh: closed, visible: true}, camera, 'object', () => [], cachedMeshManifoldReport(closed))
    expect(clean.defects).toEqual({boundaryEdges: [], nonManifoldEdges: [], nonManifoldVertices: []})
  })

  it('projects pinched vertices reported for touching closed shells', () => {
    const pinched = {
      positions: Float64Array.from([0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 2, -2, 0, 0, 0, -2, 0, 0, 0, -2]),
      indices: Uint32Array.from([0, 2, 1, 0, 1, 3, 1, 2, 3, 0, 3, 2, 0, 5, 4, 0, 4, 6, 4, 5, 6, 0, 6, 5]),
    }
    const report = cachedMeshManifoldReport(pinched)
    expect(Array.from(report.nonManifoldVertices)).toEqual([0])
    const camera = defaultDirectCamera()
    const result = projectMeshModelerObject({mesh: pinched, visible: true}, camera, 'object', () => [], report)
    expect(result.defects.nonManifoldVertices).toHaveLength(1)
    const expected = projectDirectPoint([0, 0, 0], camera)
    expect(result.defects.nonManifoldVertices[0]).toEqual({id: 0, x: expected[0], y: -expected[1]})
  })
})
