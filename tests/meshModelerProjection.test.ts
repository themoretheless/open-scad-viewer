import { describe, expect, it } from 'vitest'
import { createBoxMesh, deleteFaces, flipFaces } from '../src/services/meshEditing'
import { projectMeshModelerFaces } from '../src/services/meshModelerProjection'
import { defaultDirectCamera } from '../src/services/directModelingTools'

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
