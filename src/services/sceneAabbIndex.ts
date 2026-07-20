import { rayAabbDistance, type Aabb3, type Ray3, type Vec3 } from './math3d'

/** A world-space object bound stored in the scene-level picking index. */
export interface SceneAabbItem {
  readonly id: number
  readonly bounds: Aabb3
}

export interface SceneAabbCandidate {
  readonly id: number
  /** Non-negative ray parameter at which the ray enters the object bound. */
  readonly distance: number
}

interface IndexedSceneAabbItem extends SceneAabbItem {
  readonly center: Vec3
}

export type SceneAabbNode = {
  readonly bounds: Aabb3
  readonly left: SceneAabbNode | null
  readonly right: SceneAabbNode | null
  readonly items: readonly IndexedSceneAabbItem[]
}

export interface SceneAabbIndex {
  readonly root: SceneAabbNode | null
  readonly itemCount: number
  readonly nodeCount: number
}

const DEFAULT_LEAF_SIZE = 4
const MAX_LEAF_SIZE = 32

function finiteBounds(bounds: Aabb3): boolean {
  return bounds.min.every(Number.isFinite)
    && bounds.max.every(Number.isFinite)
    && bounds.min[0] <= bounds.max[0]
    && bounds.min[1] <= bounds.max[1]
    && bounds.min[2] <= bounds.max[2]
}

function cloneBounds(bounds: Aabb3): Aabb3 {
  return {
    min: [...bounds.min] as Vec3,
    max: [...bounds.max] as Vec3,
  }
}

function combine(items: readonly IndexedSceneAabbItem[]): Aabb3 {
  const min: Vec3 = [Infinity, Infinity, Infinity]
  const max: Vec3 = [-Infinity, -Infinity, -Infinity]
  for (const item of items) {
    for (let axis = 0; axis < 3; axis++) {
      min[axis] = Math.min(min[axis], item.bounds.min[axis])
      max[axis] = Math.max(max[axis], item.bounds.max[axis])
    }
  }
  return { min, max }
}

/**
 * Build a small deterministic BVH over world-space object bounds.
 *
 * This is deliberately separate from each mesh's triangle BVH: it cheaply
 * rejects whole objects before the renderer asks their more expensive BVHs
 * for exact triangle hits.
 */
export function buildSceneAabbIndex(
  source: readonly SceneAabbItem[],
  requestedLeafSize = DEFAULT_LEAF_SIZE,
): SceneAabbIndex {
  const leafSize = Number.isFinite(requestedLeafSize)
    ? Math.max(1, Math.min(MAX_LEAF_SIZE, Math.trunc(requestedLeafSize)))
    : DEFAULT_LEAF_SIZE
  const items: IndexedSceneAabbItem[] = []
  for (const item of source) {
    if (!Number.isSafeInteger(item.id) || !finiteBounds(item.bounds)) continue
    const bounds = cloneBounds(item.bounds)
    items.push({
      id: item.id,
      bounds,
      center: [
        bounds.min[0] + (bounds.max[0] - bounds.min[0]) * 0.5,
        bounds.min[1] + (bounds.max[1] - bounds.min[1]) * 0.5,
        bounds.min[2] + (bounds.max[2] - bounds.min[2]) * 0.5,
      ],
    })
  }

  let nodeCount = 0
  const buildNode = (nodeItems: IndexedSceneAabbItem[]): SceneAabbNode => {
    nodeCount++
    const bounds = combine(nodeItems)
    if (nodeItems.length <= leafSize) {
      // Stable leaf order makes equal-distance query results reproducible.
      nodeItems.sort((left, right) => left.id - right.id)
      return { bounds, left: null, right: null, items: nodeItems }
    }

    const centroidMin: Vec3 = [Infinity, Infinity, Infinity]
    const centroidMax: Vec3 = [-Infinity, -Infinity, -Infinity]
    for (const item of nodeItems) {
      for (let axis = 0; axis < 3; axis++) {
        centroidMin[axis] = Math.min(centroidMin[axis], item.center[axis])
        centroidMax[axis] = Math.max(centroidMax[axis], item.center[axis])
      }
    }
    let axis = 0
    if (centroidMax[1] - centroidMin[1] > centroidMax[axis] - centroidMin[axis]) axis = 1
    if (centroidMax[2] - centroidMin[2] > centroidMax[axis] - centroidMin[axis]) axis = 2
    nodeItems.sort((left, right) => left.center[axis] - right.center[axis] || left.id - right.id)
    const middle = nodeItems.length >>> 1
    return {
      bounds,
      left: buildNode(nodeItems.slice(0, middle)),
      right: buildNode(nodeItems.slice(middle)),
      items: [],
    }
  }

  const root = items.length ? buildNode(items) : null
  return { root, itemCount: items.length, nodeCount }
}

/** Return object bounds intersected by a ray, ordered front-to-back. */
export function querySceneAabbIndex(
  index: SceneAabbIndex,
  ray: Ray3,
  maximumDistance = Infinity,
): SceneAabbCandidate[] {
  const maxDistance = Number.isFinite(maximumDistance)
    ? Math.max(0, maximumDistance)
    : Infinity
  if (!index.root) return []
  const rootDistance = rayAabbDistance(ray, index.root.bounds, maxDistance)
  if (rootDistance === null) return []

  const pending: SceneAabbNode[] = [index.root]
  const candidates: SceneAabbCandidate[] = []

  // Node order does not affect correctness: item entry distances are sorted
  // once after the hierarchy has rejected non-intersecting branches.
  while (pending.length) {
    const current = pending.pop()!
    if (current.left || current.right) {
      for (const child of [current.left, current.right]) {
        if (!child) continue
        if (rayAabbDistance(ray, child.bounds, maxDistance) !== null) pending.push(child)
      }
      continue
    }

    for (const item of current.items) {
      const distance = rayAabbDistance(ray, item.bounds, maxDistance)
      if (distance !== null) candidates.push({ id: item.id, distance })
    }
  }

  candidates.sort((left, right) => left.distance - right.distance || left.id - right.id)
  return candidates
}
