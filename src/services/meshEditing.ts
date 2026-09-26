import type { SculptBrush } from './geometryEditing'
import { booleanPolygonMeshes, brushPolygonMesh, sculptPolygonMesh, deformPolygonMesh, extrudePolygonFaces, extrudePolygonProfile, exportPolygonStl, inspectPolygonMesh, normalizePolygonMesh, type PolygonMesh } from './geometry/polygon'
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
  const positions: number[] = []
  const indices: number[] = []
  const map = new Map<string, number>()
  const keyOf = (x: number, y: number, z: number) => {
    const s = 1 / weld
    return `${Math.round(x * s)},${Math.round(y * s)},${Math.round(z * s)}`
  }
  for (let t = 0; t < imported.triangleCount; t++) {
    const o = t * 9
    const tri: number[] = []
    for (let v = 0; v < 3; v++) {
      const x = imported.positions[o + v * 3], y = imported.positions[o + v * 3 + 1], z = imported.positions[o + v * 3 + 2]
      const key = keyOf(x, y, z)
      let index = map.get(key)
      if (index === undefined) {
        index = positions.length / 3
        map.set(key, index)
        positions.push(x, y, z)
      }
      tri.push(index)
    }
    if (tri[0] !== tri[1] && tri[1] !== tri[2] && tri[2] !== tri[0]) indices.push(tri[0], tri[1], tri[2])
  }
  if (indices.length < 3) throw new Error('STL produced no usable triangles.')
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(indices) }
}

export function createBoxMesh(size: [number, number, number] = [10, 10, 10]): PolygonMesh {
  const [sx, sy, sz] = size
  const built = extrudePolygonProfile({
    outer: [[0, 0], [sx, 0], [sx, sy], [0, sy]],
  }, [0, 0, sz])
  return { positions: built.positions, indices: built.indices }
}

