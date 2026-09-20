import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { GcodePreviewWorker, type GcodeWorkerPort } from '../src/services/gcodePreviewWorker'
import { checkGcodePreviewJob, GCODE_PREVIEW_MAX_BYTES, type GcodePreviewDocument, type GcodePreviewJob, type GcodePreviewRequest } from '../src/services/gcodePreviewProtocol'
import { drawGcodeLayer, gcodeLayerRange, gcodeMeshBounds } from '../src/services/gcodePreviewGeometry'
import { executeGcodePreview, executeGcodePreviewAsync } from '../src/services/gcodePreviewRuntime'
import { prepareGcodePreviewTransfer } from '../src/services/gcodePreviewWorkerTransport'

const geometry = vi.hoisted(() => ({ emit: vi.fn(), emitJob: vi.fn(), parse: vi.fn(), flatten: vi.fn(), warm: vi.fn() }))
vi.mock('../src/services/geometry/kernel', () => ({ warmGeometryKernel: geometry.warm }))
vi.mock('../src/services/geometry/polygon', () => ({
  emitPolygonMeshGcode: geometry.emit,
  emitPolygonMeshGcodeJob: geometry.emitJob,
  inspectGcode: geometry.parse,
  GCODE_PREVIEW_DIALECT: 'open-scad-viewer/print-preview 2',
  GCODE_FLAVORS: ['marlin', 'klipper', 'reprapfirmware'],
}))
vi.mock('../src/services/meshFlatten', () => ({
  flattenGroupGeometry: (...args: unknown[]) => geometry.flatten(...args),
}))
const identity = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
const mesh = () => ({ vertices: new Float32Array([0, 0, 0, 0, 0, 1, 5, 0, 0, 0, 0, 1, 0, 5, 2, 0, 0, 1]), indices: new Uint32Array([0, 1, 2]), transform: identity.slice() })
const job = (): GcodePreviewJob => ({ kind: 'slice', mesh: mesh(), zMin: 0, zMax: 2, settings: { layerHeightMm: .2 } })
const result = (): GcodePreviewDocument => ({
  dialect: 'open-scad-viewer/print-preview 2', gcode: '; preview',
  preview: { layers: 2, extrusionMm: 2, depositedVolumeMm3: 4, travelDistanceMm: 3, printDistanceMm: 10, estimatedTimeS: 1,
    bounds: { min: [2, 3, .2], max: [7, 3, .4] }, moves: [
      { x: 2, y: 3, z: .2, e: 0, extruded: false, feedrateMmS: 100, layerIndex: 0 },
      { x: 7, y: 3, z: .2, e: 1, extruded: true, feedrateMmS: 50, layerIndex: 0 },
      { x: 2, y: 3, z: .4, e: 1, extruded: false, feedrateMmS: 100, layerIndex: 1 },
      { x: 7, y: 3, z: .4, e: 2, extruded: true, feedrateMmS: 50, layerIndex: 1 },
    ] },
})

class FakeWorker implements GcodeWorkerPort {
  listeners = new Map<string, Set<EventListener>>()
  posted: GcodePreviewRequest[] = []
  terminate = vi.fn()
  postMessage = vi.fn((message: GcodePreviewRequest) => { this.posted.push(structuredClone(message)) })
  addEventListener(type: string, listener: EventListener) { const listeners = this.listeners.get(type) ?? new Set(); listeners.add(listener); this.listeners.set(type, listeners) }
  removeEventListener(type: string, listener: EventListener) { this.listeners.get(type)?.delete(listener) }
  send(type: string, data?: unknown) { for (const listener of this.listeners.get(type) ?? []) listener({ data } as MessageEvent) }
  complete(document = result()) { this.send('message', { version: 1, id: this.posted.at(-1)!.id, ok: true, result: document }) }
}
const clients: GcodePreviewWorker[] = []
function setup(timeout = 120000) {
  const workers: FakeWorker[] = []
  const client = new GcodePreviewWorker(() => { const worker = new FakeWorker(); workers.push(worker); return worker }, timeout)
  clients.push(client)
  return { client, workers }
}
beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  geometry.warm.mockReset().mockResolvedValue(undefined)
  geometry.flatten.mockImplementation(() => ({ positions: [0, 0, 0, 5, 0, 0, 0, 5, 2], indices: [0, 1, 2] }))
})
afterEach(() => { clients.splice(0).forEach(client => client.dispose()); vi.useRealTimers() })

