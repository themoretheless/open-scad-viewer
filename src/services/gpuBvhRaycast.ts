/**
 * WebGPU Compute Batched BVH Raycaster & Multi-Ray Picking Accelerator.
 *
 * Evaluates N rays in parallel against a flat linear BVH node array on GPU:
 * 1. Builds a compact 32-byte-per-node linear BVH from scene AABB items.
 * 2. Dispatches 1 GPU compute thread per ray with stack-based ordered child traversal.
 * 3. Returns closest hit [id, tNear] per ray, with deterministic CPU fallback.
 */

import { runGpuCompute } from './webgpuCompute'
import type { Ray3 } from './math3d'

export interface AabbItem {
  id: string
  min: [number, number, number]
  max: [number, number, number]
}

export interface RayHit {
  id: string
  tNear: number
}


export interface FlatBvhTree {
  /** 32-byte per node buffer (interleaved f32 bounds and u32 child/object indices) */
  nodeBuffer: ArrayBuffer
  /** Number of BVH nodes */
  nodeCount: number
  /** Index of root node (0 when non-empty) */
  rootIndex: number
  /** Mapping from integer leaf index -> original string item id */
  idTable: string[]
}

export interface BatchedRayInput {
  origin: [number, number, number]
  direction: [number, number, number]
  tMin?: number
  tMax?: number
}

