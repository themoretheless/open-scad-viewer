import {GeometryKernelError,decodeNurbsResult,kernelRuntime,kernelVertexFormat} from './kernel'
import {copyBuffer} from './bufferTransport'

export interface KernelManifoldReport {
  readonly vertexCount: number
  readonly triangleCount: number
  readonly componentCount: number
  readonly isManifold: boolean
  readonly isManifoldWithBoundary: boolean
  /** Flat [a, b] vertex pairs per edge. */
  readonly boundaryEdges: Uint32Array<ArrayBuffer>
  readonly nonManifoldEdges: Uint32Array<ArrayBuffer>
  readonly orientationEdges: Uint32Array<ArrayBuffer>
  readonly degenerateTriangles: Uint32Array<ArrayBuffer>
  readonly nonManifoldVertices: Uint32Array<ArrayBuffer>
  readonly isolatedVertices: Uint32Array<ArrayBuffer>
}

/** Repair result from the manifold-core kernel (abi_manifold_repair). */
export interface KernelManifoldRepair {
  readonly positions: Float64Array<ArrayBuffer>
  readonly indices: Uint32Array<ArrayBuffer>
  readonly weldedVertices: number
  readonly removedDegenerateTriangles: number
  readonly flippedTriangles: number
  /** Full mode: vertices duplicated when splitting non-manifold edges/fans. */
  readonly splitVertices: number
  /** Full mode: boundary loops triangulated. */
  readonly filledHoles: number
  /** Full mode: triangles added by hole filling. */
  readonly filledTriangles: number
  /** 0 = Conservative, 1 = Full. */
  readonly mode: number
  readonly isManifold: boolean
  readonly isManifoldWithBoundary: boolean
}

/** Run the manifold-core manifoldness check on an uploaded mesh. */
export function checkManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
): KernelManifoldReport {
  const {exports: wasm, memory, takeResponse} = kernelRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_check(vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices))),
    )
    // Allocation above may grow memory. Never cache memory.buffer between calls.
    const buffer = memory.buffer
    const u32 = (slot: number) =>
      new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, slot), wasm.abi_array_field(handle, slot + 1)))
    const flags = wasm.abi_array_field(handle, 15)
    return {
      boundaryEdges: u32(0),
      nonManifoldEdges: u32(2),
      orientationEdges: u32(4),
      degenerateTriangles: u32(6),
      nonManifoldVertices: u32(8),
      isolatedVertices: u32(10),
      vertexCount: wasm.abi_array_field(handle, 12),
      triangleCount: wasm.abi_array_field(handle, 13),
      componentCount: wasm.abi_array_field(handle, 14),
      isManifold: (flags & 1) !== 0,
      isManifoldWithBoundary: (flags & 2) !== 0,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Repair a mesh toward manifoldness in the kernel. `full` additionally splits
 * non-manifold edges/vertices and fills simple boundary loops. */
export function repairManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
  epsilon: number,
  full = false,
): KernelManifoldRepair {
  const {exports: wasm, memory, takeResponse} = kernelRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_repair(vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices), epsilon, full ? 1 : 0)),
    )
    const buffer = memory.buffer
    const flags = wasm.abi_array_field(handle, 7)
    return {
      positions: new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1))),
      indices: new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3))),
      weldedVertices: wasm.abi_array_field(handle, 4),
      removedDegenerateTriangles: wasm.abi_array_field(handle, 5),
      flippedTriangles: wasm.abi_array_field(handle, 6),
      isManifold: (flags & 1) !== 0,
      isManifoldWithBoundary: (flags & 2) !== 0,
      splitVertices: wasm.abi_array_field(handle, 8),
      filledHoles: wasm.abi_array_field(handle, 9),
      filledTriangles: wasm.abi_array_field(handle, 10),
      mode: wasm.abi_array_field(handle, 11),
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Per-component metrics from the manifold-core kernel (abi_manifold_metrics). */
export interface KernelManifoldComponentMetrics {
  readonly triangleCount: number
  readonly vertexCount: number
  readonly edgeCount: number
  readonly boundaryEdges: number
  readonly eulerCharacteristic: number
  /** Genus for closed components; `null` when undefined (open/non-manifold). */
  readonly genus: number | null
  readonly signedVolume: number
  readonly surfaceArea: number
}

/** Whole-mesh metrics from the manifold-core kernel (abi_manifold_metrics). */
export interface KernelManifoldMetrics {
  readonly vertexCount: number
  readonly triangleCount: number
  readonly edgeCount: number
  readonly eulerCharacteristic: number
  readonly componentCount: number
  readonly signedVolume: number
  readonly surfaceArea: number
  readonly watertight: boolean
  readonly components: KernelManifoldComponentMetrics[]
}

