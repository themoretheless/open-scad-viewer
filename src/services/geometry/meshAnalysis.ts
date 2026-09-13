/**
 * Raw-buffer bindings for the Rust mesh-analysis kernels (BVH build and
 * semantic edge extraction). Transport follows the handle + typed-view copy
 * pattern: the kernel returns a handle, the views are copied into TS-owned
 * buffers (they must survive memory.grow and worker transfers), then the
 * handle is released. Validation and error contracts stay with the callers.
 */
import type {MeshTopologyDiagnostics} from '../../core/mesh'
import {GeometryKernelError, decodeNurbsResult, kernelRuntime} from './kernel'

export interface KernelBvhResult {
  readonly nodeCount: number
  readonly bounds: Float32Array
  readonly nodes: Uint32Array
  readonly triangles: Uint32Array
}

export interface KernelSemanticEdgesResult {
  readonly indices: Uint32Array
  readonly diagnostics: MeshTopologyDiagnostics
}

function copyBuffer(wasm: ReturnType<typeof kernelRuntime>['exports'], bytes: Uint8Array): number {
  const ptr = wasm.abi_alloc(bytes.byteLength)
  if (!ptr) throw new GeometryKernelError('GEOMETRY_RESOURCE_LIMIT', 'Mesh exceeds transport limit')
  new Uint8Array(kernelRuntime().memory.buffer, ptr, bytes.byteLength).set(bytes)
  return ptr
}