export const BVH_RAYCAST_WGSL = /* wgsl */`
struct RaycastParams {
  num_rays: u32,
  num_nodes: u32,
  root_index: u32,
  _pad0: u32,
};

struct BvhNode {
  min_x: f32,
  min_y: f32,
  min_z: f32,
  left_or_obj: u32,
  max_x: f32,
  max_y: f32,
  max_z: f32,
  right_or_leaf: u32,
};

struct RayInput {
  ox: f32,
  oy: f32,
  oz: f32,
  t_min: f32,
  dx: f32,
  dy: f32,
  dz: f32,
  t_max: f32,
};

struct RayHitOutput {
  object_id: u32,
  t_hit: f32,
};

@group(0) @binding(0) var<uniform> params: RaycastParams;
@group(0) @binding(1) var<storage, read> nodes: array<BvhNode>;
@group(0) @binding(2) var<storage, read> rays: array<RayInput>;
@group(0) @binding(3) var<storage, read_write> hits: array<RayHitOutput>;

fn slab_test(
  node: BvhNode,
  ox: f32, oy: f32, oz: f32,
  dx: f32, dy: f32, dz: f32,
  t_min_in: f32,
  t_max_in: f32
) -> f32 {
  var t_near = t_min_in;
  var t_far = t_max_in;

  if (abs(dx) < 1e-12) {
    if (ox < node.min_x || ox > node.max_x) { return -1.0; }
  } else {
    let inv_x = 1.0 / dx;
    let t0 = (node.min_x - ox) * inv_x;
    let t1 = (node.max_x - ox) * inv_x;
    t_near = max(t_near, min(t0, t1));
    t_far = min(t_far, max(t0, t1));
    if (t_near > t_far) { return -1.0; }
  }

  if (abs(dy) < 1e-12) {
    if (oy < node.min_y || oy > node.max_y) { return -1.0; }
  } else {
    let inv_y = 1.0 / dy;
    let t0 = (node.min_y - oy) * inv_y;
    let t1 = (node.max_y - oy) * inv_y;
    t_near = max(t_near, min(t0, t1));
    t_far = min(t_far, max(t0, t1));
    if (t_near > t_far) { return -1.0; }
  }

  if (abs(dz) < 1e-12) {
    if (oz < node.min_z || oz > node.max_z) { return -1.0; }
  } else {
    let inv_z = 1.0 / dz;
    let t0 = (node.min_z - oz) * inv_z;
    let t1 = (node.max_z - oz) * inv_z;
    t_near = max(t_near, min(t0, t1));
    t_far = min(t_far, max(t0, t1));
    if (t_near > t_far) { return -1.0; }
  }

  return t_near;
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let ray_idx = id.x;
  if (ray_idx >= params.num_rays) {
    return;
  }

  if (params.num_nodes == 0u) {
    hits[ray_idx] = RayHitOutput(0xFFFFFFFFu, 1e30);
    return;
  }

  let ray = rays[ray_idx];
  let len_sq = ray.dx * ray.dx + ray.dy * ray.dy + ray.dz * ray.dz;
  if (len_sq <= 0.0) {
    hits[ray_idx] = RayHitOutput(0xFFFFFFFFu, 1e30);
    return;
  }

  var best_t = ray.t_max;
  var best_obj = 0xFFFFFFFFu;

  var stack: array<u32, 32>;
  var stack_ptr = 0u;
  stack[0] = params.root_index;
  stack_ptr = 1u;

  while (stack_ptr > 0u) {
    stack_ptr -= 1u;
    let node_idx = stack[stack_ptr];
    if (node_idx >= params.num_nodes) {
      continue;
    }

    let node = nodes[node_idx];
    let t_entry = slab_test(
      node,
      ray.ox, ray.oy, ray.oz,
      ray.dx, ray.dy, ray.dz,
      max(0.0, ray.t_min),
      best_t
    );

    if (t_entry < 0.0 || t_entry > best_t) {
      continue;
    }

    if (node.right_or_leaf == 0xFFFFFFFFu) {
      let obj_id = node.left_or_obj;
      if (t_entry < best_t || (t_entry == best_t && obj_id < best_obj)) {
        best_t = t_entry;
        best_obj = obj_id;
      }
    } else {
      let left_idx = node.left_or_obj;
      let right_idx = node.right_or_leaf;

      let left_node = nodes[left_idx];
      let right_node = nodes[right_idx];

      let t_left = slab_test(
        left_node,
        ray.ox, ray.oy, ray.oz,
        ray.dx, ray.dy, ray.dz,
        max(0.0, ray.t_min),
        best_t
      );
      let t_right = slab_test(
        right_node,
        ray.ox, ray.oy, ray.oz,
        ray.dx, ray.dy, ray.dz,
        max(0.0, ray.t_min),
        best_t
      );

      let hit_left = t_left >= 0.0 && t_left <= best_t;
      let hit_right = t_right >= 0.0 && t_right <= best_t;

      if (hit_left && hit_right) {
        if (t_left <= t_right) {
          if (stack_ptr + 2u <= 32u) {
            stack[stack_ptr] = right_idx;
            stack[stack_ptr + 1u] = left_idx;
            stack_ptr += 2u;
          }
        } else {
          if (stack_ptr + 2u <= 32u) {
            stack[stack_ptr] = left_idx;
            stack[stack_ptr + 1u] = right_idx;
            stack_ptr += 2u;
          }
        }
      } else if (hit_left) {
        if (stack_ptr + 1u <= 32u) {
          stack[stack_ptr] = left_idx;
          stack_ptr += 1u;
        }
      } else if (hit_right) {
        if (stack_ptr + 1u <= 32u) {
          stack[stack_ptr] = right_idx;
          stack_ptr += 1u;
        }
      }
    }
  }

  hits[ray_idx] = RayHitOutput(best_obj, best_t);
}
`

interface IndexedLeaf {
  objIndex: number
  id: string
  min: [number, number, number]
  max: [number, number, number]
  center: [number, number, number]
}

interface TempNode {
  min: [number, number, number]
  max: [number, number, number]
  leftOrObj: number
  rightOrLeaf: number
}

/**
 * Builds a flat 32-byte-per-node linear BVH from scene AABB items.
 */
