import type { MeshData } from '../core/mesh'
import type { DirectBody, DirectDocument } from './directModeling'
import { emptyDirectDocument } from './directModeling'
import type { PolygonMesh } from './geometry/polygon'
import type { MeshObject, MeshWorkspaceDocument } from './meshEditing'
import { emptyMeshDocument } from './meshEditing'
import { tessellateSolidNurbsSurface } from './solidNurbs'

/** Convert renderer meshes into a Solid (Direct) document of bodies. */
export function sceneMeshesToSolidDocument(meshes: readonly MeshData[], namePrefix = 'Body'): DirectDocument {
  const doc = emptyDirectDocument()
  meshes.forEach((mesh, index) => {
    const body = meshDataToPolygonBody(mesh, `${namePrefix} ${index + 1}`, `solid-${index + 1}-${Date.now().toString(36)}`)
    if (body) doc.bodies.push(body)
  })
  return doc
}

export function meshDataToPolygonBody(mesh: MeshData, name: string, id: string): DirectBody | null {
  const polygon = meshDataToPolygon(mesh)
  if (!polygon) return null
  return { id, name, mesh: polygon }
}

/** MeshData uses interleaved position+normal (stride 6). */
export function meshDataToPolygon(mesh: MeshData): PolygonMesh | null {
  const stride = 6
  const vertexCount = mesh.vertices.length / stride
  if (!Number.isInteger(vertexCount) || vertexCount < 3 || mesh.indices.length < 3) return null
  const positions: number[] = []
  for (let i = 0; i < vertexCount; i++) {
    const o = i * stride
    positions.push(mesh.vertices[o], mesh.vertices[o + 1], mesh.vertices[o + 2])
  }
  return { positions, indices: Array.from(mesh.indices) }
}

export function solidDocumentToMeshDocument(solid: DirectDocument): MeshWorkspaceDocument {
  const doc = emptyMeshDocument()
  for (const body of solid.bodies) {
    doc.objects.push(polygonToMeshObject(body.mesh, body.name, body.id))
  }
  for (const surface of solid.surfaces ?? []) {
    doc.objects.push(polygonToMeshObject(tessellateSolidNurbsSurface(surface), `${surface.name} · tessellated`, surface.id))
  }
  return doc
}

export function meshDocumentToSolidDocument(meshDoc: MeshWorkspaceDocument): DirectDocument {
  const doc = emptyDirectDocument()
  for (const object of meshDoc.objects) {
    doc.bodies.push({
      id: object.id,
      name: object.name,
      mesh: { positions: [...object.mesh.positions], indices: [...object.mesh.indices] },
    })
  }
  return doc
}

export function polygonToMeshObject(mesh: PolygonMesh, name: string, id: string): MeshObject {
  return {
    id,
    name,
    mesh: { positions: [...mesh.positions], indices: [...mesh.indices] },
    visible: true,
  }
}
