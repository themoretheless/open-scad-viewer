import {transformRetainedSketch,offsetRetainedSketch,requirePolylineSketch} from './retainedSketchProfile'
import {callGeometryRust} from './geometry/kernel'
import type { DirectSketch, Point2 } from './directModeling'
export type Vec3 = [number,number,number]
export interface SketchPlane { origin:Vec3; u:Vec3; v:Vec3 }
export interface AnalyticCurve { kind:'circle'|'arc'; center:Point2; radius:number; start:number; sweep:number }
export const dot3=(a:number[],b:number[])=>a.reduce((s,v,i)=>s+v*b[i],0)
export const cross3=(a:number[],b:number[]):Vec3=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
export const unit3=(a:number[]):Vec3=>{const n=Math.hypot(...a);if(n<1e-9)throw Error('Zero direction');return a.map(v=>v/n) as Vec3}
export const xyPlane=():SketchPlane=>({origin:[0,0,0],u:[1,0,0],v:[0,1,0]})
export function worldPoint(p: number[], plane: SketchPlane = xyPlane()): Vec3 {
  if (!Array.isArray(p) || (p.length !== 2 && p.length !== 3)) {
    throw new Error('Sketch placement requires 2D or 3D points.')
  }
  const o = plane.origin, u = plane.u, v = plane.v
  const nx = u[1] * v[2] - u[2] * v[1]
  const ny = u[2] * v[0] - u[0] * v[2]
  const nz = u[0] * v[1] - u[1] * v[0]
  const x = p[0], y = p[1], z = p.length === 3 ? p[2] : 0
  if (
    !Number.isFinite(o[0]) || !Number.isFinite(o[1]) || !Number.isFinite(o[2]) ||
    !Number.isFinite(u[0]) || !Number.isFinite(u[1]) || !Number.isFinite(u[2]) ||
    !Number.isFinite(v[0]) || !Number.isFinite(v[1]) || !Number.isFinite(v[2]) ||
    !Number.isFinite(nx) || !Number.isFinite(ny) || !Number.isFinite(nz) ||
    !Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)
  ) {
    throw new Error('Sketch geometry exceeds finite numeric range.')
  }
  const wx = o[0] + x * u[0] + y * v[0] + z * nx
  const wy = o[1] + x * u[1] + y * v[1] + z * ny
  const wz = o[2] + x * u[2] + y * v[2] + z * nz
  if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) {
    throw new Error('Sketch geometry exceeds finite numeric range.')
  }
  return [wx, wy, wz]
}

export function worldPoints(points: number[][], plane: SketchPlane = xyPlane()): Vec3[] {
  if (points.length > 300000) throw new Error('Sketch placement point budget exceeded.')
  const o = plane.origin, u = plane.u, v = plane.v
  const nx = u[1] * v[2] - u[2] * v[1]
  const ny = u[2] * v[0] - u[0] * v[2]
  const nz = u[0] * v[1] - u[1] * v[0]
  if (
    !Number.isFinite(o[0]) || !Number.isFinite(o[1]) || !Number.isFinite(o[2]) ||
    !Number.isFinite(u[0]) || !Number.isFinite(u[1]) || !Number.isFinite(u[2]) ||
    !Number.isFinite(v[0]) || !Number.isFinite(v[1]) || !Number.isFinite(v[2]) ||
    !Number.isFinite(nx) || !Number.isFinite(ny) || !Number.isFinite(nz)
  ) {
    throw new Error('Sketch geometry exceeds finite numeric range.')
  }
  const out = new Array<Vec3>(points.length)
  for (let i = 0; i < points.length; i++) {
    const p = points[i]
    if (!Array.isArray(p) || (p.length !== 2 && p.length !== 3)) {
      throw new Error('Sketch placement requires 2D or 3D points.')
    }
    const x = p[0], y = p[1], z = p.length === 3 ? p[2] : 0
    if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
      throw new Error('Sketch geometry exceeds finite numeric range.')
    }
    const wx = o[0] + x * u[0] + y * v[0] + z * nx
    const wy = o[1] + x * u[1] + y * v[1] + z * ny
    const wz = o[2] + x * u[2] + y * v[2] + z * nz
    if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) {
      throw new Error('Sketch geometry exceeds finite numeric range.')
    }
    out[i] = [wx, wy, wz]
  }
  return out
}

