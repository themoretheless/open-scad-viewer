import type {MeshData} from '../core/mesh'
import type {PhotoReconstruction, PhotoSurface} from './photogrammetryKernel'
import {buildMeshBvh} from './meshBvh'
import {extractSemanticEdges} from './meshTopology'

type RecoveredCamera = PhotoReconstruction['cameras'][number]
type Vector3 = [number, number, number]
export type PhotoPreviewMode = 'points' | 'surface'
const MAX_PREVIEW_POINTS = 6000
const MARKER_FACES = [0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3]
const IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]

interface PreviewCoordinates {
  positions: Vector3[]
  center: Vector3
  scale: number
  depth: number
}

/** All view adapters use the same transform; export coordinates never pass through it. */
function previewCoordinates(surface: PhotoSurface, camera?: RecoveredCamera): PreviewCoordinates {
  const low: Vector3 = [Infinity, Infinity, Infinity]
  const high: Vector3 = [-Infinity, -Infinity, -Infinity]
  const depths: number[] = []
  const positions = surface.positions.map(position => {
    let point: Vector3
    if (camera) {
      const q = camera.rotation.map((row, i) => row.reduce((sum, value, j) => sum + value * position[j]!, camera.translation[i]!))
      point = [q[0]!, q[2]!, -q[1]!]
      if (q[2]! > 0) depths.push(q[2]!)
    } else point = [position[0]!, position[1]!, position[2]!]
    for (let axis = 0; axis < 3; axis++) {
      low[axis] = Math.min(low[axis]!, point[axis]!)
      high[axis] = Math.max(high[axis]!, point[axis]!)
    }
    return point
  })
  const center: Vector3 = positions.length ? low.map((value, axis) => (value + high[axis]!) / 2) as Vector3 : [0, 0, 0]
  const extent = positions.length ? Math.max(...high.map((value, axis) => value - low[axis]!)) : 0
  const scale = extent > 0 ? 100 / extent : 1
  for (const point of positions) for (let axis = 0; axis < 3; axis++) point[axis] = (point[axis]! - center[axis]!) * scale
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
    const length = Math.hypot(vertices[start + 3]!, vertices[start + 4]!, vertices[start + 5]!)
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

function markerMesh(points: readonly Vector3[], color: MeshData['color']): MeshData {
  const vertices = new Float32Array(points.length * 24)
  const indices = new Uint32Array(points.length * 12)
  // Normalized preview spans 100 units. A small lower bound also handles a single point.
  const size = 0.4
  points.forEach(([x, y, z], i) => {
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
    if (!this.surface.triangles.length) {
      const step = Math.max(1, Math.ceil(positions.length / MAX_PREVIEW_POINTS))
      return markerMesh(positions.filter((_, index) => index % step === 0), [0.7, 0.75, 0.8, 1])
    }
    const vertices = new Float32Array(positions.length * 6)
    positions.forEach((point, i) => vertices.set(point, i * 6))
    const indices = new Uint32Array(this.surface.triangles.length * 3)
    this.surface.triangles.forEach((face, i) => indices.set(face, i * 3))
    return renderMesh(vertices, indices, [0.7, 0.75, 0.8, 1], true)
  }

  /** Builds only the displayed batches, avoiding a discarded full-cloud BVH/topology pass. */
  private cloudMeshes(): MeshData[] {
    const {positions} = this.coordinates
    const step = Math.max(1, Math.ceil(positions.length / MAX_PREVIEW_POINTS))
    const groups = new Map<number, {points: Vector3[]; color: Vector3}>()
    for (let source = 0; source < positions.length; source += step) {
      const color = this.surface.colors[source] ?? [180, 180, 180]
      const key = Math.floor(color[0]! / 64) * 16 + Math.floor(color[1]! / 64) * 4 + Math.floor(color[2]! / 64)
      let group = groups.get(key)
      if (!group) { group = {points: [], color: [0, 0, 0]}; groups.set(key, group) }
      group.points.push(positions[source]!)
      for (let axis = 0; axis < 3; axis++) group.color[axis] += color[axis]!
    }
    return [...groups.values()].map(group => markerMesh(group.points, [
      group.color[0] / group.points.length / 255,
      group.color[1] / group.points.length / 255,
      group.color[2] / group.points.length / 255, 1,
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
