import type { MeshData } from '../core/mesh'
import { inspectMeshManifoldDiagnostics } from '../kernel/meshInspection'

const VERTEX_STRIDE = 6

export type Vec3 = [number, number, number]

/** Measured properties of one evaluated body, in world coordinates of its display mesh. */
export interface BodyProperties {
  /** Sum of triangle areas. */
  surfaceArea: number
  /** Signed divergence-theorem volume; meaningful only when `closed`. */
  volume: number
  /** Volume centroid for closed bodies, area centroid otherwise. */
  centroid: Vec3 | null
  bounds: { min: Vec3; max: Vec3 } | null
  triangleCount: number
  /** Distinct positions after welding at 1e-6. */
  vertexCount: number
  edgeCount: number
  /** Authored or coplanar face groups (`faceIds`). */
  faceCount: number
  boundaryEdgeCount: number
  nonManifoldEdgeCount: number
  closed: boolean
  eulerCharacteristic: number
}

/** Area, volume, centroid, bounds and topology counts of a display mesh after its row-major transform. */
export function measureBodyProperties(
  mesh: Pick<MeshData, 'vertices' | 'indices' | 'transform'> & { faceIds?: Uint32Array },
): BodyProperties {
  const { vertices, indices } = mesh
  const m =
    mesh.transform.length === 16 ? mesh.transform : new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
  const vertexTotal = Math.floor(vertices.length / VERTEX_STRIDE)
  const world = new Float64Array(vertexTotal * 3)
  const min: Vec3 = [Infinity, Infinity, Infinity]
  const max: Vec3 = [-Infinity, -Infinity, -Infinity]
  const used = new Uint8Array(vertexTotal)
  for (let i = 0; i < indices.length; i++) {
    const index = indices[i]!
    if (index < vertexTotal) used[index] = 1
  }
  for (let v = 0; v < vertexTotal; v++) {
    const x = vertices[v * VERTEX_STRIDE]!,
      y = vertices[v * VERTEX_STRIDE + 1]!,
      z = vertices[v * VERTEX_STRIDE + 2]!
    const wx = m[0]! * x + m[1]! * y + m[2]! * z + m[3]!
    const wy = m[4]! * x + m[5]! * y + m[6]! * z + m[7]!
    const wz = m[8]! * x + m[9]! * y + m[10]! * z + m[11]!
    world[v * 3] = wx
    world[v * 3 + 1] = wy
    world[v * 3 + 2] = wz
    if (!used[v]) continue
    if (wx < min[0]) min[0] = wx
    if (wy < min[1]) min[1] = wy
    if (wz < min[2]) min[2] = wz
    if (wx > max[0]) max[0] = wx
    if (wy > max[1]) max[1] = wy
    if (wz > max[2]) max[2] = wz
  }

  let area = 0,
    sixVolume = 0
  const areaMoment: Vec3 = [0, 0, 0]
  const volumeMoment: Vec3 = [0, 0, 0]
  const triangleCount = Math.floor(indices.length / 3)
  for (let t = 0; t < triangleCount; t++) {
    const a = indices[t * 3]!,
      b = indices[t * 3 + 1]!,
      c = indices[t * 3 + 2]!
    if (a >= vertexTotal || b >= vertexTotal || c >= vertexTotal) continue
    const ax = world[a * 3]!,
      ay = world[a * 3 + 1]!,
      az = world[a * 3 + 2]!
    const bx = world[b * 3]!,
      by = world[b * 3 + 1]!,
      bz = world[b * 3 + 2]!
    const cx = world[c * 3]!,
      cy = world[c * 3 + 1]!,
      cz = world[c * 3 + 2]!
    const ux = bx - ax,
      uy = by - ay,
      uz = bz - az
    const vx = cx - ax,
      vy = cy - ay,
      vz = cz - az
    const triangleArea = Math.hypot(uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx) / 2
    area += triangleArea
    areaMoment[0] += (triangleArea * (ax + bx + cx)) / 3
    areaMoment[1] += (triangleArea * (ay + by + cy)) / 3
    areaMoment[2] += (triangleArea * (az + bz + cz)) / 3
    // Signed tetrahedron (origin, a, b, c) volume times six.
    const tetra = ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx)
    sixVolume += tetra
    volumeMoment[0] += (tetra * (ax + bx + cx)) / 4
    volumeMoment[1] += (tetra * (ay + by + cy)) / 4
    volumeMoment[2] += (tetra * (az + bz + cz)) / 4
  }

  const topology = inspectMeshManifoldDiagnostics(mesh)
  const closed = topology.isWatertight
  const volume = sixVolume / 6
  let centroid: Vec3 | null = null
  if (closed && Math.abs(sixVolume) > 1e-12)
    centroid = [volumeMoment[0] / sixVolume, volumeMoment[1] / sixVolume, volumeMoment[2] / sixVolume]
  else if (area > 0) centroid = [areaMoment[0] / area, areaMoment[1] / area, areaMoment[2] / area]

  let faceCount = topology.validFaceCount
  if (mesh.faceIds && mesh.faceIds.length === triangleCount) faceCount = new Set(mesh.faceIds).size

  return {
    surfaceArea: area,
    volume,
    centroid,
    bounds: Number.isFinite(min[0]) ? { min, max } : null,
    triangleCount,
    vertexCount: topology.uniqueVertexCount,
    edgeCount: topology.uniqueEdgeCount,
    faceCount,
    boundaryEdgeCount: topology.boundaryEdgeCount,
    nonManifoldEdgeCount: topology.nonManifoldEdgeCount,
    closed,
    eulerCharacteristic: topology.eulerCharacteristic,
  }
}
