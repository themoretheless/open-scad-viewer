import { projectDirectPoint, type OrbitCamera } from './directModelingTools'
import { inspectPolygonMesh, type PolygonMesh } from './geometry/polygon'

// Editing publishes a new mesh. Reuse its topology report while only the camera moves.
const orientation = new WeakMap<PolygonMesh, number>()
function closedOrientation(mesh: PolygonMesh): number {
  const cached = orientation.get(mesh)
  if (cached !== undefined) return cached
  const report = inspectPolygonMesh(mesh)
  const sign = report.closed && !report.orientationConflicts && !report.degenerateTriangles
    ? Math.sign(report.signedVolumeMm3) : 0
  orientation.set(mesh, sign)
  return sign
}

export function projectMeshModelerFaces(mesh: PolygonMesh, camera: OrbitCamera) {
  const sign = closedOrientation(mesh)
  const points = Array.from({ length: mesh.positions.length / 3 }, (_, i) =>
    projectDirectPoint([mesh.positions[i * 3], mesh.positions[i * 3 + 1], mesh.positions[i * 3 + 2]], camera))
  const faces: Array<{ points: string; face: number; depth: number }> = []
  for (let face = 0; face < mesh.indices.length / 3; face++) {
    const [a, b, c] = [0, 1, 2].map(k => points[mesh.indices[face * 3 + k]])
    const facing = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    // Centroid sorting alone lets a rear triangle cover part of a front face.
    // Only consistently oriented closed shells can safely discard their back sides.
    // Open or inconsistently wound meshes must remain editable from both sides.
    if (sign && facing * sign <= 0) continue
    faces.push({ points: [a, b, c].map(p => `${p[0]},${-p[1]}`).join(' '), face,
      depth: (a[2] + b[2] + c[2]) / 3 })
  }
  return faces
}
