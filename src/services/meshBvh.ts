/**
 * A compact, transferable triangle BVH for CPU picking.
 *
 * Construction runs in the Rust geometry kernel (crates/polygon-core/src/solid/bvh.rs)
 * through the raw-buffer binding in services/geometry/meshAnalysis.ts; this
 * module keeps option clamping, the empty-mesh fast path and the numeric
 * transport contract. Interactive raycasting uses nativePickingCache and
 * the Rust-owned BVH query; this module contains no ray intersection algorithm.
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
  /** Point in the coordinate system of the supplied ray. */
  readonly worldPoint: Vec3Tuple
  readonly localNormal: Vec3Tuple
  /** Normal transformed back to the ray's coordinate system. */
  readonly worldNormal: Vec3Tuple
  readonly frontFace: boolean
}

const DEFAULT_VERTEX_STRIDE = 6
const DEFAULT_LEAF_SIZE = 8
const MAX_LEAF_SIZE = 64

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

/** Buffers that can be passed directly in a Worker `postMessage` transfer list. */
export function meshBvhTransferables(bvh: MeshBvh): ArrayBuffer[] {
  const buffers = [bvh.bounds.buffer, bvh.nodes.buffer, bvh.triangles.buffer]
  return [...new Set(buffers)] as ArrayBuffer[]
}
