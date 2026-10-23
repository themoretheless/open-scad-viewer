import { cross3, xyPlane, transformSketch, type SketchPlane } from './directSketchGeometry'
import { booleanPolygonMeshes, type PolygonMesh } from './geometry/polygon'
import { extrudeDirectSketch, parseDirectDocument, type DirectBody, type DirectDocument, type DirectSketch, type Point2 } from './directModeling'
import { stringifyMeshJson } from './meshJson'

export interface OrbitCamera { yaw: number; pitch: number }
export const defaultDirectCamera = (): OrbitCamera => ({ yaw: Math.PI / 4, pitch: Math.atan(1 / Math.sqrt(2)) })
export function projectDirectPoint(p: number[], camera: OrbitCamera): [number, number, number] {
  const cy = Math.cos(camera.yaw), sy = Math.sin(camera.yaw), cp = Math.cos(camera.pitch), sp = Math.sin(camera.pitch)
  const horizontal = p[0] * sy + p[1] * cy, z = p[2] ?? 0
  return [p[0] * cy - p[1] * sy, horizontal * sp - z * cp, horizontal * cp + z * sp]
}
/** A scene projection shares one camera basis across all vertices and normals. */
export function createDirectProjector(camera:OrbitCamera):(p:number[])=>[number,number,number]{
 const cy=Math.cos(camera.yaw),sy=Math.sin(camera.yaw),cp=Math.cos(camera.pitch),sp=Math.sin(camera.pitch)
 return p=>{const horizontal=p[0]*sy+p[1]*cy,z=p[2]??0;return [p[0]*cy-p[1]*sy,horizontal*sp-z*cp,horizontal*cp+z*sp]}
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
  const plane = sketch.plane ?? xyPlane(), normal = cross3(plane.u, plane.v)
  const dz = baseZ + Math.min(0, height)
  const ox = normal[0] * dz, oy = normal[1] * dz, oz = normal[2] * dz
  const pos = body.mesh.positions
  for (let i = 0; i < pos.length; i += 3) {
    pos[i] += ox
    pos[i + 1] += oy
    pos[i + 2] += oz
  }
  return body
}
export function applyDirectExtrusion(document: DirectDocument, sketchId: string, height: number, baseZ: number, operation: 'new' | 'union' | 'difference', targetId: string, id: string): DirectDocument {
  const next = parseDirectDocument(stringifyMeshJson(document)), sketch = next.sketches.find(s => s.id === sketchId)
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
  return parseDirectDocument(stringifyMeshJson(next))
}
export function circularDirectCopies(sketch: DirectSketch, count: number, center: Point2, sweep: number, makeId: () => string): DirectSketch[] {
  if (!Number.isInteger(count) || count < 2 || count > 64 || !center.every(Number.isFinite) || !Number.isFinite(sweep) || Math.abs(sweep) < .01 || Math.abs(sweep) > 360) throw new Error('Use 2–64 instances and an angle up to 360 degrees.')
  const closed = Math.abs(sweep) === 360
  return Array.from({ length: count - 1 }, (_, i) => {
    const a = (i + 1) * sweep / (closed ? count : count - 1) * Math.PI / 180, c = Math.cos(a), s = Math.sin(a)
    return { ...transformSketch(sketch,[0,0],a*180/Math.PI,1,center), id: makeId(), name: (sketch.name + ' · ' + (i + 2)).slice(0,100) }
  })
}
const SCRATCH_NORMAL: [number, number, number] = [0, 0, 0]
export function directFaceShade(mesh: PolygonMesh, triangle: number, camera: OrbitCamera, project?: (p: number[]) => [number, number, number]): number {
  const base = triangle * 3
  const i0 = mesh.indices[base] * 3, i1 = mesh.indices[base + 1] * 3, i2 = mesh.indices[base + 2] * 3
  const pos = mesh.positions
  const p0x = pos[i0], p0y = pos[i0 + 1], p0z = pos[i0 + 2]
  const ax = pos[i1] - p0x, ay = pos[i1 + 1] - p0y, az = pos[i1 + 2] - p0z
  const bx = pos[i2] - p0x, by = pos[i2 + 1] - p0y, bz = pos[i2 + 2] - p0z
  const nx = ay * bz - az * by, ny = az * bx - ax * bz, nz = ax * by - ay * bx
  const length = Math.hypot(nx, ny, nz) || 1
  SCRATCH_NORMAL[0] = nx / length
  SCRATCH_NORMAL[1] = ny / length
  SCRATCH_NORMAL[2] = nz / length
  const light = project ? project(SCRATCH_NORMAL) : projectDirectPoint(SCRATCH_NORMAL, camera)
  return Math.max(24, Math.min(78, 48 + 20 * light[2] - 15 * light[1] + 8 * light[0]))
}

