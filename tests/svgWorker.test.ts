import { afterEach, describe, expect, it, vi } from 'vitest'
import { SvgWorkerClient, type SvgWorkerPort } from '../src/services/svgWorkerClient'
import type { SvgJob, SvgWorkerRequest } from '../src/services/svgWorkerProtocol'

class Port implements SvgWorkerPort {
  listeners = new Map<string, Set<EventListener>>()
  sent: SvgWorkerRequest[] = []
  terminate = vi.fn()
  postMessage = vi.fn((request: SvgWorkerRequest) => { this.sent.push(request) })
  addEventListener(type: string, listener: EventListener) { const set = this.listeners.get(type) ?? new Set(); set.add(listener); this.listeners.set(type, set) }
  removeEventListener(type: string, listener: EventListener) { this.listeners.get(type)?.delete(listener) }
  emit(data: unknown, type = 'message') { for (const listener of [...this.listeners.get(type) ?? []]) listener({ data } as MessageEvent) }
  success(result = output) { this.emit({ version: 1, id: this.sent.at(-1)!.id, ok: true, result }) }
}
const output = { svg: '<svg/>', widthMm: 10, heightMm: 10, warnings: [] }
const job: SvgJob = { kind: 'preview', svg: '<svg/>', options: {} }
const clients: SvgWorkerClient[] = []
afterEach(() => { clients.splice(0).forEach(client => client.dispose()); vi.useRealTimers() })
function setup(config: { timeoutMs?: number; idleMs?: number } = {}) {
  const ports: Port[] = [], factory = vi.fn(() => { const port = new Port(); ports.push(port); return port })
  const client = new SvgWorkerClient(factory, config); clients.push(client)
  return { client, ports, factory }
}

