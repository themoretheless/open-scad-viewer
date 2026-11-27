import { copyBuffer } from './bufferTransport'
import { decodeNurbsResult, kernelRuntime, kernelVertexFormat } from './kernel'

/** Expand an indexed mesh into a flat triangle list with crease-smoothed normals in the kernel. */
export function smoothTriangleListInKernel(
  positions: Float32Array | Float64Array,
  indices: Uint32Array,
  cosCrease: number,
): { positions: Float32Array; normals: Float32Array } {
  const { exports: wasm, memory, takeResponse } = kernelRuntime()
  let vp = 0,
    ip = 0,
    handle = 0
  try {
    if (positions.byteLength)
      vp = copyBuffer(wasm, new Uint8Array(positions.buffer, positions.byteOffset, positions.byteLength))
    if (indices.byteLength)
      ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(
        wasm.abi_smooth_triangle_list(
          vp,
          positions.length,
          ip,
          indices.length,
          kernelVertexFormat(positions),
          cosCrease,
        ),
      ),
    )
    const buffer = memory.buffer
    return {
      positions: new Float32Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1)).slice(),
      normals: new Float32Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3)).slice(),
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, positions.byteLength)
  }
}

/** Fill stride-6 vertex normal slots in-place via the Rust kernel. */
export function fillVertexNormalsInKernel(vertices: Float32Array, indices: Uint32Array): void {
  if (vertices.length === 0 || indices.length === 0) return
  const { exports: wasm, memory, takeResponse } = kernelRuntime()
  let vp = 0,
    ip = 0,
    handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_fill_vertex_normals(vp, vertices.length, ip, indices.length)),
    )
    const buffer = memory.buffer
    vertices.set(new Float32Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1)))
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Expand an indexed mesh into flat-shaded stride-6 vertices, sequential indices, and surface area in the kernel. */
export function expandFlatTrianglesInKernel(
  positions: Float32Array | Float64Array,
  indices: Uint32Array,
): { vertices: Float32Array; indices: Uint32Array; surfaceArea: number } {
  const { exports: wasm, memory, takeResponse } = kernelRuntime()
  let vp = 0,
    ip = 0,
    handle = 0
  try {
    if (positions.byteLength)
      vp = copyBuffer(wasm, new Uint8Array(positions.buffer, positions.byteOffset, positions.byteLength))
    if (indices.byteLength)
      ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(
        wasm.abi_expand_flat_triangles(vp, positions.length, ip, indices.length, kernelVertexFormat(positions)),
      ),
    )
    const buffer = memory.buffer
    const floats = new Float64Array(buffer, wasm.abi_array_field(handle, 4), wasm.abi_array_field(handle, 5))
    return {
      vertices: new Float32Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1)).slice(),
      indices: new Uint32Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3)).slice(),
      surfaceArea: floats[0] ?? 0,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, positions.byteLength)
  }
}