export function buildFlatBvh(items: readonly AabbItem[]): FlatBvhTree {
  if (items.length === 0) {
    return {
      nodeBuffer: new ArrayBuffer(32),
      nodeCount: 0,
      rootIndex: 0,
      idTable: [],
    }
  }

  // Sort items by id so lower objIndex corresponds to lexicographically smaller id for tie-breaking
  const sorted = [...items].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
  const idTable = sorted.map((it) => it.id)

  const leaves: IndexedLeaf[] = sorted.map((it, idx) => ({
    objIndex: idx,
    id: it.id,
    min: [it.min[0], it.min[1], it.min[2]],
    max: [it.max[0], it.max[1], it.max[2]],
    center: [
      0.5 * (it.min[0] + it.max[0]),
      0.5 * (it.min[1] + it.max[1]),
      0.5 * (it.min[2] + it.max[2]),
    ],
  }))

  const nodes: TempNode[] = []

  function buildRange(list: IndexedLeaf[]): number {
    const nodeIdx = nodes.length
    nodes.push({
      min: [0, 0, 0],
      max: [0, 0, 0],
      leftOrObj: 0,
      rightOrLeaf: 0xFFFFFFFF,
    })

    if (list.length === 1) {
      const leaf = list[0]!
      nodes[nodeIdx] = {
        min: leaf.min,
        max: leaf.max,
        leftOrObj: leaf.objIndex,
        rightOrLeaf: 0xFFFFFFFF,
      }
      return nodeIdx
    }

    let cMinX = Infinity, cMinY = Infinity, cMinZ = Infinity
    let cMaxX = -Infinity, cMaxY = -Infinity, cMaxZ = -Infinity
    let bMinX = Infinity, bMinY = Infinity, bMinZ = Infinity
    let bMaxX = -Infinity, bMaxY = -Infinity, bMaxZ = -Infinity

    for (let i = 0; i < list.length; i++) {
      const it = list[i]!
      if (it.center[0] < cMinX) cMinX = it.center[0]
      if (it.center[1] < cMinY) cMinY = it.center[1]
      if (it.center[2] < cMinZ) cMinZ = it.center[2]
      if (it.center[0] > cMaxX) cMaxX = it.center[0]
      if (it.center[1] > cMaxY) cMaxY = it.center[1]
      if (it.center[2] > cMaxZ) cMaxZ = it.center[2]

      if (it.min[0] < bMinX) bMinX = it.min[0]
      if (it.min[1] < bMinY) bMinY = it.min[1]
      if (it.min[2] < bMinZ) bMinZ = it.min[2]
      if (it.max[0] > bMaxX) bMaxX = it.max[0]
      if (it.max[1] > bMaxY) bMaxY = it.max[1]
      if (it.max[2] > bMaxZ) bMaxZ = it.max[2]
    }

    const sx = cMaxX - cMinX
    const sy = cMaxY - cMinY
    const sz = cMaxZ - cMinZ
    const axis: 0 | 1 | 2 = sx >= sy && sx >= sz ? 0 : sy >= sz ? 1 : 2

    list.sort((a, b) => {
      const d = a.center[axis] - b.center[axis]
      return d !== 0 ? d : a.objIndex - b.objIndex
    })

    const mid = list.length >> 1
    const leftIdx = buildRange(list.slice(0, mid))
    const rightIdx = buildRange(list.slice(mid))

    nodes[nodeIdx] = {
      min: [bMinX, bMinY, bMinZ],
      max: [bMaxX, bMaxY, bMaxZ],
      leftOrObj: leftIdx,
      rightOrLeaf: rightIdx,
    }
    return nodeIdx
  }

  const rootIndex = buildRange(leaves)
  const nodeBuffer = new ArrayBuffer(Math.max(1, nodes.length) * 32)
  const view = new DataView(nodeBuffer)

  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i]!
    const off = i * 32
    view.setFloat32(off + 0, n.min[0], true)
    view.setFloat32(off + 4, n.min[1], true)
    view.setFloat32(off + 8, n.min[2], true)
    view.setUint32(off + 12, n.leftOrObj >>> 0, true)
    view.setFloat32(off + 16, n.max[0], true)
    view.setFloat32(off + 20, n.max[1], true)
    view.setFloat32(off + 24, n.max[2], true)
    view.setUint32(off + 28, n.rightOrLeaf >>> 0, true)
  }

  return {
    nodeBuffer,
    nodeCount: nodes.length,
    rootIndex,
    idTable,
  }
}

/**
 * Exact CPU reference implementation of batched BVH raycast over FlatBvhTree.
 */
