import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest'
import type {PhotoDiagnostics, PhotoReconstruction, PhotoSurface} from '../src/services/photogrammetry/kernel'
import type {PhotoWorkerEvent, PhotoWorkerRequest} from '../src/services/photogrammetry/workerProtocol'

const factory = vi.hoisted(() => vi.fn())
vi.mock('../src/services/photogrammetry/kernel', () => ({
  PhotogrammetryKernel: class {
    constructor() { return factory() }
  },
}))
const gpuSweep = vi.hoisted(() => vi.fn())
vi.mock('../src/services/photogrammetry/gpuSweep', () => ({runGpuSweep: gpuSweep}))
const gpuMatching = vi.hoisted(() => vi.fn())
vi.mock('../src/services/photogrammetry/gpuMatching', () => ({runGpuMatching: gpuMatching}))

const sparse: PhotoReconstruction = {
  positions: new Float64Array([0, 0, 1]), colors: new Uint8Array([10, 20, 30]), triangles: new Uint32Array(),
  cameras: [], inputImages: 2, reprojectionRmse: 0.5,
}
const surface: PhotoSurface = {
  positions: new Float64Array([0, 0, 1, 1, 0, 1, 0, 1, 1]),
  colors: new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80, 90]), triangles: new Uint32Array([0, 1, 2]),
}
const diagnostics: PhotoDiagnostics = {
  images: [], initialPair: null, seedPairsTested: 2, matchingRequests: 3,
  computedPairs: 3, bundleRuns: [], warnings: [],
  seedTrials: [{pair: [0, 1], registeredImages: 0, points: 0, reprojectionRmse: null, error: 'No valid seed'}],
}
const request: PhotoWorkerRequest = {
  // The panel compiles the module once on the main thread and passes it in; the
  // mocked kernel below ignores it, any object marks the argument as provided.
  images: [], dense: true, resolution: 128, module: {} as WebAssembly.Module,
}

function makeKernel() {
  return {
    add: vi.fn(() => 1),
    densePrepare: vi.fn(() => null),
    denseFinish: vi.fn(() => structuredClone(surface)),
    sparsePrepare: vi.fn(() => null),
    sparseFinish: vi.fn(() => structuredClone(sparse)),
    sparse: vi.fn(() => structuredClone(sparse)),
    dense: vi.fn(() => structuredClone(surface)),
    compact: vi.fn(() => structuredClone(surface)),
    report: vi.fn<() => PhotoDiagnostics | null>(() => null),
    clear: vi.fn(),
  }
}

let kernel: ReturnType<typeof makeKernel>
let events: PhotoWorkerEvent[]
let port: {
  onmessage: ((event: MessageEvent<PhotoWorkerRequest>) => void) | null
  postMessage: (event: PhotoWorkerEvent) => void
}

beforeEach(() => {
  vi.resetModules()
  factory.mockReset()
  kernel = makeKernel()
  factory.mockReturnValue(kernel)
  events = []
  port = {onmessage: null, postMessage: event => events.push(structuredClone(event))}
  vi.stubGlobal('self', port)
})
afterEach(() => vi.unstubAllGlobals())

async function run(input = request): Promise<void> {
  // Exercise the real entry point and its error boundaries, not a reimplementation.
  await import('../src/workers/photogrammetry.worker')
  await port.onmessage!({data: input} as MessageEvent<PhotoWorkerRequest>)!
}

function terminalEvents() {
  return events.filter(event => event.type === 'done' || event.type === 'error')
}