/**
 * Computes 3D translation offsets `[dx, dy, 0]` for a rectangular linear pattern of `countX × countY`
 * instances spaced by `(spacingX, spacingY)` in millimeters.
 */
export function computeLinearPatternOffsets(
  countX: number,
  countY: number,
  spacingX: number,
  spacingY: number,
): Array<[number, number, number]> {
  if (!Number.isInteger(countX) || countX < 1 || countX > 256 || !Number.isInteger(countY) || countY < 1 || countY > 256) {
    throw new RangeError('countX and countY must be integers between 1 and 256.')
  }
  if (!Number.isFinite(spacingX) || !Number.isFinite(spacingY)) {
    throw new RangeError('spacingX and spacingY must be finite numbers.')
  }
  const offsets: Array<[number, number, number]> = []
  for (let iy = 0; iy < countY; iy++) {
    for (let ix = 0; ix < countX; ix++) {
      const dx = ix * spacingX
      const dy = iy * spacingY
      offsets.push([dx === 0 ? 0 : dx, dy === 0 ? 0 : dy, 0])
    }
  }
  return offsets
}

/**
 * Computes circular pattern placements around the Z axis on a circle of `radiusMm`
 * over `fullAngleDeg` (default 360°). When `|fullAngleDeg| === 360`, instances are spaced
 * evenly by `fullAngleDeg / count`; for partial arcs (`|fullAngleDeg| < 360`), instances
 * span endpoints from `0` to `fullAngleDeg` (`fullAngleDeg / (count - 1)`).
 */
export function computeCircularPatternTransforms(
  count: number,
  radiusMm: number,
  fullAngleDeg = 360,
): Array<{ offset: [number, number, number]; angleRad: number }> {
  if (!Number.isInteger(count) || count < 1 || count > 256) {
    throw new RangeError('count must be an integer between 1 and 256.')
  }
  if (!Number.isFinite(radiusMm) || radiusMm < 0) {
    throw new RangeError('radiusMm must be a non-negative finite number.')
  }
  if (!Number.isFinite(fullAngleDeg) || Math.abs(fullAngleDeg) > 360 || (count > 1 && Math.abs(fullAngleDeg) === 0)) {
    throw new RangeError('fullAngleDeg must be a finite non-zero angle within [-360, 360].')
  }
  if (count === 1) {
    return [{ offset: [radiusMm === 0 ? 0 : radiusMm, 0, 0], angleRad: 0 }]
  }
  const isFullCircle = Math.abs(Math.abs(fullAngleDeg) - 360) < 1e-9
  const stepDeg = isFullCircle ? fullAngleDeg / count : fullAngleDeg / (count - 1)
  const transforms: Array<{ offset: [number, number, number]; angleRad: number }> = []
  for (let i = 0; i < count; i++) {
    const angleRad = ((i * stepDeg) * Math.PI) / 180
    const ox = radiusMm * Math.cos(angleRad)
    const oy = radiusMm * Math.sin(angleRad)
    transforms.push({
      offset: [Math.abs(ox) < 1e-12 ? 0 : ox, Math.abs(oy) < 1e-12 ? 0 : oy, 0],
      angleRad: Math.abs(angleRad) < 1e-12 ? 0 : angleRad,
    })
  }
  return transforms
}
