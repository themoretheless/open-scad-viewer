import { Worker } from 'node:worker_threads'
import { afterEach, describe, expect, it } from 'vitest'
import { sha256Hex } from '../src/core/sha256'
import {
  ManifoldPlanQualificationWorkerLane,
  type ManifoldPlanQualificationWorkerLike,
} from '../src/services/manifoldPlanQualificationWorkerLane'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerEvent,
  type GeometryBuildRequest,
  type GeometryWorkerEvent,
} from '../src/services/geometryWorkerProtocol'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationTerminal,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationTerminal,
} from '../src/services/manifoldPlanQualificationProtocol'

const liveWorkers = new Set<Worker>()
const harnessUrl = new URL('./fixtures/web-worker-node-harness.mjs', import.meta.url)

function spawnWebWorker(entryUrl: URL, announceReady = true): Worker {
  const worker = new Worker(harnessUrl, {
    workerData: { entryUrl: entryUrl.href, announceReady },
  })
  liveWorkers.add(worker)
  worker.once('exit', () => liveWorkers.delete(worker))
  return worker
}

class NodeWorkerAsWebWorker implements ManifoldPlanQualificationWorkerLike {
  private readonly listeners = new Map<EventListener, (...args: unknown[]) => void>()

  constructor(private readonly worker: Worker) {}

  postMessage(message: unknown): void {
    this.worker.postMessage(message)
  }

  terminate(): void {
    void this.worker.terminate()
  }

  addEventListener(
    type: 'message' | 'error' | 'messageerror',
    listener: EventListener,
  ): void {
    const wrapped = type === 'message'
      ? (data: unknown) => listener({ data } as MessageEvent<unknown>)
      : type === 'error'
        ? (error: unknown) => listener({ error } as ErrorEvent)
        : (error: unknown) => listener({ data: error } as MessageEvent<unknown>)
    this.listeners.set(listener, wrapped)
    this.worker.on(type, wrapped)
  }

  removeEventListener(
    type: 'message' | 'error' | 'messageerror',
    listener: EventListener,
  ): void {
    const wrapped = this.listeners.get(listener)
    if (wrapped === undefined) return
    this.listeners.delete(listener)
    this.worker.off(type, wrapped)
  }
}

function waitForMessage<T>(worker: Worker, predicate: (value: unknown) => value is T): Promise<T> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      cleanup()
      reject(new Error('Timed out waiting for real Worker boundary message'))
    }, 10_000)
    const onMessage = (value: unknown) => {
      if (!predicate(value)) return
      cleanup()
      resolve(value)
    }
    const onError = (error: Error) => {
      cleanup()
      reject(error)
    }
    const cleanup = () => {
      clearTimeout(timeout)
      worker.off('message', onMessage)
      worker.off('error', onError)
    }
    worker.on('message', onMessage)
    worker.on('error', onError)
  })
}

async function ready(worker: Worker): Promise<void> {
  await waitForMessage(worker, (value): value is { __webWorkerHarness: 'ready' } => (
    value !== null && typeof value === 'object'
      && (value as { __webWorkerHarness?: unknown }).__webWorkerHarness === 'ready'
  ))
}

function isDirectTerminalFor(request: GeometryBuildRequest) {
  return (value: unknown): value is GeometryWorkerEvent => isGeometryWorkerEvent(value)
    && ['succeeded', 'failed', 'cancelled', 'stale'].includes(value.status)
    && value.documentRevision === request.documentRevision
    && value.jobId === request.jobId
    && value.quality === request.quality
    && value.sourceSha256 === request.sourceSha256
}

async function terminate(worker: Worker): Promise<void> {
  liveWorkers.delete(worker)
  await worker.terminate()
}

afterEach(async () => {
  await Promise.all([...liveWorkers].map(terminate))
})

