import assert from 'node:assert/strict'
import {cpus} from 'node:os'
import {pathToFileURL} from 'node:url'
import {resolve} from 'node:path'
import {identity, type Mat4} from '../src/services/math3d'

// A/B benchmark for the GPU-driven vertex morph (candidate) vs the CPU
// interpolation + full vertex-buffer rewrite (baseline).
//
// Usage:
//   git worktree add tmp/bench-morph-baseline HEAD
//   node --import tsx benchmarks/morph-animation.mts tmp/bench-morph-baseline/src/services/webgpuRenderer.ts
//   git worktree remove tmp/bench-morph-baseline

assert.ok(process.argv[2], 'Pass the baseline webgpuRenderer.ts path (a clean worktree checkout)')
const baselineUrl = pathToFileURL(resolve(process.argv[2]))

type RendererModule = typeof import('../src/services/webgpuRenderer')

class FakeBuffer {
  readonly contents: Uint8Array
  constructor(readonly size: number, readonly usage: number) {
    this.contents = new Uint8Array(size)
  }
  destroy() { /* GC-owned; destroy counting is not part of this benchmark */ }
}

class FakeDevice {
  readonly buffers: FakeBuffer[] = []
  bytesWritten = 0
  writeCalls = 0
  readonly limits = { maxBufferSize: 1 << 30 }
  readonly queue = {
    writeBuffer: (buffer: FakeBuffer, offset: number, data: ArrayBuffer | ArrayBufferView) => {
      this.writeCalls++
      const bytes = ArrayBuffer.isView(data)
        ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
        : new Uint8Array(data)
      this.bytesWritten += bytes.byteLength
      buffer.contents.set(bytes, offset)
    },
  }
  createBuffer(descriptor: { size: number; usage: number }) {
    const buffer = new FakeBuffer(descriptor.size, descriptor.usage)
    this.buffers.push(buffer)
    return buffer
  }
  createBindGroup() { return {} }
}

interface MorphMeshState {
  morph?: { started: number }
  vb: FakeBuffer
  ub: FakeBuffer
}

function createHarness(module: RendererModule) {
  const renderer = new module.WebGPURenderer()
  const device = new FakeDevice()
  const internal = renderer as unknown as Record<string, unknown>
  internal.dev = device as unknown as GPUDevice
  internal.initialized = true
  internal.dead = false
  internal.lost = false
  internal.objBGL = {} as GPUBindGroupLayout
  internal.initialFitDone = true
  internal.scheduleEdgeBufferWarmup = () => {}
  internal.requestRender = () => {}
  return {
    renderer,
    device,
    meshes: () => (internal.meshes as MorphMeshState[]),
    advance: (now: number) => (internal.advanceGeometryAnimation as (n: number) => boolean).call(renderer, now),
  }
}

function gridFixture(side: number) {
  const vertices = new Float32Array((side + 1) ** 2 * 6)
  for (let y = 0; y <= side; y++) for (let x = 0; x <= side; x++) {
    const offset = (y * (side + 1) + x) * 6
    vertices[offset] = x
    vertices[offset + 1] = y
    vertices[offset + 5] = 1
  }
  const indices = new Uint32Array(side * side * 6)
  for (let y = 0; y < side; y++) for (let x = 0; x < side; x++) {
    const a = y * (side + 1) + x, b = a + 1, c = a + side + 1, d = c + 1
    indices.set([a, b, c, b, d, c], (y * side + x) * 6)
  }
  const rest = identity() as Mat4
  const moved = identity() as Mat4
  moved[3] = 20
  const publication = (transform: Float32Array) => ({
    entityId: 'entity:grid',
    vertices, indices,
    edgeIndices: new Uint32Array(0),
    faceIds: new Uint32Array(indices.length / 3),
    transform,
    color: [0.5, 0.5, 0.5, 1],
    provenance: [],
  })
  return { rest: publication(new Float32Array(rest)), moved: publication(new Float32Array(moved)) }
}

