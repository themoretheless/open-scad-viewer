import type {MeshData} from '../../core/mesh'
import type {PhotoReconstruction, PhotoSurface} from './kernel'
import {buildMeshBvh} from '../meshBvh'
import {extractSemanticEdges} from '../meshTopology'

type RecoveredCamera = PhotoReconstruction['cameras'][number]
type Vector3 = [number, number, number]
export type PhotoPreviewMode = 'points' | 'surface'
const MAX_PREVIEW_POINTS = 6000
const MARKER_FACES = [0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3]
const IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]

interface PreviewCoordinates {
  positions: Float32Array
  center: Vector3
  scale: number
  depth: number
}

/** All view adapters use the same transform; export coordinates never pass through it. */
function previewCoordinates(surface: PhotoSurface, camera?: RecoveredCamera): PreviewCoordinates {
  const count = surface.positions.length / 3
  const low: Vector3 = [Infinity, Infinity, Infinity]
  const high: Vector3 = [-Infinity, -Infinity, -Infinity]
  const depths: number[] = []
  const transformed = new Float64Array(surface.positions.length)
  for (let i = 0; i < count; i++) {
    const x = surface.positions[i * 3]!, y = surface.positions[i * 3 + 1]!, z = surface.positions[i * 3 + 2]!
    let px = x, py = y, pz = z
    if (camera) {
      const r = camera.rotation, t = camera.translation
      const q0 = t[0]! + r[0]![0]! * x + r[0]![1]! * y + r[0]![2]! * z
      const q1 = t[1]! + r[1]![0]! * x + r[1]![1]! * y + r[1]![2]! * z
      const q2 = t[2]! + r[2]![0]! * x + r[2]![1]! * y + r[2]![2]! * z
      px = q0; py = q2; pz = -q1
      if (q2 > 0) depths.push(q2)
    }
    low[0] = Math.min(low[0], px); low[1] = Math.min(low[1], py); low[2] = Math.min(low[2], pz)
    high[0] = Math.max(high[0], px); high[1] = Math.max(high[1], py); high[2] = Math.max(high[2], pz)
    transformed[i * 3] = px
    transformed[i * 3 + 1] = py
    transformed[i * 3 + 2] = pz
  }
  const center: Vector3 = count ? low.map((value, axis) => (value + high[axis]!) / 2) as Vector3 : [0, 0, 0]
  const extent = count ? Math.max(...high.map((value, axis) => value - low[axis]!)) : 0
  const scale = extent > 0 ? 100 / extent : 1
  const positions = new Float32Array(transformed.length)
  for (let i = 0; i < transformed.length; i++) positions[i] = (transformed[i]! - center[i % 3]!) * scale
  depths.sort((a, b) => a - b)
  return {positions, center, scale, depth: depths[Math.floor(depths.length / 2)] ?? 1}
}

function fillNormals(vertices: Float32Array, indices: Uint32Array): void {
  for (let face = 0; face < indices.length; face += 3) {
    const a = indices[face]! * 6, b = indices[face + 1]! * 6, c = indices[face + 2]! * 6
    const ux = vertices[b]! - vertices[a]!, uy = vertices[b + 1]! - vertices[a + 1]!, uz = vertices[b + 2]! - vertices[a + 2]!
    const vx = vertices[c]! - vertices[a]!, vy = vertices[c + 1]! - vertices[a + 1]!, vz = vertices[c + 2]! - vertices[a + 2]!
    const normal = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx]
    for (const start of [a, b, c]) for (let axis = 0; axis < 3; axis++) vertices[start + 3 + axis] += normal[axis]!
  }
  for (let start = 0; start < vertices.length; start += 6) {
    const nx = vertices[start + 3]!, ny = vertices[start + 4]!, nz = vertices[start + 5]!
    const length = Math.sqrt(nx * nx + ny * ny + nz * nz)
    if (length > 0) for (let axis = 0; axis < 3; axis++) vertices[start + 3 + axis] /= length
  }
}

function renderMesh(vertices: Float32Array, indices: Uint32Array, color: MeshData['color'], surface: boolean): MeshData {
  fillNormals(vertices, indices)
  const edges = surface ? extractSemanticEdges(vertices, indices) : {
    indices: new Uint32Array(), diagnostics: {boundary: 0, crease: 0, nonManifold: 0, degenerate: 0},
  }
  return {
    vertices, indices, color, bvh: buildMeshBvh(vertices, indices),
    edgeIndices: edges.indices, topology: edges.diagnostics,
    faceIds: Uint32Array.from({length: indices.length / 3}, (_, i) => i),
    provenance: [], transform: new Float32Array(IDENTITY),
  }
}

