import {meshObjectStats,buildEdgeList} from './meshEditing'
import {callGeometryRust} from './geometry/kernel'
import {projectDirectPoint, type OrbitCamera } from './directModelingTools'
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

export function projectMeshModelerObject(object:{visible?:boolean;mesh:PolygonMesh},camera:OrbitCamera,selectMode:string,getEdgeList:(mesh:PolygonMesh)=>Array<[number,number]>) {
  if (!object.visible) return { tris: [] as Array<{ points: string; face: number; depth: number }>, verts: [] as Array<{ x: number; y: number; id: number }>, edges: [] as Array<{ x1: number; y1: number; x2: number; y2: number; id: number }> }
  const cam = camera
  const tris = projectMeshModelerFaces(object.mesh, cam)
  const verts: Array<{ x: number; y: number; id: number }> = []
  const pos = object.mesh.positions
  if (selectMode === 'vertex') {
    for (let i = 0, o = 0; o < pos.length; i++, o += 3) {
      const p = projectDirectPoint([pos[o], pos[o + 1], pos[o + 2]], cam)
      verts.push({ x: p[0], y: -p[1], id: i })
    }
  }
  const edges = selectMode === 'edge' ? (() => {
    const edgeList = getEdgeList(object.mesh)
    const vertexCount = pos.length / 3
    const projX = new Float64Array(vertexCount)
    const projY = new Float64Array(vertexCount)
    for (let i = 0, o = 0; i < vertexCount; i++, o += 3) {
      const p = projectDirectPoint([pos[o], pos[o + 1], pos[o + 2]], cam)
      projX[i] = p[0]
      projY[i] = -p[1]
    }
    return edgeList.map(([a, b], id) => ({ id, x1: projX[a], y1: projY[a], x2: projX[b], y2: projY[b] }))
  })() : []
  return { tris, verts, edges }
}

const statistics=new WeakMap<PolygonMesh,ReturnType<typeof meshObjectStats>&{edges:number}>()
export function cachedMeshModelerStats(mesh:PolygonMesh) {
 const cached=statistics.get(mesh);if(cached)return cached
 const result={...meshObjectStats(mesh),edges:buildEdgeList(mesh).length};statistics.set(mesh,result);return result
}