describe('G-code worker lifecycle', () => {
  it('requires a finite positive timeout within the worker deadline', () => {
    const factory = vi.fn(() => new FakeWorker())
    for (const timeout of [0, -1, NaN, Infinity, 120001]) expect(() => new GcodePreviewWorker(factory, timeout)).toThrow('timeout must be between')
    expect(factory).not.toHaveBeenCalled()
  })

  it('runs off-thread with a snapshot of only mesh geometry and reuses its warm worker', async () => {
    const { client, workers } = setup(), input = job()
    const pending = client.run(input)
    const sent = workers[0].posted[0]
    if (sent.job.kind !== 'slice' || input.kind !== 'slice') throw new Error('Expected slice')
    expect(Object.keys(sent.job.mesh).sort()).toEqual(['indices', 'transform', 'vertices'])
    input.mesh.vertices[0] = 99
    expect(sent.job.mesh.vertices[0]).toBe(0)
    expect(input.mesh.vertices.byteLength).toBeGreaterThan(0)
    workers[0].complete()
    await expect(pending).resolves.toEqual(result())
    const next = client.run({ kind: 'parse', gcode: '; preview' })
    expect(workers).toHaveLength(1)
    workers[0].complete()
    await next
    expect(workers[0].listeners.get('message')?.size).toBe(0)
    await vi.advanceTimersByTimeAsync(15000)
    expect(workers[0].terminate).toHaveBeenCalledOnce()
  })

  it('terminates cancelled or superseded work and ignores delivery from the old realm', async () => {
    const { client, workers } = setup()
    const first = client.run(job()), failure = expect(first).rejects.toMatchObject({ name: 'AbortError' })
    const second = client.run(job())
    await failure
    expect(workers[0].terminate).toHaveBeenCalledOnce()
    workers[0].complete()
    workers[1].complete()
    await expect(second).resolves.toEqual(result())
    const third = client.run(job()), cancelled = expect(third).rejects.toMatchObject({ name: 'AbortError' })
    client.cancel()
    await cancelled
    expect(workers[1].terminate).toHaveBeenCalledOnce()
  })

  it.each(['error', 'messageerror'])('recovers after a worker %s', async type => {
    const { client, workers } = setup()
    const first = client.run(job())
    workers[0].send(type)
    await expect(first).rejects.toThrow('stopped unexpectedly')
    expect(workers[0].terminate).toHaveBeenCalledOnce()
    const retry = client.run(job()); workers[1].complete(); await retry
  })

  it('rejects malformed responses and starts fresh', async () => {
    const { client, workers } = setup(), pending = client.run(job())
    workers[0].send('message', { version: 1, id: 999, ok: true, result: result() })
    await expect(pending).rejects.toThrow('Invalid response')
    expect(workers[0].terminate).toHaveBeenCalledOnce()
  })

  it('requests transferable moves, validates them and discards a malformed worker before retrying', async () => {
    const {client, workers} = setup(), first = client.run(job())
    expect(workers[0].posted[0].responseFormat).toBe('f64-moves-v1')
    const packed = prepareGcodePreviewTransfer({version: 1, id: workers[0].posted[0].id, ok: true, result: result()})
    workers[0].send('message', structuredClone(packed.response, {transfer: packed.transfer}))
    await expect(first).resolves.toEqual(result())
    const invalid = client.run(job())
    workers[0].send('message', {version: 1, id: workers[0].posted[1].id, ok: true,
      responseFormat: 'f64-moves-v1', result: {...result(), preview: {...result().preview, moves: undefined, moveRows: new Float64Array(1)}}})
    await expect(invalid).rejects.toThrow('Invalid response')
    expect(workers[0].terminate).toHaveBeenCalledOnce()
    const retry = client.run(job()); workers[1].complete(); await retry
  })

  it('times out unresponsive work and cleans up on owner disposal', async () => {
    const { client, workers } = setup(50), pending = client.run(job())
    const timedOut = expect(pending).rejects.toThrow('exceeded two minutes')
    await vi.advanceTimersByTimeAsync(50); await timedOut
    expect(workers[0].terminate).toHaveBeenCalledOnce()
    const retry = client.run(job()), disposed = expect(retry).rejects.toMatchObject({ name: 'AbortError' })
    client.dispose(); await disposed
    await expect(client.run(job())).rejects.toThrow('closed')
    expect(workers[1].listeners.get('message')?.size).toBe(0)
  })

  it('reports clone failures and recoverable computation errors', async () => {
    const { client, workers } = setup()
    const first = client.run(job())
    workers[0].send('message', { version: 1, id: 1, ok: false, error: 'Open mesh cannot be sliced.' })
    await expect(first).rejects.toThrow('Open mesh')
    workers[0].postMessage.mockImplementationOnce(() => { throw new Error('clone failed') })
    await expect(client.run(job())).rejects.toThrow('Could not send')
    expect(workers[0].terminate).toHaveBeenCalledOnce()
  })
})