export function referenceBatchedBvhRaycast(
  bvh: FlatBvhTree,
  rays: readonly BatchedRayInput[]
): (RayHit | null)[] {
  if (bvh.nodeCount === 0 || rays.length === 0) {
    return rays.map(() => null)
  }

  const view = new DataView(bvh.nodeBuffer)

  function readNode(idx: number) {
    const off = idx * 32
    return {
      minX: view.getFloat32(off + 0, true),
      minY: view.getFloat32(off + 4, true),
      minZ: view.getFloat32(off + 8, true),
      leftOrObj: view.getUint32(off + 12, true),
      maxX: view.getFloat32(off + 16, true),
      maxY: view.getFloat32(off + 20, true),
      maxZ: view.getFloat32(off + 24, true),
      rightOrLeaf: view.getUint32(off + 28, true),
    }
  }

  function slabTest(
    n: ReturnType<typeof readNode>,
    ox: number, oy: number, oz: number,
    dx: number, dy: number, dz: number,
    tMinIn: number,
    tMaxIn: number
  ): number {
    let tNear = tMinIn
    let tFar = tMaxIn

    if (Math.abs(dx) < 1e-12) {
      if (ox < n.minX || ox > n.maxX) return -1
    } else {
      const invX = 1 / dx
      const t0 = (n.minX - ox) * invX
      const t1 = (n.maxX - ox) * invX
      tNear = Math.max(tNear, Math.min(t0, t1))
      tFar = Math.min(tFar, Math.max(t0, t1))
      if (tNear > tFar) return -1
    }

    if (Math.abs(dy) < 1e-12) {
      if (oy < n.minY || oy > n.maxY) return -1
    } else {
      const invY = 1 / dy
      const t0 = (n.minY - oy) * invY
      const t1 = (n.maxY - oy) * invY
      tNear = Math.max(tNear, Math.min(t0, t1))
      tFar = Math.min(tFar, Math.max(t0, t1))
      if (tNear > tFar) return -1
    }

    if (Math.abs(dz) < 1e-12) {
      if (oz < n.minZ || oz > n.maxZ) return -1
    } else {
      const invZ = 1 / dz
      const t0 = (n.minZ - oz) * invZ
      const t1 = (n.maxZ - oz) * invZ
      tNear = Math.max(tNear, Math.min(t0, t1))
      tFar = Math.min(tFar, Math.max(t0, t1))
      if (tNear > tFar) return -1
    }

    return tNear
  }

  const results: (RayHit | null)[] = []
  const stack = new Uint32Array(64)

  for (let r = 0; r < rays.length; r++) {
    const ray = rays[r]!
    const [ox, oy, oz] = ray.origin
    const [dx, dy, dz] = ray.direction
    const lenSq = dx * dx + dy * dy + dz * dz
    if (!(lenSq > 0)) {
      results.push(null)
      continue
    }

    const tMin = Math.max(0, ray.tMin ?? 0)
    let bestT = ray.tMax ?? 1e30
    let bestObj = 0xFFFFFFFF

    stack[0] = bvh.rootIndex
    let stackPtr = 1

    while (stackPtr > 0) {
      stackPtr--
      const nodeIdx = stack[stackPtr]!
      if (nodeIdx >= bvh.nodeCount) continue

      const node = readNode(nodeIdx)
      const tEntry = slabTest(node, ox, oy, oz, dx, dy, dz, tMin, bestT)
      if (tEntry < 0 || tEntry > bestT) continue

      if (node.rightOrLeaf === 0xFFFFFFFF) {
        const objId = node.leftOrObj
        if (tEntry < bestT || (tEntry === bestT && objId < bestObj)) {
          bestT = tEntry
          bestObj = objId
        }
      } else {
        const leftIdx = node.leftOrObj
        const rightIdx = node.rightOrLeaf
        const leftNode = readNode(leftIdx)
        const rightNode = readNode(rightIdx)

        const tLeft = slabTest(leftNode, ox, oy, oz, dx, dy, dz, tMin, bestT)
        const tRight = slabTest(rightNode, ox, oy, oz, dx, dy, dz, tMin, bestT)
        const hitLeft = tLeft >= 0 && tLeft <= bestT
        const hitRight = tRight >= 0 && tRight <= bestT

        if (hitLeft && hitRight) {
          if (tLeft <= tRight) {
            stack[stackPtr++] = rightIdx
            stack[stackPtr++] = leftIdx
          } else {
            stack[stackPtr++] = leftIdx
            stack[stackPtr++] = rightIdx
          }
        } else if (hitLeft) {
          stack[stackPtr++] = leftIdx
        } else if (hitRight) {
          stack[stackPtr++] = rightIdx
        }
      }
    }

    if (bestObj !== 0xFFFFFFFF && bestObj < bvh.idTable.length) {
      results.push({ id: bvh.idTable[bestObj]!, tNear: bestT })
    } else {
      results.push(null)
    }
  }

  return results
}

