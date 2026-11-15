import type { Vec3 } from '../kernel/geometry/math3d'
import { frameFromAxes, type Frame } from './coordinateSystems'

/**
 * Saved gizmo pivots (roadmap C6). A pivot belongs to a selection, keyed by its sorted object
 * ids, so a single body and a multi-body selection each keep their own pivot. The optional
 * axes define the selection's Local coordinate system.
 */
export type PivotSnap = 'vertex' | 'center' | 'origin' | 'free'
export interface DirectPivot {
  key: string
  point: Vec3
  snap: PivotSnap
  /** Local frame: normal (Z) and X direction. Absent means world-aligned. */
  axes?: { x: Vec3; z: Vec3 }
}
/** Construction plane picked from a face. */
export interface DirectCPlane {
  origin: Vec3
  normal: Vec3
  xAxis: Vec3
}

export const MAX_DIRECT_PIVOTS = 1000
const SNAPS: readonly PivotSnap[] = ['vertex', 'center', 'origin', 'free']

export const pivotKey = (ids: readonly string[]): string => [...new Set(ids)].sort().join('\n')

const vec3 = (value: unknown): value is Vec3 =>
  Array.isArray(value) &&
  value.length === 3 &&
  value.every(v => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6)
const nonZero = (v: Vec3) => Math.hypot(...v) > 1e-9

export function validPivot(value: unknown): value is DirectPivot {
  if (!value || typeof value !== 'object') return false
  const p = value as Partial<DirectPivot>
  if (
    typeof p.key !== 'string' ||
    !p.key ||
    p.key.length > 64_000 ||
    !vec3(p.point) ||
    !SNAPS.includes(p.snap as PivotSnap)
  )
    return false
  if (p.axes !== undefined && !(p.axes && vec3(p.axes.x) && vec3(p.axes.z) && nonZero(p.axes.x) && nonZero(p.axes.z)))
    return false
  return Object.keys(p).every(k => ['key', 'point', 'snap', 'axes'].includes(k))
}

export const validCPlane = (value: unknown): value is DirectCPlane => {
  if (!value || typeof value !== 'object') return false
  const c = value as Partial<DirectCPlane>
  return vec3(c.origin) && vec3(c.normal) && vec3(c.xAxis) && nonZero(c.normal) && Object.keys(c).length === 3
}

/** Throws on malformed pivots; returns undefined when the list is empty so the key is omitted. */
export function validatePivots(value: unknown, objectIds: ReadonlySet<string>): DirectPivot[] | undefined {
  if (value === undefined) return undefined
  if (!Array.isArray(value) || value.length > MAX_DIRECT_PIVOTS) throw Error('Invalid saved pivot collection.')
  const keys = new Set<string>()
  const kept: DirectPivot[] = []
  for (const pivot of value) {
    if (!validPivot(pivot) || keys.has(pivot.key)) throw Error('Invalid saved pivot.')
    keys.add(pivot.key)
    // Pivots of objects that no longer exist are dropped instead of failing the whole document.
    if (pivot.key.split('\n').every(id => objectIds.has(id))) kept.push(pivot)
  }
  return kept.length ? kept : undefined
}

export interface PivotDocument {
  pivots?: DirectPivot[]
}

export function savedPivot(document: PivotDocument, ids: readonly string[]): DirectPivot | undefined {
  const key = pivotKey(ids)
  return document.pivots?.find(p => p.key === key)
}

/** Copy of the document with the selection's pivot set, or cleared when `pivot` is null. */
export function withSavedPivot<T extends PivotDocument>(
  document: T,
  ids: readonly string[],
  pivot: Omit<DirectPivot, 'key'> | null,
): T {
  if (!ids.length) throw Error('Select an object to set its pivot.')
  const key = pivotKey(ids)
  const others = (document.pivots ?? []).filter(p => p.key !== key)
  const next = pivot ? [...others, { key, ...pivot, point: [...pivot.point] as Vec3 }] : others
  const { pivots: _drop, ...rest } = document
  return (next.length ? { ...rest, pivots: next } : rest) as T
}

/** Snap a requested pivot to the nearest vertex, the selection centre or the origin. */
export function snapPivot(
  snap: PivotSnap,
  request: Vec3,
  context: { vertices: ArrayLike<number>; center: Vec3 },
): Vec3 {
  if (snap === 'origin') return [0, 0, 0]
  if (snap === 'center') return [...context.center] as Vec3
  if (snap === 'free') return [...request] as Vec3
  const v = context.vertices
  let best: Vec3 | null = null,
    bestDistance = Infinity
  for (let i = 0; i + 2 < v.length; i += 3) {
    const d = Math.hypot(v[i] - request[0], v[i + 1] - request[1], v[i + 2] - request[2])
    if (d < bestDistance) {
      bestDistance = d
      best = [v[i], v[i + 1], v[i + 2]]
    }
  }
  if (!best) throw Error('The selection has no vertices to snap to.')
  return best
}

export const pivotFrame = (pivot: DirectPivot | undefined): Frame | null =>
  pivot?.axes ? frameFromAxes(pivot.point, pivot.axes.z, pivot.axes.x) : null

export const cplaneFrame = (cplane: DirectCPlane | undefined): Frame | null =>
  cplane ? frameFromAxes(cplane.origin, cplane.normal, cplane.xAxis) : null

export const cplaneFromFrame = (frame: Frame): DirectCPlane => ({
  origin: [...frame.origin] as Vec3,
  normal: [...frame.z] as Vec3,
  xAxis: [...frame.x] as Vec3,
})
