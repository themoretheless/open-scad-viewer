import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerEvent,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/kernel/geometryWorkerProtocol'
import { sha256Hex } from '../src/core/sha256'

// Count every kernel job step that crosses the ABI, from any caller.
const jobSteps = vi.hoisted(() => ({ calls: 0, onStep: null as null | (() => void) }))
vi.mock('../src/kernel/geometry/kernel', async importOriginal => {
  const original = await importOriginal<typeof import('../src/kernel/geometry/kernel')>()
  return {
    ...original,
    kernelRuntime: () => {
      const runtime = original.kernelRuntime()
      const exports = {
        ...runtime.exports,
        abi_job_step: (job: number, budget: number) => {
          jobSteps.calls++
          jobSteps.onStep?.()
          return runtime.exports.abi_job_step(job, budget)
        },
      }
      return { ...runtime, exports }
    },
  }
})

afterEach(() => {
  jobSteps.calls = 0
  jobSteps.onStep = null
  vi.unstubAllGlobals()
})

async function cad() {
  const { default: Module } = await import('../src/kernel/geometry/module')
  return Module()
}

function cancelledAfter(checks: number) {
  let seen = 0
  return async () => {
    if (++seen > checks) throw new Error('cancelled')
  }
}

describe('kernel start/step/cancel jobs', () => {
  it('cooperative Booleans and render preparation match the one-shot kernel paths', async () => {
    const { CadSolid } = await cad()
    const { renderMeshInKernel } = await import('../src/kernel/geometry/meshAnalysis')
    const { renderMeshCooperativelyInKernel } = await import('../src/kernel/geometry/kernelJobs')
    const operands = [
      CadSolid.cube(4, true),
      CadSolid.sphere(1.5, 24).translate([1.5, 0, 0]),
      CadSolid.sphere(1, 24).translate([-1.8, 0, 0]),
      CadSolid.cube(1, true).translate([9, 0, 0]),
    ]
    for (const operation of ['union', 'difference', 'intersection'] as const) {
      const expected = CadSolid[operation](operands)
      const got = await CadSolid.combineCooperatively(operands, operation, async () => {})
      const a = expected.getMesh(),
        b = got.getMesh()
      expect(b.vertProperties).toEqual(a.vertProperties)
      expect(b.triVerts).toEqual(a.triVerts)
      expect(b.faceID).toEqual(a.faceID)
      expected.delete()
      got.delete()
    }
    const sphere = CadSolid.sphere(3, 160)
    const crease = Math.cos((52.5 * Math.PI) / 180)
    const oneShot = renderMeshInKernel(sphere.handle, crease)
    jobSteps.calls = 0
    const cooperative = await renderMeshCooperativelyInKernel(sphere.handle, crease, async () => {})
    expect(jobSteps.calls).toBeGreaterThan(1)
    expect(cooperative).toEqual(oneShot)
    sphere.delete()
    for (const solid of operands) solid.delete()
  })

  it('stops within one step of a cancelled checkpoint and frees the job', async () => {
    const { CadSolid } = await cad()
    const operands = Array.from({ length: 12 }, (_, i) => CadSolid.sphere(1, 16).translate([i * 0.8, 0, 0]))
    jobSteps.calls = 0
    await CadSolid.combineCooperatively(operands, 'union', async () => {}).then(solid => solid.delete())
    const full = jobSteps.calls
    expect(full).toBeGreaterThan(4)
    // Far more cancelled jobs than the kernel admits concurrently: each one
    // must release its slot, or a later start would hit the job budget.
    for (let cancelAt = 0; cancelAt < 16; cancelAt++) {
      jobSteps.calls = 0
      const checks = cancelAt % (full - 1)
      await expect(CadSolid.combineCooperatively(operands, 'union', cancelledAfter(checks))).rejects.toThrow(
        'cancelled',
      )
      // One checkpoint precedes every step: no step runs after the refusal.
      expect(jobSteps.calls).toBe(checks)
    }
    const after = await CadSolid.combineCooperatively(operands, 'union', async () => {})
    expect(after.isEmpty()).toBe(false)
    after.delete()
    for (const solid of operands) solid.delete()
  })
})

class FakeWorkerScope {
  readonly events: GeometryWorkerEvent[] = []
  private readonly listeners = new Set<EventListenerOrEventListenerObject>()
  postMessage(event: GeometryWorkerEvent) {
    this.events.push(event)
  }
  addEventListener(_type: string, listener: EventListenerOrEventListenerObject) {
    this.listeners.add(listener)
  }
  dispatchMessage(data: GeometryWorkerRequest) {
    const event = { data } as MessageEvent<unknown>
    for (const listener of this.listeners) {
      if (typeof listener === 'function') listener(event)
      else listener.handleEvent(event)
    }
  }
}

function build(jobId: number, documentRevision: number, source: string): GeometryWorkerRequest {
  return {
    protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
    type: 'build',
    documentRevision,
    jobId,
    source,
    sourceSha256: sha256Hex(source),
    quality: 'full',
  }
}

const terminal = (scope: FakeWorkerScope, jobId: number) =>
  scope.events.find(
    event => event.jobId === jobId && ['succeeded', 'failed', 'cancelled', 'stale'].includes(event.status),
  )

describe('geometry Worker kernel job cancellation', () => {
  it('cancels a Boolean job within a bounded number of steps and keeps serving builds', async () => {
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    vi.resetModules()
    await import('../src/workers/geometry.worker')
    const heavy = 'union() { for (i = [0:23]) translate([i * 0.7, 0, 0]) sphere(1, $fn = 24); }'
    let stepsAtCancel = -1
    jobSteps.onStep = () => {
      if (stepsAtCancel >= 0 || jobSteps.calls < 2) return
      stepsAtCancel = jobSteps.calls
      // Delivered on the next macrotask, exactly like a MessagePort message.
      setTimeout(
        () =>
          scope.dispatchMessage({
            protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
            type: 'cancel',
            documentRevision: 1,
            jobId: 1,
            reason: 'user',
          }),
        0,
      )
    }
    scope.dispatchMessage(build(1, 1, heavy))
    await vi.waitFor(() => expect(terminal(scope, 1)).toBeDefined(), { timeout: 60_000 })
    expect(terminal(scope, 1)).toMatchObject({ status: 'cancelled', reason: 'user' })
    expect(stepsAtCancel).toBeGreaterThan(0)
    // The cancel is a macrotask; at most the step in flight plus the one whose
    // checkpoint yielded to deliver it can run after it was queued.
    expect(jobSteps.calls - stepsAtCancel).toBeLessThanOrEqual(2)
    expect(scope.events.every(event => isGeometryWorkerEvent(event))).toBe(true)

    // Same Worker scope and module instance: the next build runs to success.
    jobSteps.onStep = null
    const before = jobSteps.calls
    scope.dispatchMessage(build(2, 2, 'difference() { cube(4, center = true); sphere(2.5, $fn = 24); }'))
    await vi.waitFor(() => expect(terminal(scope, 2)).toBeDefined(), { timeout: 60_000 })
    expect(terminal(scope, 2)).toMatchObject({ status: 'succeeded' })
    expect(jobSteps.calls).toBeGreaterThan(before)
  }, 120_000)
})