export function createUvSphereMesh(radius = 5, segments = 16, rings = 12): PolygonMesh {
  const positions: number[] = []
  const indices: number[] = []
  for (let y = 0; y <= rings; y++) {
    const v = y / rings, phi = v * Math.PI
    for (let x = 0; x <= segments; x++) {
      const u = x / segments, theta = u * Math.PI * 2
      positions.push(
        radius * Math.sin(phi) * Math.cos(theta),
        radius * Math.cos(phi),
        radius * Math.sin(phi) * Math.sin(theta),
      )
    }
  }
  for (let y = 0; y < rings; y++) {
    for (let x = 0; x < segments; x++) {
      const a = y * (segments + 1) + x
      const b = a + segments + 1
      indices.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(indices) }
}

export function transformMesh(mesh: PolygonMesh, delta: [number, number, number], angleDeg: number, scale: number): PolygonMesh {
  if (!delta.every(finite) || !finite(angleDeg) || !finite(scale) || scale <= 0) throw new Error('Invalid transform.')
  const count = mesh.positions.length / 3
  const center = [0, 0, 0]
  for (let i = 0; i < count; i++) {
    center[0] += mesh.positions[i * 3]
    center[1] += mesh.positions[i * 3 + 1]
    center[2] += mesh.positions[i * 3 + 2]
  }
  center[0] /= count; center[1] /= count; center[2] /= count
  const a = angleDeg * Math.PI / 180, c = Math.cos(a), s = Math.sin(a)
  const positions = mesh.positions.slice()
  for (let i = 0; i < count; i++) {
    let x = (positions[i * 3] - center[0]) * scale
    let y = (positions[i * 3 + 1] - center[1]) * scale
    const z = (positions[i * 3 + 2] - center[2]) * scale
    const rx = c * x - s * y, ry = s * x + c * y
    positions[i * 3] = center[0] + rx + delta[0]
    positions[i * 3 + 1] = center[1] + ry + delta[1]
    positions[i * 3 + 2] = center[2] + z + delta[2]
  }
  return { positions, indices: mesh.indices.slice() }
}

export function moveVertices(mesh: PolygonMesh, vertexIds: number[], delta: [number, number, number]): PolygonMesh {
  if (!delta.every(finite)) throw new Error('Invalid delta.')
  const positions = mesh.positions.slice()
  for (const id of vertexIds) {
    if (!Number.isInteger(id) || id < 0 || id >= positions.length / 3) throw new Error('Invalid vertex.')
    positions[id * 3] += delta[0]
    positions[id * 3 + 1] += delta[1]
    positions[id * 3 + 2] += delta[2]
  }
  return { positions, indices: mesh.indices.slice() }
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
  if (!delta.every(finite) || !finite(radius) || radius <= 0) throw new Error('Invalid proportional transform.')
  const selected = [...new Set(vertexIds)]
  if (!selected.length) throw new Error('Select vertices for proportional editing.')
  for (const id of selected) {
    if (!Number.isInteger(id) || id < 0 || id >= mesh.positions.length / 3) throw new Error('Invalid vertex.')
  }
  const positions = mesh.positions.slice()
  for (let id = 0; id < mesh.positions.length / 3; id++) {
    let distance = Infinity
    for (const selectedId of selected) {
      distance = Math.min(distance, Math.hypot(
        mesh.positions[id * 3] - mesh.positions[selectedId * 3],
        mesh.positions[id * 3 + 1] - mesh.positions[selectedId * 3 + 1],
        mesh.positions[id * 3 + 2] - mesh.positions[selectedId * 3 + 2],
      ))
    }
    if (distance >= radius) continue
    const linear = 1 - distance / radius
    const weight = linear * linear * (3 - 2 * linear)
    positions[id * 3] += delta[0] * weight
    positions[id * 3 + 1] += delta[1] * weight
    positions[id * 3 + 2] += delta[2] * weight
  }
  return { positions, indices: mesh.indices.slice() }
}

export function deleteFaces(mesh: PolygonMesh, faceIds: number[]): PolygonMesh {
  const remove = new Set(faceIds)
  const indices: number[] = []
  for (let f = 0; f < mesh.indices.length / 3; f++) {
    if (remove.has(f)) continue
    indices.push(mesh.indices[f * 3], mesh.indices[f * 3 + 1], mesh.indices[f * 3 + 2])
  }
  if (indices.length < 3) throw new Error('Cannot delete all faces.')
  return compactMesh({ positions: mesh.positions.slice(), indices: Uint32Array.from(indices) })
}

export function flipFaces(mesh: PolygonMesh, faceIds?: number[]): PolygonMesh {
  const indices = mesh.indices.slice()
  const faces = faceIds ?? Array.from({ length: indices.length / 3 }, (_, i) => i)
  for (const f of faces) {
    const o = f * 3
    const t = indices[o + 1]
    indices[o + 1] = indices[o + 2]
    indices[o + 2] = t
  }
  return { positions: mesh.positions.slice(), indices }
}

/** One level of mid-edge subdivision for selected faces (or all). */
export function subdivideFaces(mesh: PolygonMesh, faceIds?: number[]): PolygonMesh {
  const selected = new Set(faceIds ?? Array.from({ length: mesh.indices.length / 3 }, (_, i) => i))
  // Growable staging: typed arrays are fixed-length, so subdivision builds in a
  // plain array and boxes once at return.
  const positions: number[] = [...mesh.positions]
  const indices: number[] = []
  const midpoint = new Map<string, number>()
  const mid = (a: number, b: number) => {
    const key = a < b ? `${a}:${b}` : `${b}:${a}`
    let id = midpoint.get(key)
    if (id !== undefined) return id
    id = positions.length / 3
    positions.push(
      (mesh.positions[a * 3] + mesh.positions[b * 3]) / 2,
      (mesh.positions[a * 3 + 1] + mesh.positions[b * 3 + 1]) / 2,
      (mesh.positions[a * 3 + 2] + mesh.positions[b * 3 + 2]) / 2,
    )
    midpoint.set(key, id)
    return id
  }
  for (let f = 0; f < mesh.indices.length / 3; f++) {
    const a = mesh.indices[f * 3], b = mesh.indices[f * 3 + 1], c = mesh.indices[f * 3 + 2]
    if (!selected.has(f)) {
      indices.push(a, b, c)
      continue
    }
    const ab = mid(a, b), bc = mid(b, c), ca = mid(c, a)
    indices.push(a, ab, ca, ab, b, bc, ca, bc, c, ab, bc, ca)
  }
  if (positions.length > 300_000 || indices.length > 300_000) throw new Error('Subdivision exceeds mesh budget.')
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(indices) }
}

export function extrudeSelectedFaces(mesh: PolygonMesh, faceIds: number[], distance: number): PolygonMesh {
  if (!finite(distance) || faceIds.length === 0) throw new Error('Choose faces and a finite distance.')
  const normals = faceIds.map(f => faceNormal(mesh, f))
  const average = normals.reduce((s, n) => [s[0] + n[0], s[1] + n[1], s[2] + n[2]] as [number, number, number], [0, 0, 0] as [number, number, number])
  const len = Math.hypot(average[0], average[1], average[2]) || 1
  const vector = [average[0] / len * distance, average[1] / len * distance, average[2] / len * distance]
  return extrudePolygonFaces(mesh, faceIds, vector)
}

