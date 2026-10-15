/**
 * Browser WebGPU execution of the kernel's NCC depth sweep. The WGSL shader
 * text and the payload layout come from the photogrammetry kernel itself
 * (photo_dense_prepare), so the browser runs exactly the qualified shader.
 * Everything before (sparse) and after (selection, consistency, fusion) runs
 * in the kernel as usual; this module only dispatches the scoring shader.
 */

import { runGpuCompute } from '../webgpuCompute'
import type { WgslVariant } from '../webgpuFeatures'

export const SWEEP_LINEAR_INDEXING_VARIANT = 'photogrammetry-sweep-linear-indexing'

export interface GpuSweepSource {
  image: number
  /** 16 f32: row-major rotation (9), translation (3), focal, cx, cy, pad. */
  floats: Float32Array
}
export interface GpuSweepView {
  mapWidth: number
  mapHeight: number
  needed: number
  refImage: number
  step: number
  refF: number
  refCx: number
  refCy: number
  hypotheses: Float32Array
  sources: GpuSweepSource[]
}
export interface GpuSweepPayload {
  imageWidth: number[]
  imageHeight: number[]
  imageOffset: number[]
  gray: Float32Array
  hypothesesPerView: number
  patchRadius: number
  views: (GpuSweepView | null)[]
}

/** Parses the kernel's SWP1 payload; all views share the concatenated grays. */
export function parseSweepPayload(blob: Uint8Array): GpuSweepPayload {
  const view = new DataView(blob.buffer, blob.byteOffset, blob.byteLength)
  let at = 0
  const u32 = () => {
    const v = view.getUint32(at, true)
    at += 4
    return v
  }
  const floats = (n: number) => {
    const out = new Float32Array(blob.buffer, blob.byteOffset + at, n)
    at += n * 4
    return out
  }
  if (u32() !== 0x31505753) throw new Error('Invalid sweep payload magic')
  const imageCount = u32()
  const hypothesesPerView = u32()
  const patchRadius = u32()
  const imageWidth: number[] = []
  const imageHeight: number[] = []
  const imageOffset: number[] = []
  let grayFloats = 0
  for (let i = 0; i < imageCount; i++) {
    imageWidth.push(u32())
    imageHeight.push(u32())
    imageOffset.push(grayFloats)
    grayFloats += imageWidth[i] * imageHeight[i]
  }
  // The gray block starts here; share one backing store with zero copies.
  const gray = floats(grayFloats)
  const viewCount = u32()
  const views: (GpuSweepView | null)[] = new Array(viewCount)
  for (let i = 0; i < viewCount; i++) {
    if (u32() === 0) {
      views[i] = null
      continue
    }
    const mapWidth = u32()
    const mapHeight = u32()
    const sourceCount = u32()
    const needed = u32()
    const refImage = u32()
    const hdrFloats = floats(4)
    const step = hdrFloats[0]!
    const refF = hdrFloats[1]!
    const refCx = hdrFloats[2]!
    const refCy = hdrFloats[3]!
    const hypotheses = floats(hypothesesPerView).slice()
    const sources: GpuSweepSource[] = new Array(sourceCount)
    for (let s = 0; s < sourceCount; s++) {
      const floats16 = floats(16).slice()
      const image = u32()
      u32()
      u32()
      u32()
      sources[s] = { image, floats: floats16 }
    }
    views[i] = {
      mapWidth, mapHeight, needed, refImage,
      step, refF, refCx, refCy,
      hypotheses, sources,
    }
  }
  return { imageWidth, imageHeight, imageOffset, gray, hypothesesPerView, patchRadius, views }
}

const EMPTY_F32 = new Float32Array(0)

/** Scores every present view with the kernel's shader; views stay in image order.
 * One pass, one submit, concurrent readbacks; the shared gray raster uploads once. */
export async function runGpuSweep(blob: Uint8Array, wgsl: string, wgslVariants?: readonly WgslVariant[]): Promise<Float32Array> {
  const payload = parseSweepPayload(blob)
  const patchLen = (payload.patchRadius * 2 + 1) ** 2
  const dispatches = []
  for (let vIdx = 0; vIdx < payload.views.length; vIdx++) {
    const view = payload.views[vIdx]
    if (!view) continue
    const scoresFloats = view.mapWidth * view.mapHeight * payload.hypothesesPerView
    const params = new ArrayBuffer(64)
    const pu32 = new Uint32Array(params, 0, 12)
    const pf32 = new Float32Array(params, 48, 4)
    const srcCount = view.sources.length
    pu32[0] = view.mapWidth
    pu32[1] = view.mapHeight
    pu32[2] = payload.hypothesesPerView
    pu32[3] = srcCount
    pu32[4] = patchLen
    pu32[5] = payload.patchRadius
    pu32[6] = view.needed
    pu32[7] = payload.imageWidth[view.refImage]!
    pu32[8] = payload.imageHeight[view.refImage]!
    pu32[9] = payload.imageOffset[view.refImage]!
    pu32[10] = 0
    pu32[11] = 0
    pf32[0] = view.step
    pf32[1] = view.refF
    pf32[2] = view.refCx
    pf32[3] = view.refCy
    const srcf = new Float32Array(srcCount * 16)
    const srcm = new Uint32Array(srcCount * 4)
    for (let s = 0; s < srcCount; s++) {
      const source = view.sources[s]!
      srcf.set(source.floats, s * 16)
      const mBase = s * 4
      const img = source.image
      srcm[mBase] = payload.imageOffset[img]!
      srcm[mBase + 1] = payload.imageWidth[img]!
      srcm[mBase + 2] = payload.imageHeight[img]!
      srcm[mBase + 3] = 0
    }
    dispatches.push({
      buffers: [
        { binding: 0, data: params, uniform: true },
        { binding: 1, data: view.hypotheses },
        { binding: 2, data: srcf },
        { binding: 3, data: srcm },
        { binding: 4, data: payload.gray },
        { binding: 5, data: EMPTY_F32, output: true },
      ],
      outputBytes: scoresFloats * 4,
      workgroups: [Math.ceil(view.mapWidth / 16), Math.ceil(view.mapHeight / 16), 1] as [number, number, number],
      variantWorkgroups: {
        [SWEEP_LINEAR_INDEXING_VARIANT]: [Math.ceil((view.mapWidth * view.mapHeight) / 256), 1, 1] as [number, number, number],
      },
    })
  }
  const parts = await runGpuCompute({ wgsl, wgslVariants, entryPoint: 'sweep', dispatches })
  let total = 0
  for (let i = 0; i < parts.length; i++) {
    total += parts[i]!.length
  }
  const flat = new Float32Array(total)
  let offset = 0
  for (let i = 0; i < parts.length; i++) {
    const scores = parts[i]!
    flat.set(scores, offset)
    offset += scores.length
  }
  return flat
}