/** Compute geometric and topological metrics of a mesh in the kernel. */
export function metricsManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
): KernelManifoldMetrics {
  const {exports: wasm, memory, takeResponse} = kernelRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_metrics(vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices))),
    )
    const buffer = memory.buffer
    const stats = new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1)))
    const componentFloats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3)))
    const floats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 4), wasm.abi_array_field(handle, 5)))
    const components: KernelManifoldComponentMetrics[] = []
    for (let c = 0; c < stats.length / 6; c++) {
      const genus = stats[c * 6 + 5]
      components.push({
        triangleCount: stats[c * 6],
        vertexCount: stats[c * 6 + 1],
        edgeCount: stats[c * 6 + 2],
        boundaryEdges: stats[c * 6 + 3],
        eulerCharacteristic: stats[c * 6 + 4] | 0,
        genus: genus === 0 ? null : genus - 1,
        signedVolume: componentFloats[c * 2],
        surfaceArea: componentFloats[c * 2 + 1],
      })
    }
    return {
      vertexCount: wasm.abi_array_field(handle, 6),
      triangleCount: wasm.abi_array_field(handle, 7),
      edgeCount: wasm.abi_array_field(handle, 8),
      eulerCharacteristic: wasm.abi_array_field(handle, 9) | 0,
      componentCount: wasm.abi_array_field(handle, 10),
      signedVolume: floats[0],
      surfaceArea: floats[1],
      watertight: (wasm.abi_array_field(handle, 11) & 1) !== 0,
      components,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Boolean operation kinds for the manifold-ops kernel (abi_manifold_boolean). */
export type ManifoldBooleanOp = 'union' | 'intersection' | 'difference'

const MANIFOLD_BOOLEAN_OP_TAG: Record<ManifoldBooleanOp, number> = {union: 0, intersection: 1, difference: 2}

/** Boolean result from the manifold-ops kernel (abi_manifold_boolean). */
export interface KernelManifoldBoolean {
  readonly positions: Float64Array<ArrayBuffer>
  readonly indices: Uint32Array<ArrayBuffer>
  /** True when an operand needed Full pre-repair before the boolean. */
  readonly operandRepaired: readonly [boolean, boolean]
  readonly kernel: {
    readonly toleranceMm: number
    readonly work: number
    readonly fragments: number
    readonly inputTriangles: readonly [number, number]
  }
  /** Strictly manifold output is the kernel contract; always true on success. */
  readonly isManifold: boolean
}

/**
 * Boolean `a OP b` with a guaranteed strictly manifold result: non-manifold
 * operands are pre-repaired (Full mode), unrepairable input and any
 * non-manifold kernel output reject with a GeometryKernelError.
 */
export function booleanManifoldInKernel(
  aVertices: Float32Array | Float64Array,
  aIndices: Uint32Array,
  bVertices: Float32Array | Float64Array,
  bIndices: Uint32Array,
  op: ManifoldBooleanOp,
): KernelManifoldBoolean {
  const {exports: wasm, memory, takeResponse} = kernelRuntime()
  let avp = 0
  let aip = 0
  let bvp = 0
  let bip = 0
  let handle = 0
  try {
    avp = copyBuffer(wasm, new Uint8Array(aVertices.buffer, aVertices.byteOffset, aVertices.byteLength))
    aip = copyBuffer(wasm, new Uint8Array(aIndices.buffer, aIndices.byteOffset, aIndices.byteLength))
    bvp = copyBuffer(wasm, new Uint8Array(bVertices.buffer, bVertices.byteOffset, bVertices.byteLength))
    bip = copyBuffer(wasm, new Uint8Array(bIndices.buffer, bIndices.byteOffset, bIndices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_boolean(
        avp, aVertices.length, aip, aIndices.length, kernelVertexFormat(aVertices),
        bvp, bVertices.length, bip, bIndices.length, kernelVertexFormat(bVertices),
        MANIFOLD_BOOLEAN_OP_TAG[op],
      )),
    )
    const buffer = memory.buffer
    const floats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 4), wasm.abi_array_field(handle, 5)))
    return {
      positions: new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1))),
      indices: new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3))),
      operandRepaired: [wasm.abi_array_field(handle, 7) !== 0, wasm.abi_array_field(handle, 8) !== 0],
      kernel: {
        toleranceMm: floats[0],
        work: wasm.abi_array_field(handle, 10),
        fragments: wasm.abi_array_field(handle, 9),
        inputTriangles: [wasm.abi_array_field(handle, 11), wasm.abi_array_field(handle, 12)],
      },
      isManifold: (wasm.abi_array_field(handle, 13) & 1) !== 0,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (bip) wasm.abi_free(bip, bIndices.byteLength)
    if (bvp) wasm.abi_free(bvp, bVertices.byteLength)
    if (aip) wasm.abi_free(aip, aIndices.byteLength)
    if (avp) wasm.abi_free(avp, aVertices.byteLength)
  }
}
