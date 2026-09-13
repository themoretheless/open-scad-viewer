import type { Aabb3, Ray3 } from './math3d'
import { callGeometryRust } from './geometry/kernel'

export interface SceneAabbItem {
  readonly id: number
  readonly bounds: Aabb3
}
export interface SceneAabbCandidate {
  readonly id: number
  /** Non-negative ray parameter at which the ray enters the object bound. */
  readonly distance: number
}
export interface SceneAabbIndex {
  readonly itemCount: number
  readonly nodeCount: number
}
const handles = new WeakMap<SceneAabbIndex, string>()
const disposed = new WeakSet<SceneAabbIndex>()

/** Upload once. Rust validates bounds and constructs/owns the hierarchy. */
export function buildSceneAabbIndex(source: readonly SceneAabbItem[], requestedLeafSize = 4): SceneAabbIndex {
  // The binary transport admits finite numbers only. Geometric validity,
  // hierarchy construction and all intersection calculations belong to Rust.
  const items = source.filter(item => Number.isSafeInteger(item.id)
    && item.bounds.min.every(Number.isFinite) && item.bounds.max.every(Number.isFinite))
  if (!items.length) return Object.freeze({ itemCount: 0, nodeCount: 0 })
  const leafSize = Number.isFinite(requestedLeafSize) ? Math.max(1, Math.min(32, Math.trunc(requestedLeafSize))) : 4
  const result = callGeometryRust<{ handle: string; itemCount: number; nodeCount: number }>(
    'scene_picking', { action: 'create', items, leafSize })
  const index = Object.freeze({ itemCount: result.itemCount, nodeCount: result.nodeCount })
  handles.set(index, result.handle)
  return index
}

/** Release with scene replacement or renderer teardown. Idempotent on the host. */
export function disposeSceneAabbIndex(index: SceneAabbIndex): void {
  const handle = handles.get(index)
  if (handle !== undefined) {
    callGeometryRust('scene_picking', { action: 'dispose', handle })
    handles.delete(index)
  }
  disposed.add(index)
}

/** Query the retained native index; only the ray and candidates cross the ABI. */
export function querySceneAabbIndex(index: SceneAabbIndex, ray: Ray3, maximumDistance = Infinity): SceneAabbCandidate[] {
  if (disposed.has(index)) throw new Error('Scene picking index has been disposed')
  const handle = handles.get(index)
  if (handle === undefined || ![...ray.origin, ...ray.direction].every(Number.isFinite)) return []
  return callGeometryRust('scene_picking', {
    action: 'query', handle, origin: ray.origin, direction: ray.direction,
    maximum: Number.isFinite(maximumDistance) ? maximumDistance : null,
  })
}
