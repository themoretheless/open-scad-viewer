import { cross3, dot3, unit3, type Vec3 } from '../kernel/geometry/math3d'

/**
 * Coordinate systems for gizmo moves, rotations and numeric input (roadmap C6).
 * World uses the document axes, CPlane a construction plane picked from a face, and
 * Local the frame saved with the selection's pivot (world axes when none is saved).
 */
export type CoordinateSystem = 'world' | 'cplane' | 'local'
export const COORDINATE_SYSTEMS: readonly CoordinateSystem[] = ['world', 'cplane', 'local']

/** Right-handed orthonormal frame. */
export interface Frame {
  origin: Vec3
  x: Vec3
  y: Vec3
  z: Vec3
}

export const WORLD_FRAME: Frame = Object.freeze({
  origin: [0, 0, 0],
  x: [1, 0, 0],
  y: [0, 1, 0],
  z: [0, 0, 1],
}) as Frame

export const nextCoordinateSystem = (current: CoordinateSystem): CoordinateSystem =>
  COORDINATE_SYSTEMS[(COORDINATE_SYSTEMS.indexOf(current) + 1) % COORDINATE_SYSTEMS.length]

export function coordinateSystemLabel(system: CoordinateSystem, locale: string): string {
  const ru = locale === 'ru'
  return system === 'world' ? (ru ? 'Мир' : 'World') : system === 'cplane' ? 'CPlane' : ru ? 'Локальная' : 'Local'
}

/** Orthonormal frame from an origin, a normal and a preferred X direction (projected onto the plane). */
export function frameFromAxes(origin: Vec3, normal: Vec3, xHint: Vec3): Frame {
  const z = unit3(normal)
  if (!Number.isFinite(z[0]) || Math.hypot(...z) < 0.5) throw Error('Frame normal must be non-zero.')
  let hint = xHint
  const along = dot3(hint, z)
  let x: Vec3 = [hint[0] - along * z[0], hint[1] - along * z[1], hint[2] - along * z[2]]
  if (Math.hypot(...x) < 1e-9) {
    hint = Math.abs(z[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0]
    const d = dot3(hint, z)
    x = [hint[0] - d * z[0], hint[1] - d * z[1], hint[2] - d * z[2]]
  }
  x = unit3(x)
  return { origin: [...origin] as Vec3, x, y: cross3(z, x), z }
}

/** CPlane from a planar face loop: Newell normal, X along the first non-degenerate edge, origin at the centroid. */
export function frameFromFace(points: readonly Vec3[]): Frame {
  if (points.length < 3) throw Error('A face needs at least three points.')
  const n: Vec3 = [0, 0, 0]
  const c: Vec3 = [0, 0, 0]
  for (let i = 0; i < points.length; i++) {
    const a = points[i],
      b = points[(i + 1) % points.length]
    n[0] += (a[1] - b[1]) * (a[2] + b[2])
    n[1] += (a[2] - b[2]) * (a[0] + b[0])
    n[2] += (a[0] - b[0]) * (a[1] + b[1])
    for (let k = 0; k < 3; k++) c[k] += a[k] / points.length
  }
  if (Math.hypot(...n) < 1e-12) throw Error('The face is degenerate.')
  let edge: Vec3 = [1, 0, 0]
  for (let i = 0; i < points.length; i++) {
    const a = points[i],
      b = points[(i + 1) % points.length],
      d: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
    if (Math.hypot(...d) > 1e-9) {
      edge = d
      break
    }
  }
  return frameFromAxes(c, n, edge)
}

export const frameAxis = (frame: Frame, axis: 'x' | 'y' | 'z'): Vec3 => frame[axis]

/** A delta given in frame components, expressed in world components. */
export function frameDeltaToWorld(frame: Frame, delta: Vec3): Vec3 {
  return [0, 1, 2].map(k => delta[0] * frame.x[k] + delta[1] * frame.y[k] + delta[2] * frame.z[k]) as Vec3
}

/** World point to frame coordinates (what the numeric panel shows). */
export function worldToFrame(frame: Frame, point: Vec3): Vec3 {
  const d: Vec3 = [point[0] - frame.origin[0], point[1] - frame.origin[1], point[2] - frame.origin[2]]
  return [dot3(d, frame.x), dot3(d, frame.y), dot3(d, frame.z)]
}

/** Frame coordinates (typed into the panel) back to a world point. */
export function frameToWorld(frame: Frame, point: Vec3): Vec3 {
  const d = frameDeltaToWorld(frame, point)
  return [frame.origin[0] + d[0], frame.origin[1] + d[1], frame.origin[2] + d[2]]
}

/** Rodrigues rotation of `point` about the line through `pivot` along `axis`, in degrees. */
export function rotateAboutAxis(point: Vec3, pivot: Vec3, axis: Vec3, degrees: number): Vec3 {
  const k = unit3(axis),
    t = (degrees * Math.PI) / 180,
    cos = Math.cos(t),
    sin = Math.sin(t)
  const v: Vec3 = [point[0] - pivot[0], point[1] - pivot[1], point[2] - pivot[2]]
  const kv = cross3(k, v),
    kd = dot3(k, v) * (1 - cos)
  return [0, 1, 2].map(i => pivot[i] + v[i] * cos + kv[i] * sin + k[i] * kd) as Vec3
}

/** Move all points by `amount` along one frame axis. */
export function moveAlongFrameAxis(
  points: readonly Vec3[],
  frame: Frame,
  axis: 'x' | 'y' | 'z',
  amount: number,
): Vec3[] {
  const d = frame[axis]
  return points.map(p => [p[0] + d[0] * amount, p[1] + d[1] * amount, p[2] + d[2] * amount] as Vec3)
}

/** Resolve the active frame; CPlane and Local fall back to World when not defined. */
export function activeFrame(system: CoordinateSystem, frames: { cplane?: Frame | null; local?: Frame | null }): Frame {
  if (system === 'cplane' && frames.cplane) return frames.cplane
  if (system === 'local' && frames.local) return frames.local
  return WORLD_FRAME
}
