import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { GcodePreviewWorker, type GcodeWorkerPort } from '../src/services/gcodePreviewWorker'
import { checkGcodePreviewJob, GCODE_PREVIEW_MAX_BYTES, type GcodePreviewDocument, type GcodePreviewJob, type GcodePreviewRequest } from '../src/services/gcodePreviewProtocol'
import { drawGcodeLayer, gcodeLayerRange, gcodeMeshBounds } from '../src/services/gcodePreviewGeometry'
import { executeGcodePreview } from '../src/services/gcodePreviewRuntime'

const geometry = vi.hoisted(() => ({ emit: vi.fn(), parse: vi.fn() }))
vi.mock('../src/services/geometry/polygon', () => ({ emitPolygonMeshGcode: geometry.emit, parseGcodePreview: geometry.parse, GCODE_PREVIEW_DIALECT: 'open-scad-viewer/print-preview 2' }))
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
beforeEach(() => { vi.useFakeTimers(); vi.clearAllMocks() })
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
    geometry.emit.mockReturnValue({ ...result(), layerCount: 2 })
    const response = executeGcodePreview({ version: 1, id: 1, job: input })
    expect(response.ok).toBe(true)
    expect(geometry.emit).toHaveBeenCalledWith({ positions: [20, 30, -4, 15, 30, -4, 20, 28, 1], indices: [0, 2, 1] }, 0, 2, { layerHeightMm: .2 })
    expect(geometry.emit).toHaveBeenCalledOnce()
    expect(geometry.parse).not.toHaveBeenCalled()
    expect(() => gcodeMeshBounds({ ...input.mesh, indices: new Uint32Array([999]) })).toThrow('invalid vertex')
  })

  it('parses own preview files with the same canonical metadata contract', () => {
    geometry.parse.mockReturnValue(result().preview)
    const response = executeGcodePreview({ version: 1, id: 3, job: { kind: 'parse', gcode: '; file' } })
    expect(response).toEqual({ version: 1, id: 3, ok: true, result: { ...result(), gcode: '; file' } })
    expect(geometry.parse).toHaveBeenCalledWith('; file')
    expect(geometry.emit).not.toHaveBeenCalled()
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
