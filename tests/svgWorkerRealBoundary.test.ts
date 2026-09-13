import { Worker } from 'node:worker_threads'
import { afterEach, expect, it } from 'vitest'
import { SvgWorkerClient, type SvgWorkerPort } from '../src/services/svgWorkerClient'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import type { SvgWorkerRequest } from '../src/services/svgWorkerProtocol'

const workers: Worker[] = [], clients: SvgWorkerClient[] = []
class NodePort implements SvgWorkerPort {
  private listeners = new Map<EventListener, (value: unknown) => void>()
  constructor(readonly worker: Worker) {}
  postMessage(value: SvgWorkerRequest) { this.worker.postMessage(value) }
  terminate() { void this.worker.terminate() }
  addEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener) {
    const callback = (value: unknown) => listener({ data: value } as MessageEvent)
    this.listeners.set(listener, callback); this.worker.on(type, callback)
  }
  removeEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener) {
    const callback = this.listeners.get(listener); if (callback) this.worker.off(type, callback)
  }
}
afterEach(async () => { clients.splice(0).forEach(client => client.dispose()); await Promise.all(workers.splice(0).map(worker => worker.terminate())) })
it('runs the shipped browser entry in an isolated real worker and returns portable artwork and valid SCAD', async () => {
  const client = new SvgWorkerClient(() => {
    const worker = new Worker(new URL('./fixtures/web-worker-node-harness.mjs', import.meta.url), { workerData: { entryUrl: new URL('../src/workers/svg.worker.ts', import.meta.url).href, announceReady: false } })
    workers.push(worker); return new NodePort(worker)
  }); clients.push(client)
  let ticks = 0
  const timer = setInterval(() => ticks++, 5)
  try {
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><defs><linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs><rect width="20" height="10" fill="url(#g)"/></svg>'
    const preview = await client.run({ kind: 'preview', svg, options: {} })
    expect(preview.svg).toContain('linearGradient'); expect(preview.widthMm).toBe(20); expect(ticks).toBeGreaterThan(0)
    const solid = await client.run({ kind: 'extrude', svg, height: 3, options: {} })
    expect(workers).toHaveLength(1)
    const analysis = await new HeadlessGeometryService().analyze(solid.source!, 'full')
    expect(analysis.volume).toBeCloseTo(600, 5)
    expect(analysis.topology.boundary).toBe(0)
    const contours = await client.run({ kind: 'contours', svg: solid.svg, options: {} })
    expect(contours.svg).toContain('fill-rule="evenodd"')
    const geometry = new HeadlessGeometryService(), cube = await geometry.compile('cube([40,30,2]);', 'full')
    const bytesBefore = cube.meshes[0].vertices.byteLength
    const projected = await client.run({ kind: 'project', meshes: cube.meshes, axis: 'z', options: {} })
    expect(projected.svg).toContain('width="40mm"')
    expect(projected.svg).toContain('height="30mm"')
    expect(projected.svg).not.toContain('transform=')
    expect(cube.meshes[0].vertices.byteLength).toBe(bytesBefore)
    const imported = await client.run({ kind: 'extrude', svg: projected.svg, height: 2, options: {} })
    expect((await geometry.analyze(imported.source!, 'full')).bounds).toEqual({ min: [0, 0, 0], max: [40, 30, 2] })
  } finally { clearInterval(timer) }
}, 30000)
it('terminates a real noncooperative worker on timeout so a later operation can start', async () => {
  const client = new SvgWorkerClient(() => {
    const worker = new Worker('const {parentPort}=require("node:worker_threads");parentPort.on("message",()=>{for(;;){}})', { eval: true })
    workers.push(worker); return new NodePort(worker)
  }, { timeoutMs: 100 }); clients.push(client)
  await expect(client.run({ kind: 'preview', svg: '<svg/>', options: {} })).rejects.toMatchObject({ code: 'SVG_TIMEOUT' })
  await expect(client.run({ kind: 'preview', svg: '<svg/>', options: {} })).rejects.toMatchObject({ code: 'SVG_TIMEOUT' })
  expect(workers).toHaveLength(2)
}, 10000)

it('preserves CSS geometry and non-scaling stroke volume through the real worker and model export', async () => {
  const client = new SvgWorkerClient(() => {
    const worker = new Worker(new URL('./fixtures/web-worker-node-harness.mjs', import.meta.url), { workerData: { entryUrl: new URL('../src/workers/svg.worker.ts', import.meta.url).href, announceReady: false } })
    workers.push(worker); return new NodePort(worker)
  }); clients.push(client)
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="4mm" height="4mm" viewBox="0 0 100 100" style="width:100mm;height:100mm">
    <style>path { d:path('M10 20H20'); fill:none; stroke:#369; stroke-width:2mm; vector-effect:non-scaling-stroke }</style>
    <path transform="scale(3 .5)"/></svg>`
  const preview = await client.run({ kind: 'preview', svg, options: {} })
  expect(preview.widthMm).toBeCloseTo(100, 4)
  expect(preview.heightMm).toBeCloseTo(100, 4)
  const solid = await client.run({ kind: 'extrude', svg, height: 3, options: {} })
  const geometry = new HeadlessGeometryService()
  const analysis = await geometry.analyze(solid.source!, 'full')
  expect(analysis.volume).toBeCloseTo(180, 3)
  expect(analysis.topology.boundary).toBe(0)
  const scene = await geometry.compile(solid.source!, 'full')
  const projected = await client.run({ kind: 'project', meshes: scene.meshes, axis: 'z', options: {} })
  const imported = await client.run({ kind: 'extrude', svg: projected.svg, height: 3, options: {} })
  expect((await geometry.analyze(imported.source!, 'full')).volume).toBeCloseTo(180, 3)
}, 30000)