describe('real structured-clone geometry Worker boundaries', () => {
  it('preflights direct-route 256/257 identities before transfer and keeps the same real Worker healthy', async () => {
    const worker = spawnWebWorker(new URL('../src/workers/geometry.worker.ts', import.meta.url))
    await ready(worker)
    const request = (jobId: number, source: string): GeometryBuildRequest => ({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: jobId,
      jobId,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    })

    const moduleName = 'x'.repeat(91)
    const exactBoundarySource = `module ${moduleName}(){cube(1);} ${moduleName}();`
    const exactRequest = request(1, exactBoundarySource)
    worker.postMessage(exactRequest)
    const exact = await waitForMessage(worker, isDirectTerminalFor(exactRequest))
    expect(exact).toMatchObject({ jobId: 1, status: 'succeeded' })
    if (exact.status !== 'succeeded') throw new Error('expected exact-boundary success')
    expect(exact.meshes[0].entityId).toHaveLength(256)
    expect(exact.meshes[0].provenance[0].source?.instanceId).toHaveLength(256)

    const oversizedSource = 'assert(true) if(true) let(x=2) cube(x);'
    const oversizedRequest = request(2, oversizedSource)
    worker.postMessage(oversizedRequest)
    const oversized = await waitForMessage(worker, isDirectTerminalFor(oversizedRequest))
    expect(oversized).toMatchObject({jobId:2,status:'succeeded',meshes:[expect.objectContaining({entityId:expect.stringMatching(/^entity:sha256:/)})]})
    expect(JSON.stringify(oversized)).not.toContain('entity:root>op:root/call%3Aassert')

    const recoveryRequest = request(3, 'cube(1);')
    worker.postMessage(recoveryRequest)
    await expect(waitForMessage(worker, isDirectTerminalFor(recoveryRequest))).resolves.toMatchObject({
      jobId: 3,
      status: 'succeeded',
    })
    await terminate(worker)
  }, 30_000)

  it('runs plan-route 256/257 preflight in real disposable browser Worker realms', async () => {
    const run = async (source: string, workerEpoch: number): Promise<McpManifoldPlanQualificationTerminal> => {
      const worker = spawnWebWorker(
        new URL('../src/workers/manifoldPlanQualification.worker.ts', import.meta.url),
      )
      const exited = new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error('Qualification Worker did not exit')), 10_000)
        worker.once('exit', () => {
          clearTimeout(timeout)
          resolve()
        })
      })
      await ready(worker)
      const request: McpManifoldPlanQualificationRequest = {
        protocolVersion: MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
        type: 'evaluate',
        workerEpoch,
        jobId: workerEpoch,
        source,
        sourceSha256: sha256Hex(source),
        quality: 'full',
        identity: MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
      }
      worker.postMessage(request)
      const terminal = await waitForMessage(worker, (value): value is McpManifoldPlanQualificationTerminal => (
        isMcpManifoldPlanQualificationTerminal(value, request)
      ))
      await exited
      return terminal
    }

    const moduleName = 'x'.repeat(91)
    const exactBoundarySource = `module ${moduleName}(){cube(1);} ${moduleName}();`
    const exact = await run(exactBoundarySource, 1)
    expect(exact.status).toBe('succeeded')
    if (exact.status !== 'succeeded') throw new Error('expected plan exact-boundary success')
    expect(exact.result.meshes[0].entityId).toHaveLength(256)

    await expect(run('assert(true) if(true) let(x=2) cube(x);', 2)).resolves.toMatchObject({
      status: 'succeeded', result: {meshes:[expect.objectContaining({entityId:expect.stringMatching(/^entity:sha256:/)})]},
    })
    await expect(run('cube(1);', 3)).resolves.toMatchObject({
      status: 'succeeded',
      result: { volume: 1, surfaceArea: 6 },
    })
  }, 30_000)

  it('isolates every identity boundary in real disposable Node Worker realms', async () => {
    const entryUrl = new URL('./fixtures/browser-qualification.worker.ts', import.meta.url)
    const lane = new ManifoldPlanQualificationWorkerLane({
      workerFactory: () => new NodeWorkerAsWebWorker(spawnWebWorker(entryUrl, false)),
      startupTimeoutMs: 10_000,
      deadlineMs: 80,
      cancellationGraceMs: 0,
    })

    await expect(lane.evaluate('hang')).rejects.toMatchObject({
      code: 'E_MANIFOLD_PLAN_WORKER_DEADLINE',
      workerEpoch: 1,
    })
    expect(lane.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      activeWorkerStarted: false,
      lastTerminatedWorkerEpoch: 1,
      workersStarted: 1,
      workersTerminated: 1,
    })

    await expect(lane.evaluate('recovery')).resolves.toMatchObject({ meshes: [], quality: 'full' })
    for (const field of ['entityId', 'instanceId', 'operationId'] as const) {
      const exact = await lane.evaluate(`identity-${field}-256`)
      const mesh = exact.meshes[0]
      const identities = {
        entityId: mesh.entityId,
        instanceId: mesh.provenance[0]?.source?.instanceId,
        operationId: mesh.provenance[0]?.source?.operationId,
      }
      expect(identities[field]).toHaveLength(256)
      for (const other of ['entityId', 'instanceId', 'operationId'] as const) {
        if (other !== field) expect(identities[other]!.length).toBeLessThan(256)
      }

      await expect(lane.evaluate(`identity-${field}-257`)).rejects.toMatchObject({
        code: 'CONTROLLED_RESULT_UNPUBLISHABLE',
        message: 'Controlled result cannot cross the qualification Worker boundary',
      })
      const refusedEpoch = lane.snapshot().lastTerminatedWorkerEpoch
      await expect(lane.evaluate(`recovery-${field}`)).resolves.toMatchObject({ meshes: [] })
      expect(lane.snapshot().lastTerminatedWorkerEpoch).toBe(refusedEpoch + 1)
    }
    expect(lane.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      workersStarted: 11,
      workersTerminated: 11,
    })
  }, 30_000)
})
