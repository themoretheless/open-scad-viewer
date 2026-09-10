/**
 * A compact, transferable triangle BVH for CPU picking.
 *
 * Construction runs in the Rust geometry kernel (crates/polygon-core/src/bvh.rs)
 * through the raw-buffer binding in services/geometry/meshAnalysis.ts; this
 * module keeps option clamping, the empty-mesh fast path and the numeric
 * contract (f64 arithmetic over Float32 inputs). Raycasting deliberately
 * stays here: it is an interactive bridge interaction over the TS-resident
 * BVH that is transferred between workers and stored on scene objects.
 *
 * Nodes are stored in two typed arrays. `bounds` contains six floats per node
 * (`minX, minY, minZ, maxX, maxY, maxZ`) and `nodes` contains two uints:
 *
 * - branch: `[leftChild, rightChild]`
 * - leaf: `[firstTriangleSlot, LEAF_BIT | triangleCount]`
 *
 * `triangles` maps leaf slots back to the original triangle number in the
 * indexed mesh. Invalid and exactly degenerate triangles are omitted.
 */

import type { MeshBvh } from '../core/mesh'
import { buildBvhInKernel } from './geometry/meshAnalysis'

export type { MeshBvh } from '../core/mesh'

export type Vec3Tuple = [number, number, number]

export interface BuildMeshBvhOptions {
  /** Number of floats per vertex. Position must occupy the first three. */
  vertexStride?: number
  /** Maximum triangles per leaf. Clamped to 1..64. Defaults to 8. */
  leafSize?: number
}

export interface BvhRay {
  readonly origin: readonly [number, number, number]
  /** May be unnormalised. `t` is expressed in this direction's parameter. */
  readonly direction: readonly [number, number, number]
}

export interface RaycastMeshBvhOptions {
  /** Inclusive lower ray parameter. Defaults to 0. */
  minT?: number
  /** Inclusive upper ray parameter. Defaults to Infinity. */
  maxT?: number
  /** Triangle identities to skip, used by bounded same-depth hit traversal. */
  excludedTriangles?: ReadonlySet<number>
  /**
   * Optional affine row-major matrix mapping the supplied world-space ray to
   * mesh-local space. Its direction is deliberately not normalised, so the
   * resulting `t` remains in the supplied world ray's parameterisation.
   */
  localFromWorld?: ArrayLike<number>
}

export interface MeshBvhHit {
  /** Original triangle number (`indices[triangleIndex * 3]`). */
  readonly triangleIndex: number
  readonly triangleVertexIndices: [number, number, number]
  readonly t: number
  /** Barycentric weights corresponding to triangle vertices A, B and C. */
  readonly barycentric: [number, number, number]
  readonly localPoint: Vec3Tuple
  /** Point in the coordinate system of the ray passed to `raycastMeshBvh`. */
  readonly worldPoint: Vec3Tuple
  readonly localNormal: Vec3Tuple
  /** Normal transformed back to the ray's coordinate system. */
  readonly worldNormal: Vec3Tuple
  readonly frontFace: boolean
}

const LEAF_BIT = 0x80000000
const LEAF_COUNT_MASK = 0x7fffffff
const DEFAULT_VERTEX_STRIDE = 6
const DEFAULT_LEAF_SIZE = 8
const MAX_LEAF_SIZE = 64
const BARYCENTRIC_EPSILON = 1e-12

function emptyBvh(vertexStride: number, leafSize: number): MeshBvh {
  return {
    version: 1,
    vertexStride,
    leafSize,
    nodeCount: 0,
    bounds: new Float32Array(0),
    nodes: new Uint32Array(0),
    triangles: new Uint32Array(0),
  }
}

function finiteInteger(value: number | undefined, fallback: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, Math.trunc(value!)))
}

/**
 * Build a deterministic, balanced median-split BVH.
 *
 * The computation runs in the Rust kernel; TypeScript retains option
 * clamping and the empty-mesh fast path. Construction is O(n log n) and does
 * not mutate its inputs.
 */
