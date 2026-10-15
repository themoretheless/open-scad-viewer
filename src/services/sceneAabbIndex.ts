import type { Aabb3, Ray3 } from './math3d'
import { GeometryKernelError } from './geometry/kernel'

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

interface BvhItem {
  readonly id: number
  readonly min: [number, number, number]
  readonly max: [number, number, number]
}

interface BvhNode {
  readonly min: [number, number, number]
  readonly max: [number, number, number]
  readonly items: BvhItem[] | null
  readonly left: BvhNode | null
  readonly right: BvhNode | null
}

interface RetainedTree {
  readonly root: BvhNode | null
  readonly itemCount: number
}

const MAX_ITEMS = 100_000
const MAX_TREES = 64
const activeTrees = new Map<SceneAabbIndex, RetainedTree>()
const disposed = new WeakSet<SceneAabbIndex>()
let activeItemCount = 0

function center(item: BvhItem, axis: number): number {
  return item.min[axis] * 0.5 + item.max[axis] * 0.5
}

function buildNode(items: BvhItem[], leafSize: number, counter: { count: number }): BvhNode {
  counter.count++
  let min0 = Infinity, min1 = Infinity, min2 = Infinity
  let max0 = -Infinity, max1 = -Infinity, max2 = -Infinity
  let cmin0 = Infinity, cmin1 = Infinity, cmin2 = Infinity
  let cmax0 = -Infinity, cmax1 = -Infinity, cmax2 = -Infinity
  for (let i = 0; i < items.length; i++) {
    const it = items[i]
    if (it.min[0] < min0) min0 = it.min[0]
    if (it.min[1] < min1) min1 = it.min[1]
    if (it.min[2] < min2) min2 = it.min[2]
    if (it.max[0] > max0) max0 = it.max[0]
    if (it.max[1] > max1) max1 = it.max[1]
    if (it.max[2] > max2) max2 = it.max[2]
    const c0 = it.min[0] * 0.5 + it.max[0] * 0.5
    const c1 = it.min[1] * 0.5 + it.max[1] * 0.5
    const c2 = it.min[2] * 0.5 + it.max[2] * 0.5
    if (c0 < cmin0) cmin0 = c0
    if (c1 < cmin1) cmin1 = c1
    if (c2 < cmin2) cmin2 = c2
    if (c0 > cmax0) cmax0 = c0
    if (c1 > cmax1) cmax1 = c1
    if (c2 > cmax2) cmax2 = c2
  }
  const min: [number, number, number] = [min0, min1, min2]
  const max: [number, number, number] = [max0, max1, max2]
  if (items.length <= leafSize) {
    items.sort((a, b) => a.id - b.id)
    return { min, max, items, left: null, right: null }
  }
  const span0 = cmax0 - cmin0
  const span1 = cmax1 - cmin1
  const span2 = cmax2 - cmin2
  let axis = 0
  let bestSpan = span0
  if (span1 > bestSpan) { axis = 1; bestSpan = span1 }
  if (span2 > bestSpan) { axis = 2 }
  items.sort((a, b) => {
    const ca = center(a, axis)
    const cb = center(b, axis)
    if (ca < cb) return -1
    if (ca > cb) return 1
    return a.id - b.id
  })
  const mid = (items.length / 2) | 0
  const leftItems = items.slice(0, mid)
  const rightItems = items.slice(mid)
  const left = buildNode(leftItems, leafSize, counter)
  const right = buildNode(rightItems, leafSize, counter)
  return { min, max, items: null, left, right }
}

function parameter(bound: number, origin: number, direction: number): number {
  const delta = bound - origin
  return Number.isFinite(delta) ? delta / direction : (bound * 0.5 - origin * 0.5) / (direction * 0.5)
}

function entryDistance(
  origin: readonly [number, number, number],
  direction: readonly [number, number, number],
  min: readonly [number, number, number],
  max: readonly [number, number, number],
  maximum: number,
): number | null {
  let near = 0
  let far = maximum
  for (let a = 0; a < 3; a++) {
    const dir = direction[a]
    const orig = origin[a]
    if (dir === 0) {
      if (orig < min[a] || orig > max[a]) return null
      continue
    }
    let start = parameter(min[a], orig, dir)
    let end = parameter(max[a], orig, dir)
    if (start > end) {
      const tmp = start
      start = end
      end = tmp
    }
    if (start > near) near = start
    if (end < far) far = end
    if (far < near) return null
  }
  return Number.isFinite(near) ? near : null
}