export function insetSelectedFaces(mesh: PolygonMesh, faceIds: number[], amount: number): PolygonMesh {
  if (!finite(amount) || amount <= 0 || faceIds.length === 0) throw new Error('Inset requires positive amount and faces.')
  // Approximate inset: scale face vertices toward face centroid, then keep connectivity.
  const positions = mesh.positions.slice()
  const moved = new Set<number>()
  for (const f of faceIds) {
    const a = mesh.indices[f * 3], b = mesh.indices[f * 3 + 1], c = mesh.indices[f * 3 + 2]
    const cx = (positions[a * 3] + positions[b * 3] + positions[c * 3]) / 3
    const cy = (positions[a * 3 + 1] + positions[b * 3 + 1] + positions[c * 3 + 1]) / 3
    const cz = (positions[a * 3 + 2] + positions[b * 3 + 2] + positions[c * 3 + 2]) / 3
    for (const id of [a, b, c]) {
      if (moved.has(id)) continue
      moved.add(id)
      const vx = positions[id * 3] - cx, vy = positions[id * 3 + 1] - cy, vz = positions[id * 3 + 2] - cz
      const len = Math.hypot(vx, vy, vz) || 1
      const t = Math.min(amount / len, 0.95)
      positions[id * 3] -= vx * t
      positions[id * 3 + 1] -= vy * t
      positions[id * 3 + 2] -= vz * t
    }
  }
  return { positions, indices: mesh.indices.slice() }
}

export function mergeByDistance(mesh: PolygonMesh, distance = 1e-4): PolygonMesh {
  if (!finite(distance) || distance <= 0) throw new Error('Merge distance must be positive.')
  const map = new Map<string, number>()
  const positions: number[] = []
  const remap: number[] = []
  const s = 1 / distance
  for (let i = 0; i < mesh.positions.length / 3; i++) {
    const x = mesh.positions[i * 3], y = mesh.positions[i * 3 + 1], z = mesh.positions[i * 3 + 2]
    const key = `${Math.round(x * s)},${Math.round(y * s)},${Math.round(z * s)}`
    let id = map.get(key)
    if (id === undefined) {
      id = positions.length / 3
      map.set(key, id)
      positions.push(x, y, z)
    }
    remap[i] = id
  }
  const indices: number[] = []
  for (let f = 0; f < mesh.indices.length / 3; f++) {
    const a = remap[mesh.indices[f * 3]], b = remap[mesh.indices[f * 3 + 1]], c = remap[mesh.indices[f * 3 + 2]]
    if (a !== b && b !== c && c !== a) indices.push(a, b, c)
  }
  if (indices.length < 3) throw new Error('Merge removed all faces.')
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(indices) }
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
  const edges = new Set<string>()
  const list: Array<[number, number]> = []
  for (let f = 0; f < mesh.indices.length / 3; f++) {
    const a = mesh.indices[f * 3], b = mesh.indices[f * 3 + 1], c = mesh.indices[f * 3 + 2]
    for (const [u, v] of [[a, b], [b, c], [c, a]] as const) {
      const key = u < v ? `${u}:${v}` : `${v}:${u}`
      if (edges.has(key)) continue
      edges.add(key)
      list.push(u < v ? [u, v] : [v, u])
    }
  }
  return list
}

/** Insert a shared midpoint on every selected edge and split all adjacent triangles. */
export function knifeSplitEdges(mesh: PolygonMesh, edgeIds: number[]): PolygonMesh {
  if (!edgeIds.length) throw new Error('Select edges to cut.')
  const sourceEdges = buildEdgeList(mesh)
  const selected = [...new Set(edgeIds)].map(edgeId => {
    const edge = sourceEdges[edgeId]
    if (!edge) throw new Error('Selected edge is out of range.')
    return edge
  })
  let positions: number[] = [...mesh.positions]
  let indices: number[] = [...mesh.indices]
  for (const [a, b] of selected) {
    const midpoint = positions.length / 3
    positions.push(
      (positions[a * 3] + positions[b * 3]) / 2,
      (positions[a * 3 + 1] + positions[b * 3 + 1]) / 2,
      (positions[a * 3 + 2] + positions[b * 3 + 2]) / 2,
    )
    const next: number[] = []
    let adjacent = 0
    for (let face = 0; face < indices.length / 3; face++) {
      const tri = indices.slice(face * 3, face * 3 + 3)
      const ai = tri.indexOf(a), bi = tri.indexOf(b)
      if (ai < 0 || bi < 0) {
        next.push(...tri)
        continue
      }
      adjacent++
      const c = tri.find(vertex => vertex !== a && vertex !== b)!
      // Preserve winding by replacing the directed edge in the triangle cycle.
      if ((ai + 1) % 3 === bi) next.push(a, midpoint, c, midpoint, b, c)
      else next.push(b, midpoint, c, midpoint, a, c)
    }
    if (!adjacent) throw new Error('Selected edge has no adjacent faces.')
    indices = next
  }
  if (positions.length > 300_000 || indices.length > 300_000) throw new Error('Knife cut exceeds mesh budget.')
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(indices) }
}

