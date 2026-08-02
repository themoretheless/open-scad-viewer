import { describe, expect, it } from 'vitest'
import {
  ManifoldPlanQualificationWorkerLane,
  type ManifoldPlanQualificationWorkerLike,
} from '../src/services/manifoldPlanQualificationWorkerLane'
import type { McpManifoldPlanQualificationRequest } from '../src/services/manifoldPlanQualificationProtocol'

class ScriptedWorker implements ManifoldPlanQualificationWorkerLike {
  readonly messages: unknown[] = []
  terminated = false
  private readonly messageListeners = new Set<(event: MessageEvent<unknown>) => void>()
  private readonly errorListeners = new Set<(event: ErrorEvent) => void>()
  private readonly messageErrorListeners = new Set<(event: MessageEvent<unknown>) => void>()

  postMessage(message: unknown): void {
    this.messages.push(message)
    if (message === null || typeof message !== 'object'
      || (message as { type?: unknown }).type !== 'evaluate') return
    const request = message as McpManifoldPlanQualificationRequest
    if (request.source === 'no-start') return
    queueMicrotask(() => this.emit({
      protocolVersion: request.protocolVersion,
      workerEpoch: request.workerEpoch,
      jobId: request.jobId,
      sourceSha256: request.sourceSha256,
      identity: request.identity,
      status: 'started',
    }))
    if (request.source === 'hang') return
    const identity = request.source === 'wrong-epoch'
      ? { ...request.identity, engineKey: 'forged' }
      : request.identity
    queueMicrotask(() => this.emit({
      protocolVersion: request.protocolVersion,
      workerEpoch: request.workerEpoch,
      jobId: request.jobId,
      sourceSha256: request.sourceSha256,
      identity,
      status: 'succeeded',
      result: {
        meshes: [],
        warnings: [],
        volume: 0,
        surfaceArea: 0,
        quality: request.quality,
        reduced: false,
        timings: { parseMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
      },
    }))
  }

  terminate(): void {
    this.terminated = true
    this.messageListeners.clear()
    this.errorListeners.clear()
    this.messageErrorListeners.clear()
  }

  addEventListener(type: 'message' | 'error' | 'messageerror', listener: ((event: MessageEvent<unknown>) => void)
    | ((event: ErrorEvent) => void)): void {
    if (type === 'message') this.messageListeners.add(listener as (event: MessageEvent<unknown>) => void)
    else if (type === 'error') this.errorListeners.add(listener as (event: ErrorEvent) => void)
    else this.messageErrorListeners.add(listener as (event: MessageEvent<unknown>) => void)
  }

  removeEventListener(type: 'message' | 'error' | 'messageerror', listener: ((event: MessageEvent<unknown>) => void)
    | ((event: ErrorEvent) => void)): void {
    if (type === 'message') this.messageListeners.delete(listener as (event: MessageEvent<unknown>) => void)
    else if (type === 'error') this.errorListeners.delete(listener as (event: ErrorEvent) => void)
    else this.messageErrorListeners.delete(listener as (event: MessageEvent<unknown>) => void)
  }

  private emit(data: unknown): void {
    if (this.terminated) return
    const event = { data } as MessageEvent<unknown>
    for (const listener of this.messageListeners) listener(event)
  }
}

describe('browser Manifold-plan qualification Worker lane', () => {
  it('terminates a wedged realm at deadline and admits recovery only in a fresh epoch', async () => {
    const workers: ScriptedWorker[] = []
    const lane = new ManifoldPlanQualificationWorkerLane({
      workerFactory: () => {
        const worker = new ScriptedWorker()
        workers.push(worker)
        return worker
      },
      deadlineMs: 20,
      cancellationGraceMs: 0,
    })

    await expect(lane.evaluate('hang')).rejects.toMatchObject({
      code: 'E_MANIFOLD_PLAN_WORKER_DEADLINE',
      workerEpoch: 1,
    })
    expect(workers[0].terminated).toBe(true)
    expect(lane.snapshot()).toEqual({
      activeWorkerEpoch: null,
      activeWorkerStarted: false,
      lastTerminatedWorkerEpoch: 1,
      workersStarted: 1,
      workersTerminated: 1,
    })

    await expect(lane.evaluate('success')).resolves.toMatchObject({ quality: 'full', meshes: [] })
    expect(workers).toHaveLength(2)
    expect(workers[1].terminated).toBe(true)
    expect(lane.snapshot().lastTerminatedWorkerEpoch).toBe(2)
  })

  it('separates startup timeout from the execution deadline', async () => {
    const lane = new ManifoldPlanQualificationWorkerLane({
      workerFactory: () => new ScriptedWorker(),
      startupTimeoutMs: 10,
      deadlineMs: 1_000,
      cancellationGraceMs: 0,
    })
    await expect(lane.evaluate('no-start')).rejects.toMatchObject({
      code: 'E_MANIFOLD_PLAN_WORKER_STARTUP',
      workerEpoch: 1,
    })
    expect(lane.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      activeWorkerStarted: false,
      workersTerminated: 1,
    })
  })

  it('chooses cancellation once, rejects concurrent admission, and never reuses the old realm', async () => {
    const workers: ScriptedWorker[] = []
    const lane = new ManifoldPlanQualificationWorkerLane({
      workerFactory: () => {
        const worker = new ScriptedWorker()
        workers.push(worker)
        return worker
      },
      deadlineMs: 1_000,
      cancellationGraceMs: 0,
    })
    const controller = new AbortController()
    const pending = lane.evaluate('hang', 'full', { signal: controller.signal })
    await expect(lane.evaluate('success')).rejects.toMatchObject({
      code: 'E_MANIFOLD_PLAN_WORKER_BUSY',
      workerEpoch: 1,
    })
    controller.abort()
    await expect(pending).rejects.toMatchObject({
      code: 'E_MANIFOLD_PLAN_WORKER_CANCELLED',
      workerEpoch: 1,
    })
    expect(workers).toHaveLength(1)
    expect(workers[0].messages).toEqual([
      expect.objectContaining({ type: 'evaluate', workerEpoch: 1 }),
      expect.objectContaining({ type: 'cancel', workerEpoch: 1, reason: 'cancelled' }),
    ])
    expect(workers[0].terminated).toBe(true)
  })

  it('fences an uncorrelated terminal and terminates its realm before rejecting', async () => {
    const workers: ScriptedWorker[] = []
    const lane = new ManifoldPlanQualificationWorkerLane({
      workerFactory: () => {
        const worker = new ScriptedWorker()
        workers.push(worker)
        return worker
      },
    })
    await expect(lane.evaluate('wrong-epoch')).rejects.toMatchObject({
      code: 'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
      workerEpoch: 1,
    })
    expect(workers[0].terminated).toBe(true)
    expect(lane.snapshot().activeWorkerEpoch).toBeNull()
  })
})
