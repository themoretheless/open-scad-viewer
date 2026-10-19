import {callGeometryRust} from './geometry/kernel'
import type { SculptBrush } from './geometryEditing'
import { booleanPolygonMeshes, brushPolygonMesh, sculptPolygonMesh, deformPolygonMesh, extrudePolygonProfile, exportPolygonStl, inspectPolygonMesh, normalizePolygonMesh, type PolygonMesh } from './geometry/polygon'
import { stringifyMeshJson } from './meshJson'
import { parseBinaryStl } from './stlImport'

export type MeshSelectMode = 'object' | 'vertex' | 'edge' | 'face'
export type MeshEditTool = 'select' | 'grab' | 'rotate' | 'scale' | 'extrude' | 'inset' | 'knife'

export interface MeshObject {
  id: string
  name: string
  mesh: PolygonMesh
  visible: boolean
}

export interface MeshWorkspaceDocument {
  version: 1
  objects: MeshObject[]
}

export const emptyMeshDocument = (): MeshWorkspaceDocument => ({ version: 1, objects: [] })

const clone = <T>(value: T): T => structuredClone(value)
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6

export function parseMeshDocument(text: string): MeshWorkspaceDocument {
  if (text.length > 8_000_000) throw new Error('Mesh document exceeds 8 MB.')
  const d = JSON.parse(text) as MeshWorkspaceDocument
  if (!d || d.version !== 1 || !Array.isArray(d.objects) || d.objects.length > 200) throw new Error('Invalid mesh document.')
  const ids = new Set<string>()
  for (const object of d.objects) {
    if (typeof object.id !== 'string' || ids.has(object.id) || typeof object.name !== 'string' || object.name.length > 100) {
      throw new Error('Invalid mesh object identity.')
    }
    ids.add(object.id)
    validatePolygon(object.mesh)
    if (typeof object.visible !== 'boolean') object.visible = true
  }
  return clone(d)
}

function validatePolygon(mesh: PolygonMesh) {
  if (!mesh || !Array.isArray(mesh.positions) || !Array.isArray(mesh.indices)
    || mesh.positions.length < 9 || mesh.positions.length > 300_000 || mesh.positions.length % 3
    || mesh.indices.length < 3 || mesh.indices.length > 300_000 || mesh.indices.length % 3
    || !mesh.positions.every(finite)
    || !mesh.indices.every(i => Number.isInteger(i) && i >= 0 && i < mesh.positions.length / 3)) {
    throw new Error('Invalid mesh geometry.')
  }
  // JSON boundary: plain parsed arrays are boxed into typed views exactly once.
  normalizePolygonMesh(mesh)
}

interface MeshSnapshot { document: MeshWorkspaceDocument; characters: number; text: string }
export class MeshHistory {
  private past: MeshSnapshot[] = []
  private future: MeshSnapshot[] = []
  private current: MeshSnapshot
  constructor(document = emptyMeshDocument()) {
    const validated = parseMeshDocument(stringifyMeshJson(document))
    const validatedText = stringifyMeshJson(validated)
    this.current = {document: validated, characters: validatedText.length, text: validatedText}
  }
  get document() { return clone(this.current.document) }
  get canUndo() { return this.past.length > 0 }
  get canRedo() { return this.future.length > 0 }
  commit(document: MeshWorkspaceDocument) {
    const next = parseMeshDocument(stringifyMeshJson(document))
    const nextText = stringifyMeshJson(next)
    if (nextText.length === this.current.characters && nextText === this.current.text) return
    this.past.push(this.current)
    // Preserve the exact serialized-array character budget, including brackets
    // and commas, without serializing retained geometry on every commit.
    let retained = this.past.reduce((sum, snapshot) => sum + snapshot.characters, 0) + this.past.length + 1
    while (this.past.length > 80 || (this.past.length > 1 && retained > 24_000_000)) {
      retained -= this.past.shift()!.characters + 1
    }
    this.current = {document: next, characters: nextText.length, text: nextText}
    this.future = []
  }
  undo() {
    const d = this.past.pop()
    if (d) { this.future.push(this.current); this.current = d }
    return this.document
  }
  redo() {
    const d = this.future.pop()
    if (d) { this.past.push(this.current); this.current = d }
    return this.document
  }
}

/** Weld STL triangle soup into an indexed PolygonMesh. */
export function stlBufferToPolygonMesh(buffer: ArrayBuffer, weld = 1e-4): PolygonMesh {
  const imported = parseBinaryStl(buffer)
  return normalizePolygonMesh(callGeometryRust<PolygonMesh>('mesh_import_finalize', {positions: imported.positions, weld}))
}

export function createBoxMesh(size: [number, number, number] = [10, 10, 10]): PolygonMesh {
  const [sx, sy, sz] = size
  const built = extrudePolygonProfile({
    outer: [[0, 0], [sx, 0], [sx, sy], [0, sy]],
  }, [0, 0, sz])
  return { positions: built.positions, indices: built.indices }
}

export function createUvSphereMesh(radius = 5, segments = 16, rings = 12): PolygonMesh {
  return nativeMeshEdit('sphere', {radius, segments, rings})
}

export function transformMesh(mesh: PolygonMesh, delta: [number, number, number], angleDeg: number, scale: number): PolygonMesh {
  return nativeMeshEdit('transform', {mesh, delta, angle: angleDeg, scale})
}

export function moveVertices(mesh: PolygonMesh, vertexIds: number[], delta: [number, number, number]): PolygonMesh {
  return nativeMeshEdit('move', {mesh, ids: vertexIds, delta, radius: null})
}