export function buildMeshBvh(
  vertices: Float32Array,
  indices: Uint32Array,
  options: BuildMeshBvhOptions = {},
): MeshBvh {
  const vertexStride = finiteInteger(options.vertexStride, DEFAULT_VERTEX_STRIDE, 3, 256)
  const leafSize = finiteInteger(options.leafSize, DEFAULT_LEAF_SIZE, 1, MAX_LEAF_SIZE)
  const triangleCount = Math.floor(indices.length / 3)
  const vertexCount = Math.floor(vertices.length / vertexStride)
  if (triangleCount === 0 || vertexCount === 0) return emptyBvh(vertexStride, leafSize)

  const built = buildBvhInKernel(vertices, indices, vertexStride, leafSize)
  if (built.nodeCount === 0) return emptyBvh(vertexStride, leafSize)
  return {
    version: 1,
    vertexStride,
    leafSize,
    nodeCount: built.nodeCount,
    bounds: built.bounds,
    nodes: built.nodes,
    triangles: built.triangles,
  }
}

function transformPoint(matrix: ArrayLike<number>, point: readonly [number, number, number]): Vec3Tuple {
  const x = point[0], y = point[1], z = point[2]
  const w = matrix[12] * x + matrix[13] * y + matrix[14] * z + matrix[15]
  const inverseW = Number.isFinite(w) && Math.abs(w) > 1e-15 ? 1 / w : 1
  return [
    (matrix[0] * x + matrix[1] * y + matrix[2] * z + matrix[3]) * inverseW,
    (matrix[4] * x + matrix[5] * y + matrix[6] * z + matrix[7]) * inverseW,
    (matrix[8] * x + matrix[9] * y + matrix[10] * z + matrix[11]) * inverseW,
  ]
}

function transformVector(matrix: ArrayLike<number>, vector: readonly [number, number, number]): Vec3Tuple {
  const x = vector[0], y = vector[1], z = vector[2]
  return [
    matrix[0] * x + matrix[1] * y + matrix[2] * z,
    matrix[4] * x + matrix[5] * y + matrix[6] * z,
    matrix[8] * x + matrix[9] * y + matrix[10] * z,
  ]
}

function normalise(x: number, y: number, z: number): Vec3Tuple {
  const length = Math.hypot(x, y, z)
  return length > 0 && Number.isFinite(length) ? [x / length, y / length, z / length] : [0, 0, 0]
}

function nodeAabbNear(
  bvh: MeshBvh,
  node: number,
  origin: readonly [number, number, number],
  direction: readonly [number, number, number],
  minT: number,
  maxT: number,
): number | null {
  const offset = node * 6
  let near = minT
  let far = maxT
  for (let axis = 0; axis < 3; axis++) {
    const o = origin[axis]
    const d = direction[axis]
    const min = bvh.bounds[offset + axis]
    const max = bvh.bounds[offset + axis + 3]
    if (Math.abs(d) < Number.MIN_VALUE) {
      if (o < min || o > max) return null
      continue
    }
    let entry = (min - o) / d
    let exit = (max - o) / d
    if (entry > exit) [entry, exit] = [exit, entry]
    near = Math.max(near, entry)
    far = Math.min(far, exit)
    if (far < near) return null
  }
  return Number.isNaN(near) ? null : near
}

interface TriangleIntersection {
  t: number
  u: number
  v: number
  determinant: number
  normalX: number
  normalY: number
  normalZ: number
  ia: number
  ib: number
  ic: number
}

