import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest'
import type {PhotoDiagnostics, PhotoReconstruction, PhotoSurface} from '../src/services/photogrammetryKernel'
import type {PhotoWorkerEvent, PhotoWorkerRequest} from '../src/services/photoWorkerProtocol'

const factory = vi.hoisted(() => vi.fn())
vi.mock('../src/services/photogrammetryKernel', () => ({
  PhotogrammetryKernel: class {
    constructor() { return factory() }
  },
}))

const sparse: PhotoReconstruction = {
  positions: [[0, 0, 1]], colors: [[10, 20, 30]], triangles: [],
  cameras: [], inputImages: 2, reprojectionRmse: 0.5,
}
const surface: PhotoSurface = {
  positions: [[0, 0, 1], [1, 0, 1], [0, 1, 1]],
  colors: [[10, 20, 30], [40, 50, 60], [70, 80, 90]], triangles: [[0, 1, 2]],
}
const diagnostics: PhotoDiagnostics = {
  images: [], initialPair: null, seedPairsTested: 2, matchingRequests: 3,
  computedPairs: 3, bundleRuns: [], warnings: [],
  seedTrials: [{pair: [0, 1], registeredImages: 0, points: 0, reprojectionRmse: null, error: 'No valid seed'}],
}
const request: PhotoWorkerRequest = {
  images: [], dense: true, resolution: 128,
}

function makeKernel() {
  return {
    add: vi.fn(() => 1),
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
  port.onmessage!({data: input} as MessageEvent<PhotoWorkerRequest>)
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

})
