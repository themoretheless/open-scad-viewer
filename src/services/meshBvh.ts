/**
 * A compact, transferable triangle BVH for CPU picking.
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

export type Vec3Tuple = [number, number, number]

export interface MeshBvh {
  readonly version: 1
  readonly vertexStride: number
  readonly leafSize: number
  readonly nodeCount: number
  readonly bounds: Float32Array
  readonly nodes: Uint32Array
  readonly triangles: Uint32Array
}

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

function nextPowerOfTwo(value: number): number {
  if (value <= 1) return 1
  return 2 ** Math.ceil(Math.log2(value))
}

/**
 * Build a deterministic, balanced median-split BVH.
 *
 * Construction is O(n log n), uses bounded recursion (about 18 levels for the
 * application's 750k-triangle limit), and does not mutate its inputs.
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

  // These arrays are construction-only. Records are compacted at the front so
  // malformed triangles do not consume space in the final BVH.
  const sourceTriangles = new Uint32Array(triangleCount)
  const triangleBounds = new Float32Array(triangleCount * 6)
  const centroids = new Float32Array(triangleCount * 3)
  let validCount = 0

  for (let triangle = 0; triangle < triangleCount; triangle++) {
    const indexOffset = triangle * 3
    const ia = indices[indexOffset]
    const ib = indices[indexOffset + 1]
    const ic = indices[indexOffset + 2]
    if (ia >= vertexCount || ib >= vertexCount || ic >= vertexCount) continue

    const a = ia * vertexStride
    const b = ib * vertexStride
    const c = ic * vertexStride
    const ax = vertices[a], ay = vertices[a + 1], az = vertices[a + 2]
    const bx = vertices[b], by = vertices[b + 1], bz = vertices[b + 2]
    const cx = vertices[c], cy = vertices[c + 1], cz = vertices[c + 2]
    if (!Number.isFinite(ax) || !Number.isFinite(ay) || !Number.isFinite(az) ||
        !Number.isFinite(bx) || !Number.isFinite(by) || !Number.isFinite(bz) ||
        !Number.isFinite(cx) || !Number.isFinite(cy) || !Number.isFinite(cz)) continue

    const e1x = bx - ax, e1y = by - ay, e1z = bz - az
    const e2x = cx - ax, e2y = cy - ay, e2z = cz - az
    const nx = e1y * e2z - e1z * e2y
    const ny = e1z * e2x - e1x * e2z
    const nz = e1x * e2y - e1y * e2x
    const areaSquared = nx * nx + ny * ny + nz * nz
    if (!(areaSquared > 0) || !Number.isFinite(areaSquared)) continue

    const boundsOffset = validCount * 6
    const minX = Math.min(ax, bx, cx), minY = Math.min(ay, by, cy), minZ = Math.min(az, bz, cz)
    const maxX = Math.max(ax, bx, cx), maxY = Math.max(ay, by, cy), maxZ = Math.max(az, bz, cz)
    triangleBounds[boundsOffset] = minX
    triangleBounds[boundsOffset + 1] = minY
    triangleBounds[boundsOffset + 2] = minZ
    triangleBounds[boundsOffset + 3] = maxX
    triangleBounds[boundsOffset + 4] = maxY
    triangleBounds[boundsOffset + 5] = maxZ
    const centroidOffset = validCount * 3
    // min + half-extent avoids overflowing where (min + max) / 2 would.
    centroids[centroidOffset] = minX + (maxX - minX) * 0.5
    centroids[centroidOffset + 1] = minY + (maxY - minY) * 0.5
    centroids[centroidOffset + 2] = minZ + (maxZ - minZ) * 0.5
    sourceTriangles[validCount] = triangle
    validCount++
  }

  if (validCount === 0) return emptyBvh(vertexStride, leafSize)

  // Balanced median splits produce no more than the next power-of-two number
  // of leaves. This is < 4 * ceil(validCount / leafSize) nodes and prevents a
  // wasteful 2*n allocation for large meshes.
  const maximumLeaves = nextPowerOfTwo(Math.ceil(validCount / leafSize))
  const maximumNodes = maximumLeaves * 2 - 1
  const temporaryBounds = new Float32Array(maximumNodes * 6)
  const temporaryNodes = new Uint32Array(maximumNodes * 2)
  const order = new Uint32Array(validCount)
  for (let i = 0; i < validCount; i++) order[i] = i

  let nodeCount = 0

  const compareRecords = (left: number, right: number, axis: number): number => {
    const difference = centroids[left * 3 + axis] - centroids[right * 3 + axis]
    if (difference !== 0) return difference
    // Original triangle number is a stable, deterministic tiebreaker.
    return sourceTriangles[left] - sourceTriangles[right]
  }

  const swap = (a: number, b: number) => {
    const value = order[a]
    order[a] = order[b]
    order[b] = value
  }

  /** In-place deterministic quickselect with a three-way partition. */
  const selectNth = (start: number, end: number, nth: number, axis: number) => {
    let low = start
    let high = end
    while (high - low > 1) {
      const middle = low + ((high - low) >>> 1)
      const lowRecord = order[low]
      const middleRecord = order[middle]
      const highRecord = order[high - 1]
      // Allocation-free median-of-three pivot selection.
      let pivot = lowRecord
      if (compareRecords(lowRecord, middleRecord, axis) < 0) {
        pivot = compareRecords(middleRecord, highRecord, axis) < 0
          ? middleRecord
          : (compareRecords(lowRecord, highRecord, axis) < 0 ? highRecord : lowRecord)
      } else {
        pivot = compareRecords(lowRecord, highRecord, axis) < 0
          ? lowRecord
          : (compareRecords(middleRecord, highRecord, axis) < 0 ? highRecord : middleRecord)
      }

      let before = low
      let cursor = low
      let after = high
      while (cursor < after) {
        const comparison = compareRecords(order[cursor], pivot, axis)
        if (comparison < 0) {
          swap(before++, cursor++)
        } else if (comparison > 0) {
          swap(cursor, --after)
        } else {
          cursor++
        }
      }
      if (nth < before) high = before
      else if (nth >= after) low = after
      else return
    }
  }

  const buildNode = (start: number, end: number): number => {
    const node = nodeCount++
    const nodeBoundsOffset = node * 6
    let minX = Infinity, minY = Infinity, minZ = Infinity
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
    let centroidMinX = Infinity, centroidMinY = Infinity, centroidMinZ = Infinity
    let centroidMaxX = -Infinity, centroidMaxY = -Infinity, centroidMaxZ = -Infinity

    for (let slot = start; slot < end; slot++) {
      const record = order[slot]
      const boundsOffset = record * 6
      minX = Math.min(minX, triangleBounds[boundsOffset])
      minY = Math.min(minY, triangleBounds[boundsOffset + 1])
      minZ = Math.min(minZ, triangleBounds[boundsOffset + 2])
      maxX = Math.max(maxX, triangleBounds[boundsOffset + 3])
      maxY = Math.max(maxY, triangleBounds[boundsOffset + 4])
      maxZ = Math.max(maxZ, triangleBounds[boundsOffset + 5])
      const centroidOffset = record * 3
      const x = centroids[centroidOffset], y = centroids[centroidOffset + 1], z = centroids[centroidOffset + 2]
      centroidMinX = Math.min(centroidMinX, x); centroidMaxX = Math.max(centroidMaxX, x)
      centroidMinY = Math.min(centroidMinY, y); centroidMaxY = Math.max(centroidMaxY, y)
      centroidMinZ = Math.min(centroidMinZ, z); centroidMaxZ = Math.max(centroidMaxZ, z)
    }

    temporaryBounds[nodeBoundsOffset] = minX
    temporaryBounds[nodeBoundsOffset + 1] = minY
    temporaryBounds[nodeBoundsOffset + 2] = minZ
    temporaryBounds[nodeBoundsOffset + 3] = maxX
    temporaryBounds[nodeBoundsOffset + 4] = maxY
    temporaryBounds[nodeBoundsOffset + 5] = maxZ

    const count = end - start
    const dataOffset = node * 2
    if (count <= leafSize) {
      temporaryNodes[dataOffset] = start
      temporaryNodes[dataOffset + 1] = (LEAF_BIT | count) >>> 0
      return node
    }

    const extentX = centroidMaxX - centroidMinX
    const extentY = centroidMaxY - centroidMinY
    const extentZ = centroidMaxZ - centroidMinZ
    // Stable tie order is X, then Y, then Z.
    let axis = 0
    if (extentY > extentX) axis = 1
    if (extentZ > (axis === 0 ? extentX : extentY)) axis = 2
    const middle = start + (count >>> 1)
    selectNth(start, end, middle, axis)
    const left = buildNode(start, middle)
    const right = buildNode(middle, end)
    temporaryNodes[dataOffset] = left
    temporaryNodes[dataOffset + 1] = right
    return node
  }

  buildNode(0, validCount)

  const triangles = new Uint32Array(validCount)
  for (let slot = 0; slot < validCount; slot++) triangles[slot] = sourceTriangles[order[slot]]

  return {
    version: 1,
    vertexStride,
    leafSize,
    nodeCount,
    bounds: temporaryBounds.slice(0, nodeCount * 6),
    nodes: temporaryNodes.slice(0, nodeCount * 2),
    triangles,
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