describe('SVG worker ownership and lifecycle', () => {
  it('keeps a warm worker for sequential calls, rejects stale messages, and releases an idle realm', async () => {
    vi.useFakeTimers()
    const { client, ports, factory } = setup({ idleMs: 50 })
    const first = client.run(job); ports[0].success(); await expect(first).resolves.toEqual(output)
    const second = client.run(job)
    ports[0].emit({ version: 1, id: 1, ok: true, result: { ...output, svg: 'stale' } })
    expect(factory).toHaveBeenCalledTimes(1)
    ports[0].success(); await expect(second).resolves.toEqual(output)
    await vi.advanceTimersByTimeAsync(50)
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
    expect([...ports[0].listeners.values()].every(set => !set.size)).toBe(true)
  })
  it('terminates superseded work and ignores callbacks retained by the old worker', async () => {
    const { client, ports } = setup()
    const first = client.run(job), failed = expect(first).rejects.toMatchObject({ name: 'AbortError' })
    const old = [...ports[0].listeners.get('message')!][0]
    const second = client.run(job); await failed
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
    old({ data: { version: 1, id: 1, ok: true, result: output } } as MessageEvent)
    ports[1].success(); await expect(second).resolves.toEqual(output)
  })
  it('aborts in-flight work immediately and admits a fresh next worker', async () => {
    const { client, ports } = setup(), abort = new AbortController()
    const task = client.run(job, { signal: abort.signal }), failed = expect(task).rejects.toMatchObject({ name: 'AbortError' })
    abort.abort(); await failed
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
    const next = client.run(job); ports[1].success(); await expect(next).resolves.toEqual(output)
  })
  it('does not start a worker for already cancelled or oversized input', async () => {
    const { client, factory } = setup(), abort = new AbortController(); abort.abort()
    await expect(client.run(job, { signal: abort.signal })).rejects.toMatchObject({ name: 'AbortError' })
    await expect(client.run({ ...job, svg: 'я'.repeat(2 * 1024 * 1024 + 1) })).rejects.toThrow('4 MiB')
    expect(factory).not.toHaveBeenCalled()
  })
  it('enforces the deadline by killing noncooperative work, then recovers', async () => {
    vi.useFakeTimers()
    const { client, ports } = setup({ timeoutMs: 100 })
    const task = client.run(job), failed = expect(task).rejects.toMatchObject({ code: 'SVG_TIMEOUT' })
    await vi.advanceTimersByTimeAsync(100); await failed
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
    const next = client.run(job); ports[1].success(); await expect(next).resolves.toEqual(output)
  })
  it.each(['error', 'messageerror'])('releases a worker after %s and permits recovery', async type => {
    const { client, ports } = setup()
    const task = client.run(job); ports[0].emit({}, type)
    await expect(task).rejects.toMatchObject({ code: 'SVG_CRASH' })
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
    const next = client.run(job); ports[1].success(); await expect(next).resolves.toEqual(output)
  })
  it('keeps validated conversion errors distinct from worker failures', async () => {
    const { client, ports } = setup()
    const task = client.run(job)
    ports[0].emit({ version: 1, id: 1, ok: false, error: { name: 'GeometryKernelError', code: 'E_IMPORT_INVALID_DATA', message: 'Malformed path' } })
    await expect(task).rejects.toMatchObject({ code: 'E_IMPORT_INVALID_DATA', name: 'GeometryKernelError', message: 'Malformed path' })
    expect(ports[0].terminate).not.toHaveBeenCalled()
  })
  it('rejects malformed success payloads and releases the worker', async () => {
    const { client, ports } = setup()
    const task = client.run(job); ports[0].emit({ version: 1, id: 1, ok: true, result: { svg: '<svg onload="bad"/>' } })
    await expect(task).rejects.toMatchObject({ code: 'SVG_PROTOCOL' })
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
  })
  it('cleans up failed postMessage and constructor failures', async () => {
    const port = new Port(); port.postMessage.mockImplementation(() => { throw new Error('DataCloneError') })
    const factory = vi.fn(() => port), client = new SvgWorkerClient(factory); clients.push(client)
    await expect(client.run(job)).rejects.toMatchObject({ code: 'SVG_TRANSPORT' })
    expect(port.terminate).toHaveBeenCalledTimes(1)
    expect([...port.listeners.values()].every(set => !set.size)).toBe(true)
    factory.mockImplementation(() => { throw new Error('blocked') })
    await expect(client.run(job)).rejects.toMatchObject({ code: 'SVG_STARTUP' })
  })
  it('disposes pending jobs, timers and listeners and never restarts a disposed owner', async () => {
    vi.useFakeTimers()
    const { client, ports, factory } = setup()
    const task = client.run(job), failed = expect(task).rejects.toMatchObject({ name: 'AbortError' })
    client.dispose(); await failed
    expect(vi.getTimerCount()).toBe(0)
    expect(ports[0].terminate).toHaveBeenCalledTimes(1)
    await expect(client.run(job)).rejects.toMatchObject({ code: 'SVG_DISPOSED' })
    expect(factory).toHaveBeenCalledTimes(1)
  })
  it('sends only required mesh fields without transferring live scene buffers', async () => {
    const { client, ports } = setup()
    const mesh = { vertices: new Float32Array(18), indices: new Uint32Array([0, 1, 2]), faceIds: new Uint32Array([7]), transform: new Float32Array(16), largeMetadata: new Uint8Array(100) }
    const task = client.run({ kind: 'project', meshes: [mesh], axis: 'z', options: {} })
    const sent = ports[0].sent[0].job
    expect(sent.kind).toBe('project')
    if (sent.kind === 'project') expect(Object.keys(sent.meshes[0]).sort()).toEqual(['faceIds', 'indices', 'transform', 'vertices'])
    expect(mesh.vertices.byteLength).toBe(72)
    ports[0].success(); await task
  })
  it('admits a selected face in a large scene by copying only that face and remapping its index', async () => {
    const { client, ports } = setup()
    const mesh = { vertices: new Float32Array(18), indices: new Uint32Array(60003), faceIds: new Uint32Array(20001).fill(1), transform: new Float32Array(16) }
    mesh.indices.set([0, 1, 2], 60000); mesh.faceIds[20000] = 9
    const task = client.run({ kind: 'project', meshes: [mesh, mesh], axis: 'z', face: { meshIndex: 1, triangleIndex: 20000 }, options: {} })
    const sent = ports[0].sent[0].job
    expect(sent.kind).toBe('project')
    if (sent.kind === 'project') {
      expect(sent.face).toEqual({ meshIndex: 0, triangleIndex: 0 })
      expect(sent.meshes).toHaveLength(1)
      expect(sent.meshes[0].indices).toHaveLength(3)
      expect(sent.meshes[0].vertices).toHaveLength(18)
    }
    ports[0].success(); await task
    await expect(client.run({ kind: 'project', meshes: [mesh], axis: 'z', options: {} })).rejects.toThrow('20000')
  })
})
