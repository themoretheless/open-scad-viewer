import {Worker} from 'node:worker_threads'
import {afterEach, expect, it, vi} from 'vitest'
import {GcodePreviewWorker, type GcodeWorkerPort} from '../src/services/gcodePreviewWorker'
import type {GcodePreviewRequest, GcodePreviewJob} from '../src/services/gcodePreviewProtocol'

class NodePort implements GcodeWorkerPort {
  private listeners = new Map<EventListener, (value: unknown) => void>()
  constructor(readonly worker: Worker) {}
  postMessage(value: GcodePreviewRequest) { this.worker.postMessage(value) }
  terminate() { void this.worker.terminate() }
  addEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener) {
    const callback = (data: unknown) => listener({data} as MessageEvent)
    this.listeners.set(listener, callback)
    this.worker.on(type, callback)
  }
  removeEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener) {
    const callback = this.listeners.get(listener)
    if (callback) this.worker.off(type, callback)
    this.listeners.delete(listener)
  }
}
const workers: Worker[] = [], clients: GcodePreviewWorker[] = []
function spawn(entry = '../src/workers/gcodePreview.worker.ts', warmState?: SharedArrayBuffer) {
  const worker = new Worker(new URL('./fixtures/web-worker-node-harness.mjs', import.meta.url), {
    workerData: {entryUrl: new URL(entry, import.meta.url).href, announceReady: !!warmState, warmState},
  })
  workers.push(worker)
  return new NodePort(worker)
}
afterEach(async () => {
  clients.splice(0).forEach(client => client.dispose())
  await Promise.all(workers.splice(0).map(worker => worker.terminate()))
})
const parse: GcodePreviewJob = {kind: 'parse', gcode: 'G1 X0 Y0 Z0.2 F600\nM83\nM200 D2\nM221 S50\nG1 X10 E4\n'}

it('uses the shipped worker, preserves material accounting and recovers after a parse error', async () => {
  const client = new GcodePreviewWorker(() => spawn())
  clients.push(client)
  const result = await client.run(parse)
  expect(result.preview.depositedVolumeMm3).toBeCloseTo(2, 12)
  expect(result.preview.extrusionMm).toBeCloseTo(2 / Math.PI, 12)
  expect(result.preview.printDistanceMm).toBe(10)
  await expect(client.run({kind: 'parse', gcode: 'G1 X0 Y0 Z0\nM221 S-1\n'})).rejects.toThrow(/nonnegative/)
  expect(await client.run(parse)).toEqual(result)
  expect(workers).toHaveLength(1)
}, 15000)

it('slices and exports a print job through the same warm real worker', async () => {
  const client = new GcodePreviewWorker(() => spawn())
  clients.push(client)
  const points = [[0,0,0],[10,0,0],[10,10,0],[0,10,0],[0,0,2],[10,0,2],[10,10,2],[0,10,2]]
  const mesh = {vertices: new Float32Array(points.flatMap(p => [...p, 0, 0, 1])),
    indices: new Uint32Array([0,1,2,0,2,3,4,6,5,4,7,6,0,5,1,0,4,5,1,6,2,1,5,6,2,7,3,2,6,7,3,4,0,3,7,4]),
    transform: new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])}
  const input = {mesh, zMin: 0, zMax: 1, settings: {layerHeightMm: 0.5, wallCount: 1}}
  const slice = await client.run({kind: 'slice', ...input})
  expect(slice.preview.layers).toBe(2)
  expect(slice.preview.extrusionMm).toBeGreaterThan(0)
  const job = await client.run({kind: 'job', ...input, settings: {...input.settings, flavor: 'klipper'}})
  expect(job.flavor).toBe('klipper')
  expect(Buffer.from(job.gcode3mfBase64!, 'base64').subarray(0, 2).toString()).toBe('PK')
  expect((await client.run({kind: 'parse', gcode: job.gcode})).preview).toEqual(job.preview)
  expect(mesh.vertices.byteLength).toBe(8 * 6 * 4)
  expect(workers).toHaveLength(1)
}, 15000)

it.each(['cancel', 'supersede', 'timeout'] as const)('terminates entered async warmup on %s and recovers', async mode => {
  const state = new Int32Array(new SharedArrayBuffer(4))
  const blocked = spawn('./fixtures/gcode-pending-warmup.ts', state.buffer)
  await new Promise<void>((resolve, reject) => {
    blocked.worker.once('message', data => data.__webWorkerHarness === 'ready' ? resolve() : reject(new Error('Unexpected readiness')))
    blocked.worker.once('error', reject)
  })
  let first = true
  const client = new GcodePreviewWorker(() => {
    if (!first) return spawn()
    first = false
    return blocked
  }, mode === 'timeout' ? 5000 : 120000)
  clients.push(client)
  const pending = client.run(parse)
  const rejected = mode === 'timeout' ? expect(pending).rejects.toThrow(/exceeded two minutes/)
    : expect(pending).rejects.toMatchObject({name: 'AbortError'})
  await vi.waitFor(() => expect(Atomics.load(state, 0)).toBe(1), {timeout: 1000, interval: 5})
  const exited = new Promise(resolve => blocked.worker.once('exit', resolve))
  let replacement
  if (mode === 'cancel') client.cancel()
  else if (mode === 'supersede') replacement = client.run(parse)
  await rejected
  await exited
  const recovered = await (replacement ?? client.run(parse))
  expect(recovered.preview.depositedVolumeMm3).toBeCloseTo(2, 12)
  expect(workers).toHaveLength(2)
}, 15000)
