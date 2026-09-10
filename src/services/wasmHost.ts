/**
 * Shared helpers for the linear-memory ABI of the own Rust kernels
 * (geometry-wasm, photogrammetry-wasm): responses are packed u64
 * (len << 32 | ptr), requests are copied into fresh allocations.
 */
import {decodeBinary, type BinaryTripleHints} from './valueBinaryCodec'

export function packedPointer(packed: bigint): number { return Number(packed & 0xffffffffn) }
export function packedSize(packed: bigint): number { return Number(packed >> 32n) }

/** Copies bytes into a fresh linear-memory allocation; 0 when the kernel refused. */
export function writeLinear(memory: WebAssembly.Memory, alloc: (len: number) => number, bytes: Uint8Array): number {
  const pointer = alloc(bytes.length)
  if (pointer) new Uint8Array(memory.buffer, pointer, bytes.length).set(bytes)
  return pointer
}

/** Decodes an MGV1 response and releases its linear-memory buffer on any outcome. */
export function decodePacked<T>(
  memory: WebAssembly.Memory,
  free: (pointer: number, size: number) => void,
  packed: bigint,
  hints?: BinaryTripleHints,
): T {
  const pointer = packedPointer(packed), size = packedSize(packed)
  try {
    return decodeBinary(new Uint8Array(memory.buffer, pointer, size), hints) as T
  } finally {
    free(pointer, size)
  }
}
