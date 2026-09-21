import { cross3, xyPlane, transformSketch, type SketchPlane } from './directSketchGeometry'
import { booleanPolygonMeshes, type PolygonMesh } from './geometry/polygon'
import { extrudeDirectSketch, parseDirectDocument, type DirectBody, type DirectDocument, type DirectSketch, type Point2 } from './directModeling'

export interface OrbitCamera { yaw: number; pitch: number }
export const defaultDirectCamera = (): OrbitCamera => ({ yaw: Math.PI / 4, pitch: Math.atan(1 / Math.sqrt(2)) })
export function projectDirectPoint(p: number[], camera: OrbitCamera): [number, number, number] {
  const cy = Math.cos(camera.yaw), sy = Math.sin(camera.yaw), cp = Math.cos(camera.pitch), sp = Math.sin(camera.pitch)
  const horizontal = p[0] * sy + p[1] * cy, z = p[2] ?? 0
  return [p[0] * cy - p[1] * sy, horizontal * sp - z * cp, horizontal * cp + z * sp]
}
export function unprojectDirectXY(p: Point2, camera: OrbitCamera): Point2 {
  const sp = Math.sin(camera.pitch)
  if (Math.abs(sp) < .04) throw new Error('Rotate the view away from the horizon to move in XY.')
  const horizontal = p[1] / sp, cy = Math.cos(camera.yaw), sy = Math.sin(camera.yaw)
  return [p[0] * cy + horizontal * sy, -p[0] * sy + horizontal * cy]
}
/** Intersect an orthographic viewport ray with an arbitrary sketch plane. */
export function unprojectDirectPlane(p: Point2, plane: SketchPlane, camera: OrbitCamera): Point2 {
  const o=projectDirectPoint(plane.origin,camera),u=projectDirectPoint(plane.u,camera),v=projectDirectPoint(plane.v,camera)
  const determinant=u[0]*v[1]-u[1]*v[0]
  if(Math.abs(determinant)<1e-6)throw Error('Rotate the view away from the sketch plane edge.')
  const x=p[0]-o[0],y=p[1]-o[1]
  return [(x*v[1]-y*v[0])/determinant,(u[0]*y-u[1]*x)/determinant]
}
export function snapDirectPoint(p: Point2, candidates: Point2[], tolerance: number, grid: number): { point: Point2; kind: 'vertex' | 'grid' | null } {
  let nearest: Point2 | undefined, distance = tolerance
  for (const candidate of candidates) { const d = Math.hypot(p[0] - candidate[0], p[1] - candidate[1]); if (d < distance) { distance = d; nearest = candidate } }
  if (nearest) return { point: [...nearest], kind: 'vertex' }
  if (Number.isFinite(grid) && grid > 0) return { point: [Math.round(p[0] / grid) * grid, Math.round(p[1] / grid) * grid], kind: 'grid' }
  return { point: [...p], kind: null }
}
export function directExtrusionTool(sketch: DirectSketch, height: number, baseZ: number): DirectBody {
  if (!Number.isFinite(baseZ) || !Number.isFinite(height) || Math.abs(height) < .01 || Math.abs(baseZ) > 1e6) throw new Error('Extrusion requires a finite height of at least 0.01 mm.')
  const body = extrudeDirectSketch(sketch, Math.abs(height), 'preview-extrusion')
  const plane=sketch.plane??xyPlane(), normal=cross3(plane.u,plane.v)
  for (let i=0;i<body.mesh.positions.length;i++) body.mesh.positions[i]+=normal[i%3]*(baseZ+Math.min(0,height))
  return body
}
export function applyDirectExtrusion(document: DirectDocument, sketchId: string, height: number, baseZ: number, operation: 'new' | 'union' | 'difference', targetId: string, id: string): DirectDocument {
  const next = parseDirectDocument(JSON.stringify(document)), sketch = next.sketches.find(s => s.id === sketchId)
  if (!sketch) throw new Error('Select a sketch.')
  const tool = directExtrusionTool(sketch, height, baseZ)
  if (operation === 'new') { tool.id = id; next.bodies.push(tool) }
  else {
    const target = next.bodies.find(b => b.id === targetId)
    if (!target) throw new Error('Select the target body.')
    const mesh = booleanPolygonMeshes(target.mesh, tool.mesh, operation)
    if (!mesh.indices.length) next.bodies = next.bodies.filter(b => b.id !== target.id)
    else target.mesh = { positions: mesh.positions, indices: mesh.indices }
  }
  return parseDirectDocument(JSON.stringify(next))
}
export function circularDirectCopies(sketch: DirectSketch, count: number, center: Point2, sweep: number, makeId: () => string): DirectSketch[] {
  if (!Number.isInteger(count) || count < 2 || count > 64 || !center.every(Number.isFinite) || !Number.isFinite(sweep) || Math.abs(sweep) < .01 || Math.abs(sweep) > 360) throw new Error('Use 2–64 instances and an angle up to 360 degrees.')
  const closed = Math.abs(sweep) === 360
  return Array.from({ length: count - 1 }, (_, i) => {
    const a = (i + 1) * sweep / (closed ? count : count - 1) * Math.PI / 180, c = Math.cos(a), s = Math.sin(a)
    return { ...transformSketch(sketch,[0,0],a*180/Math.PI,1,center), id: makeId(), name: (sketch.name + ' · ' + (i + 2)).slice(0,100) }
  })
}
export function directFaceShade(mesh: PolygonMesh, triangle: number, camera: OrbitCamera): number {
  const points = mesh.indices.slice(triangle * 3, triangle * 3 + 3).map(i => mesh.positions.slice(i * 3, i * 3 + 3))
  const a = points[1].map((v, i) => v - points[0][i]), b = points[2].map((v, i) => v - points[0][i])
  const n = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]], length = Math.hypot(...n) || 1
  const light = projectDirectPoint(n.map(v=>v/length), camera)
  return Math.max(24, Math.min(78, 48 + 20 * light[2] - 15 * light[1] + 8 * light[0]))
}
