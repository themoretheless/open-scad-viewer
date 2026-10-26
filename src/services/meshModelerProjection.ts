import {meshObjectStats,buildEdgeList} from './meshEditing'
import {callGeometryRust} from './geometry/kernel'
import {projectDirectPoint, type OrbitCamera } from './directModelingTools'
import { inspectPolygonMesh, type PolygonMesh } from './geometry/polygon'
import type { KernelManifoldReport } from './geometry/manifoldAnalysis'

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

export interface ProjectedMeshLine { x1: number; y1: number; x2: number; y2: number; id: number }
export interface ProjectedMeshVertex { x: number; y: number; id: number }
export interface MeshManifoldSceneOverlay {
  boundaryEdges: ProjectedMeshLine[]
  nonManifoldEdges: ProjectedMeshLine[]
  nonManifoldVertices: ProjectedMeshVertex[]
}

const emptyManifoldOverlay = (): MeshManifoldSceneOverlay => ({boundaryEdges: [], nonManifoldEdges: [], nonManifoldVertices: []})

export function projectMeshModelerObject(
  object:{visible?:boolean;mesh:PolygonMesh},
  camera:OrbitCamera,
  selectMode:string,
  getEdgeList:(mesh:PolygonMesh)=>Array<[number,number]>,
  manifoldReport?:KernelManifoldReport,
) {
  if (!object.visible) return {
    tris: [] as Array<{ points: string; face: number; depth: number }>,
    verts: [] as ProjectedMeshVertex[], edges: [] as ProjectedMeshLine[], defects: emptyManifoldOverlay(),
  }
  const tris = projectMeshModelerFaces(object.mesh, camera)
  const pos = object.mesh.positions
  let projected: ProjectedMeshVertex[] | undefined
  const vertices = () => projected ??= Array.from({length: pos.length / 3}, (_, id) => {
    const p = projectDirectPoint([pos[id * 3], pos[id * 3 + 1], pos[id * 3 + 2]], camera)
    return {x: p[0], y: -p[1], id}
  })
  const verts = selectMode === 'vertex' ? vertices() : []
  const edges = selectMode === 'edge' ? getEdgeList(object.mesh).map(([a, b], id) => {
    const points = vertices()
    return {id, x1: points[a].x, y1: points[a].y, x2: points[b].x, y2: points[b].y}
  }) : []
  const defects = emptyManifoldOverlay()
  if (manifoldReport && !manifoldReport.isManifold) {
    const points = vertices()
    const lines = (indices: Uint32Array) => Array.from({length: indices.length / 2}, (_, id): ProjectedMeshLine => {
      const a = points[indices[id * 2]], b = points[indices[id * 2 + 1]]
      return {id, x1: a.x, y1: a.y, x2: b.x, y2: b.y}
    })
    defects.boundaryEdges = lines(manifoldReport.boundaryEdges)
    defects.nonManifoldEdges = lines(manifoldReport.nonManifoldEdges)
    defects.nonManifoldVertices = Array.from(manifoldReport.nonManifoldVertices, id => points[id])
  }
  return { tris, verts, edges, defects }
}

const statistics=new WeakMap<PolygonMesh,ReturnType<typeof meshObjectStats>&{edges:number}>()
export function cachedMeshModelerStats(mesh:PolygonMesh) {
 const cached=statistics.get(mesh);if(cached)return cached
 const result={...meshObjectStats(mesh),edges:buildEdgeList(mesh).length};statistics.set(mesh,result);return result
}
