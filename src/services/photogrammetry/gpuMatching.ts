/**
 * Browser WebGPU execution of the kernel's descriptor matching sweep. The WGSL
 * shader text and the MAT1 payload layout come from the photogrammetry kernel
 * itself (photo_sparse_prepare), so the browser runs exactly the qualified
 * shader the native `gpu` feature compiles. Everything before (feature
 * extraction, pair enumeration) and after (acceptance filter, seeding,
 * registration) runs in the kernel; this module only dispatches the matcher.
 */

import { runGpuCompute } from '../webgpuCompute'

/** Workgroup size baked into the kernel's MATCH_WGSL (the compute-core anchor). */
const MATCH_WORKGROUP = 256

export interface GpuMatchingPayload {
  imageCount: number
  pairCount: number
  featureCounts: number[]
  /** Feature-table offset (in 128-float records) per image. */
  featureOffset: number[]
  /** Concatenated little-endian f32 descriptors, image-major. */
  descriptors: Float32Array
  pairs: [number, number][]
}

/** Parses the kernel's MAT1 payload; all pairs share one descriptor table. */
export function parseMatchingPayload(blob: Uint8Array): GpuMatchingPayload {
  const view = new DataView(blob.buffer, blob.byteOffset, blob.byteLength)
  let at = 0
  const u32 = () => {
    const v = view.getUint32(at, true)
    at += 4
    return v
  }
  if (u32() !== 0x314d4154) throw new Error('Invalid matching payload magic')
  const imageCount = u32()
  const pairCount = u32()
  if (u32() !== 128) throw new Error('Unsupported descriptor stride')
  const featureCounts: number[] = []
  const featureOffset: number[] = []
  let totalFeatures = 0
  for (let i = 0; i < imageCount; i++) {
    featureOffset.push(totalFeatures)
    const count = u32()
    featureCounts.push(count)
    totalFeatures += count
  }
  // The descriptor block starts here; share one backing store with zero copies.
  const descriptors = new Float32Array(blob.buffer, blob.byteOffset + at, totalFeatures * 128)
  at += totalFeatures * 128 * 4
  const pairs: [number, number][] = []
  for (let i = 0; i < pairCount; i++) {
    pairs.push([u32(), u32()])
  }
  return { imageCount, pairCount, featureCounts, featureOffset, descriptors, pairs }
}

/**
 * Matches every payload pair with the kernel's `match_pair` entry in one pass
 * and one submit; both directions bind the shared descriptor table through
 * per-pair offsets, so it uploads once. Returns, per pair in payload order,
 * `rows * 12` bytes of packed RowBest (j, d1, d2) followed by `cols * 8` bytes
 * of packed ColBest (i, d1) — exactly the wire format the kernel's
 * `matches_from_gpu_response` parses.
 */
export async function runGpuMatching(blob: Uint8Array, wgsl: string): Promise<Uint8Array> {
  const payload = parseMatchingPayload(blob)
  const dispatches = payload.pairs.map(([a, b]) => {
    const rows = payload.featureCounts[a]!
    const cols = payload.featureCounts[b]!
    const params = new ArrayBuffer(16)
    const pv = new DataView(params)
    pv.setUint32(0, rows, true)
    pv.setUint32(4, cols, true)
    pv.setUint32(8, payload.featureOffset[a]!, true)
    pv.setUint32(12, payload.featureOffset[b]!, true)
    return {
      buffers: [
        { binding: 0, data: params, uniform: true },
        // Same backing object at both bindings: the runtime uploads the table once.
        { binding: 1, data: payload.descriptors },
        { binding: 2, data: payload.descriptors },
        { binding: 3, data: new Float32Array(0), output: true, outputBytes: rows * 12 },
        { binding: 4, data: new Float32Array(0), output: true, outputBytes: cols * 8 },
      ],
      outputBytes: Math.max(rows * 12, cols * 8, 16),
      // One workgroup per row and per column, exactly like the native kernels.
      workgroups: [rows + cols, 1, 1] as [number, number, number],
    }
  })
  const parts = await runGpuCompute({ wgsl, entryPoint: 'match_pair', dispatches })
  const total = parts.reduce((sum, part) => sum + part.byteLength, 0)
  const flat = new Uint8Array(total)
  let offset = 0
  for (const part of parts) {
    flat.set(new Uint8Array(part.buffer, part.byteOffset, part.byteLength), offset)
    offset += part.byteLength
  }
  return flat
}