/**
 * Move selected vertices and nearby vertices with a smooth Euclidean falloff.
 * This is geometric proximity, not topology-distance propagation.
 */
export function moveVerticesProportional(
  mesh: PolygonMesh,
  vertexIds: number[],
  delta: [number, number, number],
  radius: number,
): PolygonMesh {
  return nativeMeshEdit('move', {mesh, ids: vertexIds, delta, radius})
}

export function deleteFaces(mesh: PolygonMesh, faceIds: number[]): PolygonMesh {
  return nativeMeshEdit('delete', {mesh, ids: faceIds})
}

export function flipFaces(mesh: PolygonMesh, faceIds?: number[]): PolygonMesh {
  return nativeMeshEdit('flip', {mesh, ids: faceIds ?? null})
}

/** One level of mid-edge subdivision for selected faces (or all). */
export function subdivideFaces(mesh: PolygonMesh, faceIds?: number[]): PolygonMesh {
  return nativeMeshEdit('subdivide', {mesh, ids: faceIds ?? null})
}

export function extrudeSelectedFaces(mesh: PolygonMesh, faceIds: number[], distance: number): PolygonMesh {
  return nativeMeshEdit('extrude', {mesh, ids: faceIds, distance})
}

export function insetSelectedFaces(mesh: PolygonMesh, faceIds: number[], amount: number): PolygonMesh {
  return nativeMeshEdit('inset', {mesh, ids: faceIds, amount})
}

export function mergeByDistance(mesh: PolygonMesh, distance = 1e-4): PolygonMesh {
  return nativeMeshEdit('merge', {mesh, distance})
}

export function booleanMeshObjects(a: PolygonMesh, b: PolygonMesh, operation: 'union' | 'difference' | 'intersection'): PolygonMesh {
  const result = booleanPolygonMeshes(a, b, operation)
  if (!result.indices.length) throw new Error('Boolean produced an empty mesh.')
  return { positions: result.positions, indices: result.indices }
}

export function duplicateObject(object: MeshObject, delta: [number, number, number] = [5, 0, 0]): MeshObject {
  return {
    id: `mesh-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
    name: `${object.name} copy`,
    mesh: transformMesh(object.mesh, delta, 0, 1),
    visible: true,
  }
}

export function meshObjectStats(mesh: PolygonMesh) {
  const report = inspectPolygonMesh(mesh)
  return {
    vertices: mesh.positions.length / 3,
    faces: mesh.indices.length / 3,
    closed: report.closed,
    volume: report.signedVolumeMm3,
  }
}

export function exportMeshObjectStl(mesh: PolygonMesh): string {
  return exportPolygonStl(mesh)
}

export function buildEdgeList(mesh: PolygonMesh): Array<[number, number]> {
  return callGeometryRust('mesh_editor', {action: 'edges', mesh})
}

/** Insert a shared midpoint on every selected edge and split all adjacent triangles. */
export function knifeSplitEdges(mesh: PolygonMesh, edgeIds: number[]): PolygonMesh {
  return nativeMeshEdit('knife', {mesh, ids: edgeIds})
}

/** Separate selected faces into a new object; remainder stays. */
export function separateFaces(mesh: PolygonMesh, faceIds: number[]): { kept: PolygonMesh; separated: PolygonMesh } {
  const result = callGeometryRust<{kept: PolygonMesh; separated: PolygonMesh}>('mesh_editor', {action: 'separate', mesh, ids: faceIds})
  return {kept: normalizePolygonMesh(result.kept), separated: normalizePolygonMesh(result.separated)}
}

export function joinMeshes(meshes: PolygonMesh[]): PolygonMesh {
  return nativeMeshEdit('join', {meshes})
}

export function symmetrizeMesh(mesh: PolygonMesh, axis: 0 | 1 | 2 = 0): PolygonMesh {
  return nativeMeshEdit('symmetrize', {mesh, axis})
}

export function brushDisplace(mesh: PolygonMesh, center: [number, number, number], radius: number, displacement: [number, number, number]): PolygonMesh {
  if (!finite(radius) || radius <= 0 || !center.every(finite) || !displacement.every(finite)) throw new Error('Invalid brush.')
  return brushPolygonMesh(mesh, { center, radius, displacement })
}

/** Applies a sculpt stroke (grab/draw/inflate/smooth/flatten/pinch) with falloff and symmetry. */
export function sculptMesh(mesh: PolygonMesh, brush: SculptBrush): PolygonMesh {
  return sculptPolygonMesh(mesh, brush)
}

/** Mean position of `vertices` (all vertices when omitted or empty). */
export function meshCentroid(mesh: PolygonMesh, vertices: readonly number[] = []): [number, number, number] {
  return callGeometryRust('mesh_editor', {action: 'centroid', mesh, ids: vertices})
}

export function twistMesh(mesh: PolygonMesh, radiansPerUnit: number, origin: [number, number, number] = [0, 0, 0]): PolygonMesh {
  if (!finite(radiansPerUnit) || !origin.every(finite)) throw new Error('Invalid twist.')
  return deformPolygonMesh(mesh, { kind: 'twist', origin, radians_per_unit: radiansPerUnit })
}

/** Numeric editing is owned by polygon-core; this adapter boxes the decoded buffers. */
function nativeMeshEdit(action: string, payload: Record<string, unknown>): PolygonMesh {
  return normalizePolygonMesh(callGeometryRust<PolygonMesh>('mesh_editor', {action, ...payload}))
}
