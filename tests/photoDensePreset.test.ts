import {describe, expect, it, vi} from 'vitest'
import {PhotogrammetryKernel, type PhotoDensePreset} from '../src/services/photogrammetryKernel'
import {compilePhotogrammetryKernel} from '../src/services/photogrammetryModule'
import {photoReportJson} from '../src/services/photoReport'

function transport() {
  const wasm = {photo_run: vi.fn(() => 7n), photo_dense: vi.fn(() => 9n)}
  const response = vi.fn(() => ({positions: new Float64Array(), colors: new Uint8Array(), triangles: new Uint32Array()}))
  // Exercise the real public dispatch method with observable ABI calls. The
  // separate test below instantiates the actual generated WASM export.
  const kernel = Object.assign(Object.create(PhotogrammetryKernel.prototype), {wasm, response}) as PhotogrammetryKernel
  return {kernel, wasm, response}
}

describe('bounded dense preset transport', () => {
  it('keeps the legacy default ABI and resolution', () => {
    const {kernel, wasm} = transport()
    kernel.dense()
    kernel.dense(192, 'baseline')
    expect(wasm.photo_run.mock.calls).toEqual([[2, 128], [2, 192]])
    expect(wasm.photo_dense).not.toHaveBeenCalled()
  })

  it('selects the experimental preset explicitly and rejects unknown values', () => {
    const {kernel, wasm} = transport()
    kernel.dense(128, 'slanted-plane')
    expect(wasm.photo_dense).toHaveBeenCalledWith(128, 1)
    expect(wasm.photo_run).not.toHaveBeenCalled()
    for (const bad of ['auto', '', 2, null]) {
      expect(() => kernel.dense(128, bad as PhotoDensePreset)).toThrow('Unknown dense')
    }
    expect(wasm.photo_dense).toHaveBeenCalledOnce()
  })

  it('reaches the actual WASM export and validates browser resolution there', async () => {
    const kernel = new PhotogrammetryKernel(await compilePhotogrammetryKernel())
    try {
      expect(() => kernel.dense(128, 'slanted-plane')).toThrow('Reconstruct cameras first')
      expect(() => kernel.dense(300, 'slanted-plane')).toThrow('resolution')
      expect(() => kernel.dense()).toThrow('Reconstruct cameras first')
    } finally { kernel.clear() }
  })

  it('routes the shared surface preset and enforces its WASM resolution cap', async () => {
    const {kernel, wasm} = transport()
    kernel.dense(128, 'dual-scale-volume')
    expect(wasm.photo_dense).toHaveBeenCalledWith(128, 2)
    const actual = new PhotogrammetryKernel(await compilePhotogrammetryKernel())
    try {
      expect(() => actual.dense(128, 'dual-scale-volume')).toThrow('Reconstruct cameras first')
      expect(() => actual.dense(192, 'dual-scale-volume')).toThrow('128')
    } finally { actual.clear() }
  })

  it('exports selected settings and counters without truncating beyond u32', () => {
    const json = photoReportJson({settings: {dense: true, resolution: 128, maxImageSide: 960, densePreset: 'slanted-plane'},
      inputs: [], sparse: null, timings: null,
      dense: {preset: 'slanted-plane', patchRadius: 2, depthHypotheses: 64,
        evaluatedHypotheses: 1000, evaluatedSourcePatches: 2000, sampledSourcePixels: 4294967307,
        estimatedMaps: 2, selectedSourcePairs: 2, photometricSamples: 100, consistentSamples: 90,
        rejectedInconsistentSamples: 10, fusedSamples: 20, vertices: 3, triangles: 1, viewReports: []}})
    const result = JSON.parse(json)
    expect(result.settings.densePreset).toBe('slanted-plane')
    expect(result.dense.sampledSourcePixels).toBe(4294967307)
  })
})