/** Build and own the scene AABB hierarchy directly in f64 without per-query WASM serialization. */
export function buildSceneAabbIndex(source: readonly SceneAabbItem[], requestedLeafSize = 4): SceneAabbIndex {
  const admitted: SceneAabbItem[] = []
  for (let i = 0; i < source.length; i++) {
    const item = source[i]
    if (
      Number.isSafeInteger(item.id) &&
      item.bounds.min.every(Number.isFinite) &&
      item.bounds.max.every(Number.isFinite)
    ) {
      admitted.push(item)
    }
  }
  if (!admitted.length) return Object.freeze({ itemCount: 0, nodeCount: 0 })
  if (activeTrees.size >= MAX_TREES || admitted.length > MAX_ITEMS - activeItemCount) {
    throw new GeometryKernelError('GEOMETRY_INVALID_INPUT', 'Scene picking index budget exceeded')
  }
  const leafSize = Number.isFinite(requestedLeafSize) ? Math.max(1, Math.min(32, Math.trunc(requestedLeafSize))) : 4
  const validItems: BvhItem[] = []
  for (let i = 0; i < admitted.length; i++) {
    const item = admitted[i]
    const min = item.bounds.min
    const max = item.bounds.max
    if (min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]) {
      validItems.push({
        id: item.id,
        min: [min[0], min[1], min[2]],
        max: [max[0], max[1], max[2]],
      })
    }
  }
  const counter = { count: 0 }
  const root = validItems.length ? buildNode(validItems, leafSize, counter) : null
  const index: SceneAabbIndex = Object.freeze({ itemCount: validItems.length, nodeCount: counter.count })
  activeTrees.set(index, { root, itemCount: validItems.length })
  activeItemCount += validItems.length
  return index
}

/** Release with scene replacement or renderer teardown. Idempotent on the host. */
export function disposeSceneAabbIndex(index: SceneAabbIndex): void {
  const tree = activeTrees.get(index)
  if (tree !== undefined) {
    activeItemCount -= tree.itemCount
    activeTrees.delete(index)
  }
  disposed.add(index)
}

/** Query the retained hierarchy directly in f64 with zero WASM/codec overhead. */
export function querySceneAabbIndex(index: SceneAabbIndex, ray: Ray3, maximumDistance = Infinity): SceneAabbCandidate[] {
  if (disposed.has(index)) throw new Error('Scene picking index has been disposed')
  const tree = activeTrees.get(index)
  if (tree === undefined) return []
  const ox = ray.origin[0], oy = ray.origin[1], oz = ray.origin[2]
  const dx = ray.direction[0], dy = ray.direction[1], dz = ray.direction[2]
  if (
    !Number.isFinite(ox) || !Number.isFinite(oy) || !Number.isFinite(oz) ||
    !Number.isFinite(dx) || !Number.isFinite(dy) || !Number.isFinite(dz)
  ) {
    return []
  }
  if (!tree.root || (dx === 0 && dy === 0 && dz === 0)) return []
  const maximum = Number.isFinite(maximumDistance) ? Math.max(0, maximumDistance) : Infinity
  const pending: BvhNode[] = [tree.root]
  const result: SceneAabbCandidate[] = []
  while (pending.length) {
    const node = pending.pop()!
    if (entryDistance(ray.origin, ray.direction, node.min, node.max, maximum) === null) continue
    if (node.items !== null) {
      const items = node.items
      for (let i = 0; i < items.length; i++) {
        const item = items[i]
        const distance = entryDistance(ray.origin, ray.direction, item.min, item.max, maximum)
        if (distance !== null) {
          result.push({ id: item.id, distance })
        }
      }
    } else {
      pending.push(node.left!, node.right!)
    }
  }
  result.sort((a, b) => a.distance - b.distance || a.id - b.id)
  return result
}