function intersectTriangle(
  vertices: Float32Array,
  indices: Uint32Array,
  vertexStride: number,
  triangle: number,
  origin: readonly [number, number, number],
  direction: readonly [number, number, number],
  minT: number,
  maxT: number,
): TriangleIntersection | null {
  const indexOffset = triangle * 3
  if (indexOffset + 2 >= indices.length) return null
  const ia = indices[indexOffset], ib = indices[indexOffset + 1], ic = indices[indexOffset + 2]
  const a = ia * vertexStride, b = ib * vertexStride, c = ic * vertexStride
  if (a + 2 >= vertices.length || b + 2 >= vertices.length || c + 2 >= vertices.length) return null

  const ax = vertices[a], ay = vertices[a + 1], az = vertices[a + 2]
  const e1x = vertices[b] - ax, e1y = vertices[b + 1] - ay, e1z = vertices[b + 2] - az
  const e2x = vertices[c] - ax, e2y = vertices[c + 1] - ay, e2z = vertices[c + 2] - az
  const px = direction[1] * e2z - direction[2] * e2y
  const py = direction[2] * e2x - direction[0] * e2z
  const pz = direction[0] * e2y - direction[1] * e2x
  const determinant = e1x * px + e1y * py + e1z * pz
  const e1Squared = e1x * e1x + e1y * e1y + e1z * e1z
  const e2Squared = e2x * e2x + e2y * e2y + e2z * e2z
  const directionSquared = direction[0] ** 2 + direction[1] ** 2 + direction[2] ** 2
  const determinantScale = Math.sqrt(e1Squared * e2Squared * directionSquared)
  if (!Number.isFinite(determinant) || Math.abs(determinant) <= Number.EPSILON * 64 * determinantScale) return null

  const inverseDeterminant = 1 / determinant
  const tx = origin[0] - ax, ty = origin[1] - ay, tz = origin[2] - az
  const u = (tx * px + ty * py + tz * pz) * inverseDeterminant
  if (u < -BARYCENTRIC_EPSILON || u > 1 + BARYCENTRIC_EPSILON) return null
  const qx = ty * e1z - tz * e1y
  const qy = tz * e1x - tx * e1z
  const qz = tx * e1y - ty * e1x
  const v = (direction[0] * qx + direction[1] * qy + direction[2] * qz) * inverseDeterminant
  if (v < -BARYCENTRIC_EPSILON || u + v > 1 + BARYCENTRIC_EPSILON) return null
  const t = (e2x * qx + e2y * qy + e2z * qz) * inverseDeterminant
  if (!Number.isFinite(t) || t < minT || t > maxT) return null

  return {
    t, u, v, determinant,
    normalX: e1y * e2z - e1z * e2y,
    normalY: e1z * e2x - e1x * e2z,
    normalZ: e1x * e2y - e1y * e2x,
    ia, ib, ic,
  }
}

/**
 * Find the nearest double-sided triangle intersection.
 *
 * When `localFromWorld` is supplied, the input ray is treated as world-space.
 * Both local and world hit points/normals are returned. Branches are traversed
 * front-to-back, allowing the current nearest hit to prune farther nodes.
 */
