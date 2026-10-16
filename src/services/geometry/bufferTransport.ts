import {GeometryKernelError,kernelRuntime} from './kernel'

export function copyBuffer(wasm: ReturnType<typeof kernelRuntime>['exports'], bytes: Uint8Array): number {
  const ptr = wasm.abi_alloc(bytes.byteLength)
  if (!ptr) throw new GeometryKernelError('GEOMETRY_RESOURCE_LIMIT', 'Mesh exceeds transport limit')
  new Uint8Array(kernelRuntime().memory.buffer, ptr, bytes.byteLength).set(bytes)
  return ptr
}

