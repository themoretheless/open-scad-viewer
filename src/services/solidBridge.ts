import type { MeshData } from '../core/mesh'
import {isNativeGeometryArtifact} from '../core/nativeGeometry'
import type { DirectBody, DirectDocument } from './directModeling'
import { emptyDirectDocument, parseDirectDocument } from './directModeling'
import {inspectNurbsBrep, transformNurbsBrep, type NurbsBrep} from './geometry/brep'
import type { PolygonMesh } from './geometry/polygon'
import type { MeshObject, MeshWorkspaceDocument } from './meshEditing'
import { emptyMeshDocument } from './meshEditing'
import { tessellateSolidNurbsSurface } from './solidNurbs'
import {placeSolidMeshInKernel} from './geometry/meshAnalysis'
import { createSelectionTransferService } from './geometry/selectionTransfer'
import { isTopoId, type TopoId } from '../core/topologyLineage'

/** N1: transfer a Solid selection across a rebuild using durable lineage only. */
export function transferSolidSelection(selection: string, lineageJson?: string): string | null {
  if (!isTopoId(selection)) return null
  const service = createSelectionTransferService()
  if (lineageJson) {
    try {
      service.restore(JSON.parse(lineageJson))
    } catch {
      return null
    }
  } else {
    service.introduce(selection as TopoId, 'face')
  }
  const result = service.transfer(selection as TopoId)
  if (result.status === 'persistent' || result.status === 'followed') return result.id
  return null
}

/** Convert the scene while retaining authored B-rep carriers and display placement. */
export function sceneMeshesToSolidDocument(meshes: readonly MeshData[], namePrefix = 'Body'): DirectDocument {
  const doc = emptyDirectDocument()
  meshes.forEach((mesh, index) => {
    const body = meshDataToPolygonBody(mesh, `${namePrefix} ${index + 1}`, `solid-${index + 1}-${Date.now().toString(36)}`)
    if (body) doc.bodies.push(body)
  })
  return parseDirectDocument(JSON.stringify(doc))
}

export function meshDataToPolygonBody(mesh: MeshData, name: string, id: string): DirectBody | null {
  const polygon = meshDataToPolygon(mesh)
  if (mesh.nativeGeometry?.kind === 'brep') {
    if (!isNativeGeometryArtifact(mesh.nativeGeometry)) throw new Error('Invalid native B-rep snapshot.')
    const data = JSON.parse(mesh.nativeGeometry.geometryJson) as {geometry?: NurbsBrep}
    if (!data || !data.geometry) throw new Error('Missing native B-rep geometry.')
    const model = data.geometry
    inspectNurbsBrep(model)
    const empty = [model.vertices,model.edges,model.loops,model.faces,model.shells,model.bodies].every(items=>items.length===0)
    if (empty) {
      if (polygon) throw new Error('An empty B-rep cannot have a displayed Solid body.')
      return null
    }
    if (!polygon) throw new Error('A nonempty B-rep requires a display mesh for Solid.')
    const identity = mesh.transform.every((v, i) => v === (i % 5 === 0 ? 1 : 0))
    const brep = identity ? model : transformNurbsBrep(model,
      Array.from({length:4}, (_, row) => Array.from(mesh.transform.slice(row*4,row*4+4))))
    return {id, name, mesh:polygon, brep}
  }
  if (!polygon) return null
  return { id, name, mesh: polygon }
}

/** MeshData uses interleaved position+normal (stride 6). */
export function meshDataToPolygon(mesh: MeshData): PolygonMesh | null {
  return placeSolidMeshInKernel(mesh.vertices, mesh.indices, mesh.transform)
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