function markerMesh(points: Float32Array, markers: readonly number[], color: MeshData['color']): MeshData {
  const vertices = new Float32Array(markers.length * 24)
  const indices = new Uint32Array(markers.length * 12)
  // Normalized preview spans 100 units. A small lower bound also handles a single point.
  const size = 0.4
  markers.forEach((point, i) => {
    const x = points[point * 3]!, y = points[point * 3 + 1]!, z = points[point * 3 + 2]!
    const start = i * 24
    vertices.set([x + size, y, z], start)
    vertices.set([x - size, y, z], start + 6)
    vertices.set([x, y + size, z], start + 12)
    vertices.set([x, y, z + size], start + 18)
    for (let face = 0; face < MARKER_FACES.length; face++) indices[i * 12 + face] = i * 4 + MARKER_FACES[face]!
  })
  return renderMesh(vertices, indices, color, false)
}

/** Result-scoped cache: mode switches reuse transformed points and already built BVHs. */
export class PhotoPreview {
  private readonly coordinates: PreviewCoordinates
  private readonly cachedMeshes = new Map<PhotoPreviewMode, MeshData[]>()

  constructor(private readonly surface: PhotoSurface, private readonly sourceCamera?: RecoveredCamera) {
    this.coordinates = previewCoordinates(surface, sourceCamera)
  }

  meshes(mode: PhotoPreviewMode): MeshData[] {
    const cached = this.cachedMeshes.get(mode)
    if (cached) return cached
    const meshes = mode === 'points' ? this.cloudMeshes() : [this.surfaceMesh()]
    this.cachedMeshes.set(mode, meshes)
    return meshes
  }

  frame(aspect: number) {
    if (!this.sourceCamera) return null
    const {center, scale, depth} = this.coordinates
    const camera = this.sourceCamera
    const safeAspect = Number.isFinite(aspect) && aspect > 0 ? aspect : 1
    return {
      fovY: 2 * Math.atan(Math.max(camera.cy, camera.cx / Math.max(safeAspect, 0.1)) / camera.focal) * 1.05,
      camera: {yaw: 0, pitch: 0, distance: depth * scale,
        target: [-center[0] * scale, (depth - center[1]) * scale, -center[2] * scale] as Vector3,
        projection: 'perspective' as const},
    }
  }

  private surfaceMesh(): MeshData {
    const {positions} = this.coordinates
    const count = positions.length / 3
    if (!this.surface.triangles.length) {
      const step = Math.max(1, Math.ceil(count / MAX_PREVIEW_POINTS))
      const markers: number[] = []
      for (let i = 0; i < count; i += step) markers.push(i)
      return markerMesh(positions, markers, [0.7, 0.75, 0.8, 1])
    }
    const vertices = new Float32Array(count * 6)
    for (let i = 0; i < count; i++) {
      vertices[i * 6] = positions[i * 3]!
      vertices[i * 6 + 1] = positions[i * 3 + 1]!
      vertices[i * 6 + 2] = positions[i * 3 + 2]!
    }
    const indices = new Uint32Array(this.surface.triangles.length)
    indices.set(this.surface.triangles)
    return renderMesh(vertices, indices, [0.7, 0.75, 0.8, 1], true)
  }

  /** Builds only the displayed batches, avoiding a discarded full-cloud BVH/topology pass. */
  private cloudMeshes(): MeshData[] {
    const {positions} = this.coordinates
    const colors = this.surface.colors
    const count = positions.length / 3
    const step = Math.max(1, Math.ceil(count / MAX_PREVIEW_POINTS))
    const groups = new Map<number, {markers: number[]; color: Vector3}>()
    for (let source = 0; source < count; source += step) {
      const colored = source * 3 + 2 < colors.length
      const red = colored ? colors[source * 3]! : 180
      const green = colored ? colors[source * 3 + 1]! : 180
      const blue = colored ? colors[source * 3 + 2]! : 180
      const key = Math.floor(red / 64) * 16 + Math.floor(green / 64) * 4 + Math.floor(blue / 64)
      let group = groups.get(key)
      if (!group) { group = {markers: [], color: [0, 0, 0]}; groups.set(key, group) }
      group.markers.push(source)
      group.color[0]! += red; group.color[1]! += green; group.color[2]! += blue
    }
    return [...groups.values()].map(group => markerMesh(positions, group.markers, [
      group.color[0]! / group.markers.length / 255,
      group.color[1]! / group.markers.length / 255,
      group.color[2]! / group.markers.length / 255, 1,
    ]))
  }
}

export function photoMesh(surface: PhotoSurface, camera?: RecoveredCamera): MeshData {
  return new PhotoPreview(surface, camera).meshes('surface')[0]!
}
export function photoCloudMeshes(surface: PhotoSurface, camera?: RecoveredCamera): MeshData[] {
  return new PhotoPreview(surface, camera).meshes('points')
}
export function photoCameraFrame(surface: PhotoSurface, camera: RecoveredCamera, aspect: number) {
  return new PhotoPreview(surface, camera).frame(aspect)!
}