export function raycastMeshBvh(
  bvh: MeshBvh,
  vertices: Float32Array,
  indices: Uint32Array,
  ray: BvhRay,
  options: RaycastMeshBvhOptions = {},
): MeshBvhHit | null {
  const usableNodeCount = Math.min(
    Math.max(0, Math.trunc(bvh.nodeCount)),
    Math.floor(bvh.bounds.length / 6),
    Math.floor(bvh.nodes.length / 2),
  )
  if (usableNodeCount === 0 || bvh.triangles.length === 0 || bvh.vertexStride < 3) return null
  if (![...ray.origin, ...ray.direction].every(Number.isFinite)) return null
  if (ray.direction[0] === 0 && ray.direction[1] === 0 && ray.direction[2] === 0) return null

  const requestedMin = options.minT ?? 0
  const requestedMax = options.maxT ?? Infinity
  const minT = Number.isNaN(requestedMin) ? 0 : requestedMin
  let nearestT = Number.isNaN(requestedMax) ? Infinity : requestedMax
  if (nearestT < minT) return null

  const localOrigin = options.localFromWorld
    ? transformPoint(options.localFromWorld, ray.origin)
    : [...ray.origin] as Vec3Tuple
  const localDirection = options.localFromWorld
    ? transformVector(options.localFromWorld, ray.direction)
    : [...ray.direction] as Vec3Tuple
  if (![...localOrigin, ...localDirection].every(Number.isFinite)) return null
  if (localDirection[0] === 0 && localDirection[1] === 0 && localDirection[2] === 0) return null

  const rootNear = nodeAabbNear(bvh, 0, localOrigin, localDirection, minT, nearestT)
  if (rootNear === null) return null

  let stack = new Uint32Array(64)
  let stackSize = 1
  stack[0] = 0
  let nearestTriangle = 0xffffffff
  let nearestIntersection: TriangleIntersection | null = null

  const push = (node: number) => {
    if (stackSize === stack.length) {
      const grown = new Uint32Array(stack.length * 2)
      grown.set(stack)
      stack = grown
    }
    stack[stackSize++] = node
  }

  while (stackSize > 0) {
    const node = stack[--stackSize]
    if (node >= usableNodeCount) continue
    if (nodeAabbNear(bvh, node, localOrigin, localDirection, minT, nearestT) === null) continue
    const dataOffset = node * 2
    const firstOrLeft = bvh.nodes[dataOffset]
    const metadata = bvh.nodes[dataOffset + 1]

    if ((metadata & LEAF_BIT) !== 0) {
      const count = metadata & LEAF_COUNT_MASK
      const end = Math.min(firstOrLeft + count, bvh.triangles.length)
      for (let slot = firstOrLeft; slot < end; slot++) {
        const triangle = bvh.triangles[slot]
        if (options.excludedTriangles?.has(triangle)) continue
        const intersection = intersectTriangle(
          vertices, indices, bvh.vertexStride, triangle,
          localOrigin, localDirection, minT, nearestT,
        )
        if (!intersection) continue
        // Distance is authoritative. A scale-relative tolerance can let a
        // slightly farther triangle win at large ray parameters and then make
        // a continuation skip the true nearest layer. Only exact ties use the
        // original triangle id for deterministic ordering.
        if (!nearestIntersection || intersection.t < nearestT ||
            (intersection.t === nearestT && triangle < nearestTriangle)) {
          nearestT = intersection.t
          nearestTriangle = triangle
          nearestIntersection = intersection
        }
      }
      continue
    }

    const left = firstOrLeft
    const right = metadata
    const leftNear = left < usableNodeCount
      ? nodeAabbNear(bvh, left, localOrigin, localDirection, minT, nearestT)
      : null
    const rightNear = right < usableNodeCount
      ? nodeAabbNear(bvh, right, localOrigin, localDirection, minT, nearestT)
      : null
    // Stack is LIFO: push the farther child first.
    if (leftNear !== null && rightNear !== null) {
      if (leftNear <= rightNear) { push(right); push(left) }
      else { push(left); push(right) }
    } else if (leftNear !== null) push(left)
    else if (rightNear !== null) push(right)
  }

  if (!nearestIntersection) return null
  const hit = nearestIntersection
  let u = Math.min(1, Math.max(0, hit.u))
  let v = Math.min(1, Math.max(0, hit.v))
  let w = Math.min(1, Math.max(0, 1 - hit.u - hit.v))
  const barycentricSum = w + u + v
  if (barycentricSum > 0) {
    w /= barycentricSum; u /= barycentricSum; v /= barycentricSum
  }
  const localPoint: Vec3Tuple = [
    localOrigin[0] + localDirection[0] * hit.t,
    localOrigin[1] + localDirection[1] * hit.t,
    localOrigin[2] + localDirection[2] * hit.t,
  ]
  const worldPoint: Vec3Tuple = [
    ray.origin[0] + ray.direction[0] * hit.t,
    ray.origin[1] + ray.direction[1] * hit.t,
    ray.origin[2] + ray.direction[2] * hit.t,
  ]
  const localNormal = normalise(hit.normalX, hit.normalY, hit.normalZ)
  const worldNormal = options.localFromWorld
    // normal_world = transpose(localFromWorld.linear) * normal_local
    ? normalise(
        options.localFromWorld[0] * localNormal[0] + options.localFromWorld[4] * localNormal[1] + options.localFromWorld[8] * localNormal[2],
        options.localFromWorld[1] * localNormal[0] + options.localFromWorld[5] * localNormal[1] + options.localFromWorld[9] * localNormal[2],
        options.localFromWorld[2] * localNormal[0] + options.localFromWorld[6] * localNormal[1] + options.localFromWorld[10] * localNormal[2],
      )
    : [...localNormal] as Vec3Tuple

  return {
    triangleIndex: nearestTriangle,
    triangleVertexIndices: [hit.ia, hit.ib, hit.ic],
    t: hit.t,
    barycentric: [w, u, v],
    localPoint,
    worldPoint,
    localNormal,
    worldNormal,
    frontFace: hit.determinant > 0,
  }
}

/** Buffers that can be passed directly in a Worker `postMessage` transfer list. */
export function meshBvhTransferables(bvh: MeshBvh): ArrayBuffer[] {
  const buffers = [bvh.bounds.buffer, bvh.nodes.buffer, bvh.triangles.buffer]
  return [...new Set(buffers)] as ArrayBuffer[]
}