function faceNormal(mesh: PolygonMesh, face: number): [number, number, number] {
  const a = mesh.indices[face * 3], b = mesh.indices[face * 3 + 1], c = mesh.indices[face * 3 + 2]
  const ax = mesh.positions[a * 3], ay = mesh.positions[a * 3 + 1], az = mesh.positions[a * 3 + 2]
  const bx = mesh.positions[b * 3] - ax, by = mesh.positions[b * 3 + 1] - ay, bz = mesh.positions[b * 3 + 2] - az
  const cx = mesh.positions[c * 3] - ax, cy = mesh.positions[c * 3 + 1] - ay, cz = mesh.positions[c * 3 + 2] - az
  const nx = by * cz - bz * cy, ny = bz * cx - bx * cz, nz = bx * cy - by * cx
  const len = Math.hypot(nx, ny, nz) || 1
  return [nx / len, ny / len, nz / len]
}

/** Separate selected faces into a new object; remainder stays. */
export function separateFaces(mesh: PolygonMesh, faceIds: number[]): { kept: PolygonMesh; separated: PolygonMesh } {
  if (!faceIds.length) throw new Error('Select faces to separate.')
  const remove = new Set(faceIds)
  const keptIndices: number[] = []
  const sepIndices: number[] = []
  for (let f = 0; f < mesh.indices.length / 3; f++) {
    const tri = [mesh.indices[f * 3], mesh.indices[f * 3 + 1], mesh.indices[f * 3 + 2]]
    ;(remove.has(f) ? sepIndices : keptIndices).push(...tri)
  }
  if (!keptIndices.length || !sepIndices.length) throw new Error('Separation must leave two non-empty meshes.')
  return {
    kept: compactMesh({ positions: mesh.positions.slice(), indices: Uint32Array.from(keptIndices) }),
    separated: compactMesh({ positions: mesh.positions.slice(), indices: Uint32Array.from(sepIndices) }),
  }
}

export function joinMeshes(meshes: PolygonMesh[]): PolygonMesh {
  if (meshes.length < 2) throw new Error('Join needs at least two meshes.')
  const positions: number[] = []
  const indices: number[] = []
  for (const mesh of meshes) {
    const base = positions.length / 3
    positions.push(...mesh.positions)
    for (const i of mesh.indices) indices.push(i + base)
  }
  if (positions.length > 300_000 || indices.length > 300_000) throw new Error('Joined mesh exceeds budget.')
  return mergeByDistance({ positions: Float64Array.from(positions), indices: Uint32Array.from(indices) }, 1e-5)
}

export function symmetrizeMesh(mesh: PolygonMesh, axis: 0 | 1 | 2 = 0): PolygonMesh {
  const mirrored = mesh.positions.slice()
  for (let i = 0; i < mirrored.length / 3; i++) mirrored[i * 3 + axis] *= -1
  const flipped = flipFaces({ positions: mirrored, indices: mesh.indices.slice() })
  return joinMeshes([mesh, flipped])
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
  const count = mesh.positions.length / 3
  const picked = vertices.length ? vertices : Array.from({ length: count }, (_, i) => i)
  const center: [number, number, number] = [0, 0, 0]
  for (const v of picked) {
    center[0] += mesh.positions[v * 3]
    center[1] += mesh.positions[v * 3 + 1]
    center[2] += mesh.positions[v * 3 + 2]
  }
  return center.map(c => c / picked.length) as [number, number, number]
}

export function twistMesh(mesh: PolygonMesh, radiansPerUnit: number, origin: [number, number, number] = [0, 0, 0]): PolygonMesh {
  if (!finite(radiansPerUnit) || !origin.every(finite)) throw new Error('Invalid twist.')
  return deformPolygonMesh(mesh, { kind: 'twist', origin, radians_per_unit: radiansPerUnit })
}

function compactMesh(mesh: PolygonMesh): PolygonMesh {
  const used = new Set(mesh.indices)
  const remap = new Map<number, number>()
  const positions: number[] = []
  for (const id of [...used].sort((a, b) => a - b)) {
    remap.set(id, positions.length / 3)
    positions.push(mesh.positions[id * 3], mesh.positions[id * 3 + 1], mesh.positions[id * 3 + 2])
  }
  return { positions: Float64Array.from(positions), indices: Uint32Array.from(mesh.indices.map(i => remap.get(i)!)) }
}