/**
 * Evaluates N rays in parallel on WebGPU Compute against a FlatBvhTree.
 * Falls back to deterministic CPU traversal when WebGPU is unavailable.
 */
export async function runGpuBatchedBvhRaycast(
  bvh: FlatBvhTree,
  rays: readonly BatchedRayInput[]
): Promise<(RayHit | null)[]> {
  if (rays.length === 0) return []
  if (bvh.nodeCount === 0) return rays.map(() => null)

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return referenceBatchedBvhRaycast(bvh, rays)
  }

  try {
    const uniformBuf = new ArrayBuffer(16)
    const uView = new DataView(uniformBuf)
    uView.setUint32(0, rays.length, true)
    uView.setUint32(4, bvh.nodeCount, true)
    uView.setUint32(8, bvh.rootIndex, true)
    uView.setUint32(12, 0, true)

    const rayData = new Float32Array(rays.length * 8)
    for (let i = 0; i < rays.length; i++) {
      const r = rays[i]!
      const off = i * 8
      rayData[off + 0] = r.origin[0]
      rayData[off + 1] = r.origin[1]
      rayData[off + 2] = r.origin[2]
      rayData[off + 3] = r.tMin ?? 0
      rayData[off + 4] = r.direction[0]
      rayData[off + 5] = r.direction[1]
      rayData[off + 6] = r.direction[2]
      rayData[off + 7] = r.tMax ?? 1e30
    }

    const outBytes = rays.length * 8
    const outInit = new Uint32Array(rays.length * 2)

    const results = await runGpuCompute({
      wgsl: BVH_RAYCAST_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformBuf, uniform: true },
            { binding: 1, data: new Uint32Array(bvh.nodeBuffer) },
            { binding: 2, data: rayData },
            { binding: 3, data: outInit, output: true, outputBytes: outBytes },
          ],
          outputBytes: outBytes,
          workgroups: [Math.ceil(rays.length / 256), 1, 1],
        },
      ],
    })

    const rawOut = results[0]
    if (rawOut && rawOut.byteLength >= outBytes) {
      const outView = new DataView(rawOut.buffer, rawOut.byteOffset, rawOut.byteLength)
      const hits: (RayHit | null)[] = []
      for (let i = 0; i < rays.length; i++) {
        const off = i * 8
        const objId = outView.getUint32(off + 0, true)
        const tHit = outView.getFloat32(off + 4, true)
        if (objId !== 0xFFFFFFFF && objId < bvh.idTable.length) {
          hits.push({ id: bvh.idTable[objId]!, tNear: tHit })
        } else {
          hits.push(null)
        }
      }
      return hits
    }
  } catch {
    // Fallback to CPU reference when WebGPU adapter is unavailable
  }

  return referenceBatchedBvhRaycast(bvh, rays)
}

/**
 * Generates a 2D grid of frustum / selection rays for marquee box or area picking.
 */
export function buildMarqueeRayBatch(
  origin: Ray3['origin'],
  baseDirection: Ray3['direction'],
  rightVec: [number, number, number],
  upVec: [number, number, number],
  gridWidth: number,
  gridHeight: number,
  halfSpanX = 0.25,
  halfSpanY = 0.25
): BatchedRayInput[] {
  const rays: BatchedRayInput[] = []
  const w = Math.max(1, gridWidth)
  const h = Math.max(1, gridHeight)

  for (let iy = 0; iy < h; iy++) {
    const v = h === 1 ? 0 : ((iy / (h - 1)) * 2 - 1) * halfSpanY
    for (let ix = 0; ix < w; ix++) {
      const u = w === 1 ? 0 : ((ix / (w - 1)) * 2 - 1) * halfSpanX
      const dx = baseDirection[0] + u * rightVec[0] + v * upVec[0]
      const dy = baseDirection[1] + u * rightVec[1] + v * upVec[1]
      const dz = baseDirection[2] + u * rightVec[2] + v * upVec[2]
      rays.push({
        origin: [origin[0], origin[1], origin[2]],
        direction: [dx, dy, dz],
      })
    }
  }

  return rays
}