describe('photogrammetry Worker failure isolation', () => {
  it('publishes the full dense surface when optional CAD compaction fails', async () => {
    kernel.compact.mockImplementation(() => { throw new Error('Compaction failed') })
    await run()
    expect(events).toContainEqual({type: 'sparse', result: sparse})
    expect(events).toContainEqual({type: 'surface', result: surface})
    expect(events).toContainEqual({type: 'warning', message: 'Compaction failed'})
    expect(terminalEvents()).toHaveLength(1)
    expect(terminalEvents()[0].type).toBe('done')
    expect(kernel.clear).toHaveBeenCalledOnce()
  })

  it('preserves the sparse result if depth reconstruction fails', async () => {
    kernel.dense.mockImplementation(() => { throw new Error('Insufficient depth agreement') })
    await run()
    expect(events).toContainEqual({type: 'sparse', result: sparse})
    expect(events).toContainEqual({type: 'warning', message: 'Insufficient depth agreement'})
    expect(events.some(event => event.type === 'surface')).toBe(false)
    expect(kernel.compact).not.toHaveBeenCalled()
    expect(terminalEvents()).toHaveLength(1)
    expect(terminalEvents()[0].type).toBe('done')
  })

  it('keeps the original sparse error and trial diagnostics when cleanup fails', async () => {
    kernel.sparse.mockImplementation(() => { throw new Error('No valid seed') })
    kernel.report.mockReturnValue(diagnostics)
    kernel.clear.mockImplementation(() => { throw new Error('Cleanup failed') })
    await expect(run()).resolves.toBeUndefined()
    expect(terminalEvents()).toEqual([{type: 'error', message: 'No valid seed', diagnostics}])
  })

  it('keeps the original error if report retrieval and cleanup both fail', async () => {
    kernel.sparse.mockImplementation(() => { throw new Error('Original reconstruction failure') })
    kernel.report.mockImplementation(() => { throw new Error('Report failed') })
    kernel.clear.mockImplementation(() => { throw new Error('Cleanup failed') })
    await expect(run()).resolves.toBeUndefined()
    expect(terminalEvents()).toEqual([
      {type: 'error', message: 'Original reconstruction failure', diagnostics: undefined},
    ])
  })

  it('normalizes the WASM null report to an absent Worker diagnostic', async () => {
    kernel.sparse.mockImplementation(() => { throw new Error('No observations') })
    kernel.report.mockReturnValue(null)
    await run()
    expect(terminalEvents()).toEqual([{type: 'error', message: 'No observations', diagnostics: undefined}])
  })

  it('does not turn successful completion into an error when cleanup fails', async () => {
    kernel.clear.mockImplementation(() => { throw new Error('Cleanup failed') })
    await expect(run({...request, dense: false})).resolves.toBeUndefined()
    expect(kernel.dense).not.toHaveBeenCalled()
    expect(terminalEvents()).toHaveLength(1)
    expect(terminalEvents()[0].type).toBe('done')
    expect(events.some(event => event.type === 'error')).toBe(false)
  })
  it('forwards explicit calibration unchanged before camera reconstruction', async () => {
    const image = {width: 64, height: 64, focal: 70, rgb: new Uint8Array(64 * 64 * 3),
      calibration: {sourceWidth: 64, sourceHeight: 64,
        group: {id: 'lens-a', label: 'Measured lens', source: 'Calibration board', imageWidth: 64, imageHeight: 64,
          fx: 70, fy: 72, cx: 32, cy: 32,
          distortion: {model: 'brown-conrady' as const, k1: 0.1, k2: 0, k3: 0, p1: 0, p2: 0}}}}
    await run({...request, images: [image], dense: false})
    expect(kernel.add).toHaveBeenCalledWith(image)
    expect(events[0]).toEqual({type: 'stage', stage: 'calibration'})
    expect(kernel.add.mock.invocationCallOrder[0]).toBeLessThan(kernel.sparse.mock.invocationCallOrder[0])
    expect(terminalEvents()).toHaveLength(1)
  })

  it('reports an input correction failure without starting sparse reconstruction', async () => {
    kernel.add.mockImplementation(() => { throw new Error('Calibration dimensions mismatch') })
    await run({...request, images: [{width: 64, height: 64, focal: 70, rgb: new Uint8Array(64 * 64 * 3)}]})
    expect(kernel.sparse).not.toHaveBeenCalled()
    expect(terminalEvents()).toEqual([{type: 'error', message: 'Calibration dimensions mismatch', diagnostics: undefined}])
    expect(kernel.clear).toHaveBeenCalledOnce()
  })

  it('retains the legacy dense call when no preset was requested', async () => {
    await run()
    expect(kernel.dense).toHaveBeenCalledWith(128)
  })

  it('forwards the experimental preset and preserves the dense work report', async () => {
    const measured = {...surface, denseDiagnostics: {
      preset: 'slanted-plane' as const, patchRadius: 2, depthHypotheses: 64,
      evaluatedHypotheses: 1000, evaluatedSourcePatches: 2000, sampledSourcePixels: 4294967307,
      estimatedMaps: 2, selectedSourcePairs: 2, photometricSamples: 100, consistentSamples: 90,
      rejectedInconsistentSamples: 10, fusedSamples: 20, vertices: 3, triangles: 1, viewReports: [],
    }}
    kernel.dense.mockReturnValue(measured)
    await run({...request, densePreset: 'slanted-plane'})
    expect(kernel.dense).toHaveBeenCalledWith(128, 'slanted-plane')
    const event = events.find(event => event.type === 'surface')
    expect(event?.type === 'surface' && event.result.denseDiagnostics).toEqual(measured.denseDiagnostics)
  })


  it('prefers the WebGPU sweep for baseline dense and skips the CPU dense call', async () => {
    const wgslVariants = [{label: 'fast', wgsl: 'fast shader'}]
    const payload = new Uint8Array(4)
    kernel.densePrepare.mockReturnValue({payload, wgsl: 'shader', wgslVariants})
    kernel.compact.mockImplementation(() => { throw new Error('skip compact') })
    gpuSweep.mockResolvedValue(new Float32Array(8))
    await run({...request, gpu: true})
    expect(kernel.densePrepare).toHaveBeenCalledWith(128)
    expect(gpuSweep).toHaveBeenCalledWith(payload, 'shader', wgslVariants)
    expect(kernel.denseFinish).toHaveBeenCalledOnce()
    expect(kernel.dense).not.toHaveBeenCalled()
    expect(events).toContainEqual({type: 'surface', result: surface})
    expect(terminalEvents()).toHaveLength(1)
    expect(terminalEvents()[0].type).toBe('done')
  })

  it('falls back to the CPU dense path when the GPU sweep is unavailable', async () => {
    kernel.compact.mockImplementation(() => { throw new Error('skip compact') })
    kernel.densePrepare.mockReturnValue(null)
    await run({...request, gpu: true})
    expect(kernel.dense).toHaveBeenCalledOnce()
    expect(events).toContainEqual({type: 'surface', result: surface})
    expect(terminalEvents()[0].type).toBe('done')
  })

  it('falls back to the CPU dense path when the GPU sweep rejects', async () => {
    kernel.compact.mockImplementation(() => { throw new Error('skip compact') })
    kernel.densePrepare.mockReturnValue({payload: new Uint8Array(4), wgsl: 'shader'})
    gpuSweep.mockRejectedValue(new Error('no adapter'))
    await run({...request, gpu: true})
    expect(kernel.dense).toHaveBeenCalledOnce()
    expect(terminalEvents()[0].type).toBe('done')
  })

  it('keeps the CPU path for non-baseline presets even with the GPU flag', async () => {
    await run({...request, gpu: true, densePreset: 'slanted-plane'})
    expect(kernel.densePrepare).not.toHaveBeenCalled()
    expect(kernel.dense).toHaveBeenCalledWith(128, 'slanted-plane')
  })

  it('prefers the WebGPU matching for sparse and skips the CPU sparse call', async () => {
    const payload = new Uint8Array(8)
    const matched = new Uint8Array([1, 2, 3])
    kernel.sparsePrepare.mockReturnValue({payload, wgsl: 'match shader'})
    gpuMatching.mockResolvedValue(matched)
    await run({...request, gpu: true})
    expect(kernel.sparsePrepare).toHaveBeenCalledOnce()
    expect(gpuMatching).toHaveBeenCalledWith(payload, 'match shader')
    expect(kernel.sparseFinish).toHaveBeenCalledWith(matched)
    expect(kernel.sparse).not.toHaveBeenCalled()
    expect(events).toContainEqual({type: 'sparse', result: sparse})
    expect(terminalEvents()[0].type).toBe('done')
  })

  it('falls back to the CPU sparse path when GPU matching declines', async () => {
    kernel.sparsePrepare.mockReturnValue(null)
    await run({...request, gpu: true})
    expect(kernel.sparse).toHaveBeenCalledOnce()
    expect(kernel.sparseFinish).not.toHaveBeenCalled()
    expect(terminalEvents()[0].type).toBe('done')
  })

  it('falls back to the CPU sparse path when GPU matching rejects', async () => {
    kernel.sparsePrepare.mockReturnValue({payload: new Uint8Array(4), wgsl: 'shader'})
    gpuMatching.mockRejectedValue(new Error('no adapter'))
    await run({...request, gpu: true})
    expect(kernel.sparse).toHaveBeenCalledOnce()
    expect(events).toContainEqual({type: 'warning', message: 'WebGPU matching unavailable: no adapter'})
    expect(terminalEvents()[0].type).toBe('done')
  })
})