function arcDirection(degrees: number): [number, number] {
  const r = degrees % 360
  const a = r < 0 ? (r + 360 === 360 ? 0 : r + 360) : r
  if (a === 0) return [1, 0]
  if (a === 90) return [0, 1]
  if (a === 180) return [-1, 0]
  if (a === 270) return [0, -1]
  const rad = a * (Math.PI / 180)
  return [Math.cos(rad), Math.sin(rad)]
}

export function sampleCurve(c: AnalyticCurve): Point2[] {
  if (
    !c ||
    (c.kind !== 'circle' && c.kind !== 'arc') ||
    !Array.isArray(c.center) ||
    c.center.length !== 2 ||
    !Number.isFinite(c.center[0]) ||
    !Number.isFinite(c.center[1]) ||
    !Number.isFinite(c.radius) ||
    !Number.isFinite(c.start) ||
    !Number.isFinite(c.sweep) ||
    c.radius < 0.01 ||
    c.radius > 1e6 ||
    Math.abs(c.sweep) < 0.1 ||
    Math.abs(c.sweep) > 360
  ) {
    throw new Error('Invalid circle or arc parameters.')
  }
  const sweep = c.kind === 'circle' ? 360 : c.sweep
  const n = Math.max(8, Math.ceil(Math.abs(sweep) / 3))
  const count = c.kind === 'circle' ? n : n + 1
  const points = new Array<Point2>(count)
  for (let i = 0; i < count; i++) {
    const [cos, sin] = arcDirection(i === n ? c.start + sweep : c.start + (sweep * i) / n)
    const px = c.center[0] + c.radius * cos
    const py = c.center[1] + c.radius * sin
    if (!Number.isFinite(px) || !Number.isFinite(py)) {
      throw new Error('Sketch geometry exceeds finite numeric range.')
    }
    points[i] = [px, py]
  }
  return points
}

export function bakeSketch(s: DirectSketch): DirectSketch {
  requirePolylineSketch(s)
  const next = structuredClone(s)
  delete next.analytic
  return next
}

export function transformSketch(s: DirectSketch, delta: Point2, angle: number, scale: number, pivot?: Point2): DirectSketch {
  if (s.editablePath) requirePolylineSketch(s)
  if (s.retainedProfile) return transformRetainedSketch(s, delta, angle, scale, pivot)
  if (
    !Array.isArray(delta) ||
    delta.length !== 2 ||
    !Number.isFinite(delta[0]) ||
    !Number.isFinite(delta[1]) ||
    !Number.isFinite(angle) ||
    !Number.isFinite(scale) ||
    scale <= 0
  ) {
    throw new Error('Invalid sketch transform.')
  }
  const pts = s.points
  if (!Array.isArray(pts) || pts.length === 0) {
    throw new Error('Sketch requires points.')
  }
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i]
    if (!Array.isArray(p) || p.length !== 2 || !Number.isFinite(p[0]) || !Number.isFinite(p[1])) {
      throw new Error('Sketch geometry exceeds finite numeric range.')
    }
  }
  let cx: number, cy: number
  if (pivot !== undefined && pivot !== null) {
    cx = pivot[0]
    cy = pivot[1]
  } else if (s.analytic) {
    cx = s.analytic.center[0]
    cy = s.analytic.center[1]
  } else {
    cx = 0
    cy = 0
    const n = pts.length
    for (let i = 0; i < n; i++) {
      cx += pts[i][0] / n
      cy += pts[i][1] / n
    }
  }
  if (!Number.isFinite(cx) || !Number.isFinite(cy)) {
    throw new Error('Sketch geometry exceeds finite numeric range.')
  }
  const rad = angle * (Math.PI / 180)
  const sin = Math.sin(rad)
  const cos = Math.cos(rad)
  const apply = (p: readonly [number, number]): Point2 => {
    const x = p[0] - cx
    const y = p[1] - cy
    return [cx + delta[0] + scale * (x * cos - y * sin), cy + delta[1] + scale * (x * sin + y * cos)]
  }
  let moved = new Array<Point2>(pts.length)
  for (let i = 0; i < pts.length; i++) {
    const mp = apply(pts[i])
    if (!Number.isFinite(mp[0]) || !Number.isFinite(mp[1])) {
      throw new Error('Sketch geometry exceeds finite numeric range.')
    }
    moved[i] = mp
  }
  const next = structuredClone(s)
  if (next.analytic) {
    const center = apply(next.analytic.center)
    const radius = next.analytic.radius * scale
    const start = next.analytic.start + angle
    if (!Number.isFinite(center[0]) || !Number.isFinite(center[1]) || !Number.isFinite(radius) || !Number.isFinite(start)) {
      throw new Error('Sketch geometry exceeds finite numeric range.')
    }
    next.analytic.center = center
    next.analytic.radius = radius
    next.analytic.start = start
    moved = sampleCurve(next.analytic)
  }
  next.points = moved
  return next
}

