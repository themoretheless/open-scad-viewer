import {callGeometryRust} from './geometry/kernel'
import { type OrbitCamera } from './directModelingTools'
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
  const faces = callGeometryRust<Array<[[number[], number[], number[]], number, number]>>('mesh_editor', {
    action: 'project_faces', mesh, yaw: camera.yaw, pitch: camera.pitch, orientation: sign,
  })
  return faces.map(([points, face, depth]) => ({points: points.map(p => `${p[0]},${-p[1]}`).join(' '), face, depth}))
}