/** One upload per mesh; retain f64 placed coordinates without numeric JSON encoding. */
export function placeSolidMeshInKernel(vertices: Float32Array, indices: Uint32Array, matrix: Float32Array): { positions: number[]; indices: number[] } | null {
  const {exports: wasm, takeResponse} = kernelRuntime()
  const allocations: Array<[number, number]> = []
  let handle = 0
  const upload = (view: Float32Array | Uint32Array) => {
    if (!view.byteLength) return 0
    const ptr = copyBuffer(wasm, new Uint8Array(view.buffer, view.byteOffset, view.byteLength))
    allocations.push([ptr, view.byteLength])
    return ptr
  }
  try {
    const vp = upload(vertices), ip = upload(indices), mp = upload(matrix)
    handle = decodeNurbsResult<number>(takeResponse(wasm.abi_solid_placement(vp, vertices.length, ip, indices.length, mp, matrix.length)))
    if (!handle) return null
    const positionsPtr = wasm.abi_array_field(handle, 0), positionsLen = wasm.abi_array_field(handle, 1)
    const indicesPtr = wasm.abi_array_field(handle, 2), indicesLen = wasm.abi_array_field(handle, 3)
    const buffer = kernelRuntime().memory.buffer
    return {
      positions: Array.from(new Float64Array(buffer, positionsPtr, positionsLen)),
      indices: Array.from(new Uint32Array(buffer, indicesPtr, indicesLen)),
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    for (const [ptr, length] of allocations) wasm.abi_free(ptr, length)
  }
}

export interface PreparedExportMesh { positions: Float64Array; indices: Uint32Array; normals: Float64Array }
/** Batched export geometry, copied out before releasing all native buffers. */
export function prepareExportMeshInKernel(vertices: Float32Array, indices: Uint32Array, matrix: Float32Array, float32: boolean): PreparedExportMesh {
  const {exports: wasm, takeResponse} = kernelRuntime()
  const allocations: Array<[number, number]> = []
  let handle = 0
  const upload = (view: Float32Array | Uint32Array) => {
    if (!view.byteLength) return 0
    const ptr = wasm.abi_export_alloc(view.byteLength)
    if (!ptr) throw new GeometryKernelError('GEOMETRY_RESOURCE_LIMIT', 'Mesh export exceeds transport limit')
    allocations.push([ptr, view.byteLength])
    new Uint8Array(kernelRuntime().memory.buffer, ptr, view.byteLength).set(new Uint8Array(view.buffer, view.byteOffset, view.byteLength))
    return ptr
  }
  try {
    const vp = upload(vertices), ip = upload(indices), mp = upload(matrix)
    handle = decodeNurbsResult<number>(takeResponse(wasm.abi_export_prepare(vp, vertices.length, ip, indices.length, mp, matrix.length, Number(float32))))
    const p = wasm.abi_array_field(handle, 0), pl = wasm.abi_array_field(handle, 1)
    const i = wasm.abi_array_field(handle, 2), il = wasm.abi_array_field(handle, 3)
    const n = wasm.abi_array_field(handle, 4), nl = wasm.abi_array_field(handle, 5)
    const buffer = kernelRuntime().memory.buffer
    return { positions: new Float64Array(buffer,p,pl).slice(), indices: new Uint32Array(buffer,i,il).slice(), normals: new Float64Array(buffer,n,nl).slice() }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    for (const [ptr, length] of allocations) wasm.abi_free(ptr, length)
  }
}

/** Build the median-split BVH in the Rust kernel. Options are pre-clamped by the caller. */
export function createPickingSnapshotInKernel(vertices: Float32Array, indices: Uint32Array, stride: number, leafSize: number): string {
  const {exports: wasm, takeResponse} = kernelRuntime()
  let vp = 0, ip = 0
  try {
    if (vertices.byteLength) vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    if (indices.byteLength) ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    return decodeNurbsResult<string>(takeResponse(wasm.abi_picking_create(stride, leafSize, vp, vertices.length, ip, indices.length)))
  } finally {
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Build the median-split BVH in the Rust kernel. Options are pre-clamped by the caller. */
export function buildBvhInKernel(
  vertices: Float32Array,
  indices: Uint32Array,
  vertexStride: number,
  leafSize: number,
): KernelBvhResult {
  const {exports: wasm, memory, takeResponse} = kernelRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_bvh_build(vertexStride, leafSize, vp, vertices.length, ip, indices.length)),
    )
    // Allocation above may grow memory. Never cache memory.buffer between calls.
    const buffer = memory.buffer
    const boundsPtr = wasm.abi_array_field(handle, 0)
    const boundsLen = wasm.abi_array_field(handle, 1)
    const nodesPtr = wasm.abi_array_field(handle, 2)
    const nodesLen = wasm.abi_array_field(handle, 3)
    const trianglesPtr = wasm.abi_array_field(handle, 4)
    const trianglesLen = wasm.abi_array_field(handle, 5)
    return {
      nodeCount: nodesLen >>> 1,
      bounds: new Float32Array(buffer, boundsPtr, boundsLen).slice(),
      nodes: new Uint32Array(buffer, nodesPtr, nodesLen).slice(),
      triangles: new Uint32Array(buffer, trianglesPtr, trianglesLen).slice(),
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/**
 * Extract semantic edges in the Rust kernel. `creaseDotThreshold` is computed
 * by the caller (Math.cos of the clamped angle) so the classification
 * constant matches the JavaScript evaluation bit-for-bit.
 */
export function extractSemanticEdgesInKernel(
  vertices: Float32Array,
  indices: Uint32Array,
  mergeFrom: ArrayLike<number> | undefined,
  mergeTo: ArrayLike<number> | undefined,
  weldCoincident: boolean,
  creaseDotThreshold: number,
): KernelSemanticEdgesResult {
  const {exports: wasm, memory, takeResponse} = kernelRuntime()
  const mergeFromArray = mergeFrom ? Uint32Array.from(mergeFrom) : new Uint32Array()
  const mergeToArray = mergeTo ? Uint32Array.from(mergeTo) : new Uint32Array()
  let vp = 0
  let ip = 0
  let mfp = 0
  let mtp = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    mfp = copyBuffer(wasm, new Uint8Array(mergeFromArray.buffer))
    mtp = copyBuffer(wasm, new Uint8Array(mergeToArray.buffer))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_semantic_edges(
        vp, vertices.length,
        ip, indices.length,
        mfp, mergeFromArray.length,
        mtp, mergeToArray.length,
        weldCoincident ? 1 : 0,
        creaseDotThreshold,
      )),
    )
    const buffer = memory.buffer
    const indicesPtr = wasm.abi_array_field(handle, 0)
    const indicesLen = wasm.abi_array_field(handle, 1)
    return {
      indices: new Uint32Array(buffer, indicesPtr, indicesLen).slice(),
      diagnostics: {
        boundary: wasm.abi_array_field(handle, 2),
        crease: wasm.abi_array_field(handle, 3),
        nonManifold: wasm.abi_array_field(handle, 4),
        degenerate: wasm.abi_array_field(handle, 5),
      },
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (mtp) wasm.abi_free(mtp, mergeToArray.byteLength)
    if (mfp) wasm.abi_free(mfp, mergeFromArray.byteLength)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}