export function validateSimpleSketch(points: Point2[], closed = true) {
  callGeometryRust('cad_validate_sketch', { points, closed })
}

export function offsetSketch(s: DirectSketch, distance: number): DirectSketch {
  if (s.editablePath) requirePolylineSketch(s)
  if (s.retainedProfile) return offsetRetainedSketch(s, distance)
  return callGeometryRust('cad_offset_sketch', { sketch: s, distance })
}

/** Trim a clicked segment between intersections, retaining remaining chains. */
export function trimSketch(s: DirectSketch, edge: number, at: number, boundaries: DirectSketch[]): DirectSketch[] {
  requirePolylineSketch(s)
  boundaries.forEach(requirePolylineSketch)
  return callGeometryRust('cad_trim_sketch', { sketch: s, edge, at, boundaries })
}

export function extendSketch(s: DirectSketch, end: 'start' | 'end', boundaries: DirectSketch[]): DirectSketch {
  requirePolylineSketch(s)
  boundaries.forEach(requirePolylineSketch)
  return callGeometryRust('cad_extend_sketch', { sketch: s, end, boundaries })
}

/** Tessellated closed slot; width is the full end-cap diameter. */
export function slotSketch(a: Point2, b: Point2, width: number): Point2[] {
  if (
    !Array.isArray(a) ||
    !Array.isArray(b) ||
    a.length !== 2 ||
    b.length !== 2 ||
    !Number.isFinite(a[0]) ||
    !Number.isFinite(a[1]) ||
    !Number.isFinite(b[0]) ||
    !Number.isFinite(b[1]) ||
    !Number.isFinite(width) ||
    Math.abs(a[0]) > 1e6 ||
    Math.abs(a[1]) > 1e6 ||
    Math.abs(b[0]) > 1e6 ||
    Math.abs(b[1]) > 1e6 ||
    Math.abs(width) > 1e6 ||
    width < 0.02 ||
    Math.hypot(b[0] - a[0], b[1] - a[1]) < 1e-6
  ) {
    throw new Error('Slot requires distinct centers and width between 0.02 and 1000000 mm.')
  }
  const angle = Math.atan2(b[1] - a[1], b[0] - a[0])
  const points = new Array<Point2>(66)
  let idx = 0
  const caps: Array<[Point2, number]> = [
    [b, angle - Math.PI / 2],
    [a, angle + Math.PI / 2],
  ]
  for (let c = 0; c < 2; c++) {
    const [center, start] = caps[c]
    for (let i = 0; i <= 32; i++) {
      const t = start + (Math.PI * i) / 32
      const px = center[0] + width * 0.5 * Math.cos(t)
      const py = center[1] + width * 0.5 * Math.sin(t)
      if (!Number.isFinite(px) || !Number.isFinite(py) || Math.abs(px) > 1e6 || Math.abs(py) > 1e6) {
        throw new Error('Slot exceeds coordinate range.')
      }
      points[idx++] = [px, py]
    }
  }
  return points
}
