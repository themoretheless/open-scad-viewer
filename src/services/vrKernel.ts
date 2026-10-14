import bytes from '../generated/vr-core/bytes'
import {unpackWasmBase64} from './wasmPacking'

interface Kernel extends WebAssembly.Exports {
  memory: WebAssembly.Memory
  vr_alloc(length: number): number
  vr_free(pointer: number, length: number): void
  vr_prepare(pointer: number, length: number): number
  vr_anchor(pointer: number): void
}
let instance: Kernel | undefined
function kernel(): Kernel {
  // Small independent module, instantiated on first VR use. No async boundary
  // between the user's click and WebXR requestSession.
  return instance ??= new WebAssembly.Instance(new WebAssembly.Module(
    unpackWasmBase64(bytes),
  )).exports as Kernel
}
export interface KernelMesh {
  vertices: Float32Array
  indices: Uint32Array
  transform: ArrayLike<number>
  color: readonly number[]
}
export interface PreparedVrMesh {
  positions: Float32Array
  indices: Uint32Array
  color: readonly number[]
}

export function prepareKernelScene(meshes: readonly KernelMesh[]): PreparedVrMesh[] {
  if (!meshes.length) throw new Error('No visible geometry')
  const k = kernel()
  const size = 4 + meshes.reduce((n, mesh) => n + 88 + (mesh.vertices.length + mesh.indices.length) * 4, 0)
  const input = k.vr_alloc(size)
  let output = 0, outputSize = 0
  try {
    const view = new DataView(k.memory.buffer, input, size)
    let offset = 0
    const uint = (v: number) => { view.setUint32(offset, v, true); offset += 4 }
    const float = (v: number) => { view.setFloat32(offset, v, true); offset += 4 }
    uint(meshes.length)
    for (const mesh of meshes) {
      if (mesh.transform.length !== 16 || mesh.color.length !== 4) throw new Error('Invalid VR geometry')
      uint(mesh.vertices.length); uint(mesh.indices.length)
      for (let i = 0; i < 16; i++) float(mesh.transform[i])
      mesh.color.forEach(float)
      mesh.vertices.forEach(float)
      mesh.indices.forEach(uint)
    }
    output = k.vr_prepare(input, size)
    if (!output) throw new Error('Invalid VR geometry')
    // WASM calls can grow memory: acquire a fresh view after preparation.
    const result = new DataView(k.memory.buffer)
    outputSize = result.getUint32(output, true)
    offset = output + 4
    const readUint = () => { const v = result.getUint32(offset, true); offset += 4; return v }
    const readFloats = (length: number) => {
      const values = new Float32Array(length)
      for (let i = 0; i < length; i++, offset += 4) values[i] = result.getFloat32(offset, true)
      return values
    }
    const count = readUint(), scene: PreparedVrMesh[] = []
    for (let i = 0; i < count; i++) {
      const nv = readUint(), ni = readUint()
      const color = Array.from(readFloats(4)), positions = readFloats(nv)
      const indices = new Uint32Array(ni)
      for (let j = 0; j < ni; j++) indices[j] = readUint()
      scene.push({ positions, indices, color })
    }
    return scene
  } finally {
    k.vr_free(input, size)
    if (output) k.vr_free(output, outputSize)
  }
}

export function prepareVrAnchor(pose: Float32Array): Float32Array {
  if (pose.length !== 16) throw new Error('Invalid XR pose')
  const k = kernel(), pointer = k.vr_alloc(64)
  try {
    new Uint8Array(k.memory.buffer, pointer, 64).set(new Uint8Array(pose.buffer, pose.byteOffset, 64))
    k.vr_anchor(pointer)
    // Copy via bytes: the ABI allocation has byte alignment.
    return new Float32Array(k.memory.buffer.slice(pointer, pointer + 64))
  } finally { k.vr_free(pointer, 64) }
}