const FRAMES = 12
const FRAME_MS = 1000 / 60

function runRep(h: ReturnType<typeof createHarness>, fixture: ReturnType<typeof gridFixture>) {
  h.renderer.setMeshes([fixture.rest])
  const setupStart = performance.now()
  h.renderer.setMeshes([fixture.moved], { animate: true })
  const setupMs = performance.now() - setupStart
  const mesh = h.meshes()[0]
  const started = mesh.morph!.started
  h.device.bytesWritten = 0
  h.device.writeCalls = 0
  const frameStart = performance.now()
  for (let frame = 1; frame <= FRAMES; frame++) h.advance(started + frame * FRAME_MS)
  const framesMs = performance.now() - frameStart
  return {
    setupMs,
    framesMs,
    msPerFrame: framesMs / FRAMES,
    bytesPerFrame: h.device.bytesWritten / FRAMES,
    writeCallsPerFrame: h.device.writeCalls / FRAMES,
  }
}

const median = (values: number[]) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]

const candidate: RendererModule = await import('../src/services/webgpuRenderer')
const baseline: RendererModule = await import(baselineUrl.href)
const globalStub = globalThis as { GPUBufferUsage: unknown }
const previousUsage = globalStub.GPUBufferUsage
globalStub.GPUBufferUsage = { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8, STORAGE: 16 }

const samples = []
try {
  for (const side of [110, 320, 1000]) {
    const fixture = gridFixture(side)
    const hBase = createHarness(baseline)
    const hCand = createHarness(candidate)

    // Exact output parity outside timing: the finished vertex buffer and the
    // retired morph state must be identical for both implementations.
    const baseFinal = runRep(hBase, fixture)
    const candFinal = runRep(hCand, fixture)
    assert.equal(hBase.meshes()[0].morph, undefined)
    assert.equal(hCand.meshes()[0].morph, undefined)
    assert.deepEqual(hCand.meshes()[0].vb.contents, hBase.meshes()[0].vb.contents)

    const before: typeof baseFinal[] = []
    const after: typeof candFinal[] = []
    const SAMPLES = 9
    for (let sample = 0; sample < SAMPLES; sample++) {
      for (const current of sample % 2 ? [true, false] : [false, true]) {
        const result = runRep(current ? hCand : hBase, fixture)
        ;(current ? after : before).push(result)
      }
    }
    const field = (list: typeof before, pick: (r: typeof baseFinal) => number) => median(list.map(pick))
    samples.push({
      vertices: (side + 1) ** 2,
      triangles: side * side * 2,
      framesPerRep: FRAMES,
      parity: 'finished vb bytes and morph state equal outside timing',
      setupMs: { baseline: field(before, r => r.setupMs), candidate: field(after, r => r.setupMs) },
      msPerFrame: { baseline: field(before, r => r.msPerFrame), candidate: field(after, r => r.msPerFrame) },
      bytesPerFrame: { baseline: field(before, r => r.bytesPerFrame), candidate: field(after, r => r.bytesPerFrame) },
      writeCallsPerFrame: { baseline: field(before, r => r.writeCallsPerFrame), candidate: field(after, r => r.writeCallsPerFrame) },
    })
    hBase.renderer.destroy()
    hCand.renderer.destroy()
  }
} finally {
  if (previousUsage === undefined) delete globalStub.GPUBufferUsage
  else globalStub.GPUBufferUsage = previousUsage
}

console.log(JSON.stringify({
  node: process.version,
  cpu: cpus()[0]?.model,
  method: `Transform-only morph (worst case for the old path: it rewrote every vertex every frame even though from == target). ${FRAMES} frames at 60 fps per rep, 9 alternating baseline/candidate samples, medians. FakeDevice models upload cost as a memcpy into the destination buffer.`,
  samples,
}, null, 2))