describe('G-code worker boundary and scene geometry', () => {
  it('validates before async warming and waits before entering geometry', async () => {
    for (const invalid of [null, {version: 2, id: 1, job: {kind: 'parse', gcode: '; file'}},
      {version: 1, id: 1, job: {kind: 'parse', gcode: ''}},
      {version: 1, id: 1, responseFormat: 'unknown', job: {kind: 'parse', gcode: '; file'}}]) {
      expect((await executeGcodePreviewAsync(invalid as GcodePreviewRequest)).ok).toBe(false)
    }
    expect(geometry.warm).not.toHaveBeenCalled()
    let ready!: () => void
    geometry.warm.mockImplementationOnce(() => new Promise<void>(resolve => {ready = resolve}))
    geometry.parse.mockReturnValue({preview: result().preview, dialect: result().dialect, native: true})
    const request: GcodePreviewRequest = {version: 1, id: 7, job: {kind: 'parse', gcode: '; original'}}
    const pending = executeGcodePreviewAsync(request)
    expect(geometry.parse).not.toHaveBeenCalled()
    request.id = 8
    request.job = {kind: 'parse', gcode: '; replaced'}
    ready()
    expect(await pending).toMatchObject({id: 7, ok: true, result: {gcode: '; original'}})
    expect(geometry.parse).toHaveBeenCalledExactlyOnceWith('; original')
  })

  it('reports warmup failures through the existing envelope and supports a subsequent request', async () => {
    const request: GcodePreviewRequest = {version: 1, id: 1, job: {kind: 'parse', gcode: '; file'}}
    geometry.warm.mockRejectedValueOnce(new Error('WASM artifact mismatch'))
    expect(await executeGcodePreviewAsync(request)).toEqual({version: 1, id: 1, ok: false, error: 'WASM artifact mismatch'})
    expect(geometry.parse).not.toHaveBeenCalled()
    geometry.parse.mockReturnValue({preview: result().preview, dialect: result().dialect, native: true})
    expect((await executeGcodePreviewAsync({...request, id: 2})).ok).toBe(true)
    expect(geometry.parse).toHaveBeenCalledOnce()
  })

  it('shares validated slice and job execution with the synchronous entry', async () => {
    geometry.emit.mockReturnValue({...result(), layerCount: 2})
    geometry.emitJob.mockReturnValue({...result(), flavor: 'marlin', gcode3mfBase64: 'UEsDBBQAAAA='})
    const slice = job()
    if (slice.kind !== 'slice') throw new Error('Expected slice')
    for (const input of [slice, {...slice, kind: 'job' as const, settings: {flavor: 'marlin' as const}}]) {
      const request: GcodePreviewRequest = {version: 1, id: 3, job: input}
      expect(await executeGcodePreviewAsync(request)).toEqual(executeGcodePreview(request))
    }
  })

  it('rejects invalid ranges, fractional walls, blank numeric fields, and oversized UTF-8 files before launching a worker', async () => {
    const { client, workers } = setup(), input = job()
    if (input.kind !== 'slice') throw new Error('Expected slice')
    await expect(client.run({ ...input, zMax: 0 })).rejects.toThrow('Z max')
    await expect(client.run({ ...input, settings: { wallCount: 1.5 } })).rejects.toThrow('integer')
    await expect(client.run({ ...input, settings: { layerHeightMm: '' as unknown as number } })).rejects.toThrow('positive finite')
    await expect(client.run({ kind: 'parse', gcode: 'я'.repeat(GCODE_PREVIEW_MAX_BYTES / 2 + 1) })).rejects.toThrow('4 MiB')
    expect(workers).toHaveLength(0)
    for (const invalid of [null, { kind: 'other' }, { ...input, settings: null }, { ...input, settings: [] }]) {
      expect(() => checkGcodePreviewJob(invalid as GcodePreviewJob)).toThrow()
    }
    expect(executeGcodePreview(null as unknown as GcodePreviewRequest)).toEqual({ version: 1, id: 0, ok: false, error: 'Invalid G-code worker request.' })
  })

  it('omits undefined optional settings so the kernel applies defaults, while rejecting explicit null', () => {
    const input = job()
    if (input.kind !== 'slice') throw new Error('Expected slice')
    const checked = checkGcodePreviewJob({ ...input, settings: { layerHeightMm: undefined, lineWidthMm: .5 } })
    expect(checked).toMatchObject({ settings: { lineWidthMm: .5 } })
    if (checked.kind !== 'slice') throw new Error('Expected slice')
    expect(Object.keys(checked.settings)).toEqual(['lineWidthMm'])
    expect(() => checkGcodePreviewJob({ ...input, settings: { layerHeightMm: null as unknown as number } })).toThrow('positive finite')
  })

  it('computes exact transformed scene bounds and exports the same positioned mesh with mirrored winding repaired', () => {
    const input = job()
    if (input.kind !== 'slice') throw new Error('Expected slice')
    input.mesh.transform = new Float32Array([-1, 0, 0, 20, 0, 0, -1, 30, 0, 1, 0, -4, 0, 0, 0, 1])
    expect(gcodeMeshBounds(input.mesh)).toEqual({ min: [15, 28, -4], max: [20, 30, 1] })
    geometry.flatten.mockReturnValue({ positions: [20, 30, -4, 15, 30, -4, 20, 28, 1], indices: [0, 2, 1] })
    geometry.emit.mockReturnValue({ ...result(), layerCount: 2 })
    const response = executeGcodePreview({ version: 1, id: 1, job: input })
    expect(response.ok).toBe(true)
    expect(geometry.emit).toHaveBeenCalledWith({ positions: [20, 30, -4, 15, 30, -4, 20, 28, 1], indices: [0, 2, 1] }, 0, 2, { layerHeightMm: .2 })
    expect(geometry.emit).toHaveBeenCalledOnce()
    expect(geometry.parse).not.toHaveBeenCalled()
    expect(() => gcodeMeshBounds({ ...input.mesh, indices: new Uint32Array([999]) })).toThrow('invalid vertex')
  })

  it('parses own preview files with the same canonical metadata contract', () => {
    geometry.parse.mockReturnValue({ preview: result().preview, dialect: 'open-scad-viewer/print-preview 2', native: true, generator: 'open-scad-viewer', flavor: null, filamentDiameterMm: 1.75 })
    const response = executeGcodePreview({ version: 1, id: 3, job: { kind: 'parse', gcode: '; file' } })
    expect(response).toEqual({ version: 1, id: 3, ok: true, result: { ...result(), gcode: '; file', native: true, generator: 'open-scad-viewer', flavor: null } })
    expect(geometry.parse).toHaveBeenCalledWith('; file')
    expect(geometry.emit).not.toHaveBeenCalled()
  })

  it('reports detected generator and flavor for foreign slicer files', () => {
    geometry.parse.mockReturnValue({ preview: result().preview, dialect: 'PrusaSlicer G-code (tolerant preview)', native: false, generator: 'PrusaSlicer', flavor: 'marlin2', filamentDiameterMm: 1.75 })
    const response = executeGcodePreview({ version: 1, id: 5, job: { kind: 'parse', gcode: '; generated by PrusaSlicer' } })
    expect(response.ok).toBe(true)
    if (!response.ok) throw new Error('expected ok')
    expect(response.result).toMatchObject({ native: false, generator: 'PrusaSlicer', flavor: 'marlin2', dialect: 'PrusaSlicer G-code (tolerant preview)' })
  })

  it('exports job dialect gcode and gcode.3mf without network', () => {
    const input = job()
    if (input.kind !== 'slice') throw new Error('Expected slice')
    geometry.emitJob.mockReturnValue({
      ...result(),
      dialect: 'open-scad-viewer/print-job 1',
      flavor: 'klipper',
      layerCount: 2,
      gcode3mfBase64: 'UEsDBBQAAAA=',
    })
    const response = executeGcodePreview({
      version: 1,
      id: 4,
      job: { kind: 'job', mesh: input.mesh, zMin: 0, zMax: 2, settings: { layerHeightMm: 0.2, nozzleTempC: 210, homeAxes: true, flavor: 'klipper' } },
    })
    expect(response.ok).toBe(true)
    if (!response.ok) throw new Error('expected ok')
    expect(response.result.gcode3mfBase64).toBe('UEsDBBQAAAA=')
    expect(response.result.dialect).toBe('open-scad-viewer/print-job 1')
    expect(response.result.flavor).toBe('klipper')
    expect(geometry.emitJob).toHaveBeenCalledWith(expect.anything(), 0, 2, expect.objectContaining({ flavor: 'klipper' }))
    expect(() => checkGcodePreviewJob({ kind: 'job', mesh: input.mesh, zMin: 0, zMax: 2, settings: { flavor: 'sailfish' as unknown as 'marlin' } })).toThrow('flavor must be one of')
    expect(geometry.emitJob).toHaveBeenCalledOnce()
    expect(geometry.emit).not.toHaveBeenCalled()
    const checked = checkGcodePreviewJob({ kind: 'job', mesh: input.mesh, zMin: 0, zMax: 2, settings: { fanSpeed: 0, homeAxes: false } })
    expect(checked).toMatchObject({ kind: 'job', settings: { fanSpeed: 0, homeAxes: false } })
    expect(() => checkGcodePreviewJob({ kind: 'job', mesh: input.mesh, zMin: 0, zMax: 2, settings: { homeAxes: 'yes' as unknown as boolean } })).toThrow('boolean')
  })

  it('indexes layers and draws travel and extrusion without inventing an origin segment', () => {
    const preview = result().preview
    expect(gcodeLayerRange(preview, 0)).toEqual({ start: 0, end: 2, z: .2 })
    expect(gcodeLayerRange(preview, 1)).toEqual({ start: 2, end: 4, z: .4 })
    expect(gcodeLayerRange(preview, 2)).toEqual({ start: 4, end: 4, z: null })
    const context = { fillRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), setLineDash: vi.fn(), stroke: vi.fn(), fillText: vi.fn(), save: vi.fn(), translate: vi.fn(), rotate: vi.fn(), restore: vi.fn() }
    const canvas = { getContext: () => context } as unknown as HTMLCanvasElement
    drawGcodeLayer(canvas, preview, 0, true)
    expect(context.lineTo).toHaveBeenCalledOnce()
    context.lineTo.mockClear()
    drawGcodeLayer(canvas, preview, 1, true)
    expect(context.lineTo).toHaveBeenCalledTimes(2)
    context.lineTo.mockClear()
    drawGcodeLayer(canvas, preview, 1, false)
    expect(context.lineTo).toHaveBeenCalledOnce()
  })
})
