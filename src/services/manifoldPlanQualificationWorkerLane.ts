import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import { sha256Hex } from '../core/sha256'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationRequest,
  isMcpManifoldPlanQualificationStarted,
  isMcpManifoldPlanQualificationTerminal,
  type McpManifoldPlanQualificationCancel,
  type McpManifoldPlanQualificationError,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationTerminal,
} from './manifoldPlanQualificationProtocol'

export interface ManifoldPlanQualificationWorkerLike {
  postMessage(message: unknown): void
  terminate(): void
  addEventListener(type: 'message', listener: (event: MessageEvent<unknown>) => void): void
  addEventListener(type: 'error', listener: (event: ErrorEvent) => void): void
  addEventListener(type: 'messageerror', listener: (event: MessageEvent<unknown>) => void): void
  removeEventListener(type: 'message', listener: (event: MessageEvent<unknown>) => void): void
  removeEventListener(type: 'error', listener: (event: ErrorEvent) => void): void
  removeEventListener(type: 'messageerror', listener: (event: MessageEvent<unknown>) => void): void
}

export type ManifoldPlanQualificationWorkerLaneErrorCode =
  | 'E_MANIFOLD_PLAN_WORKER_BUSY'
  | 'E_MANIFOLD_PLAN_WORKER_CANCELLED'
  | 'E_MANIFOLD_PLAN_WORKER_STARTUP'
  | 'E_MANIFOLD_PLAN_WORKER_DEADLINE'
  | 'E_MANIFOLD_PLAN_WORKER_PROTOCOL'
  | 'E_MANIFOLD_PLAN_WORKER_CRASH'

export class ManifoldPlanQualificationWorkerLaneError extends Error {
  constructor(
    readonly code: ManifoldPlanQualificationWorkerLaneErrorCode,
    message: string,
    readonly workerEpoch: number | null,
    options: { cause?: unknown } = {},
  ) {
    super(message, options)
    this.name = 'ManifoldPlanQualificationWorkerLaneError'
  }
}

export class ManifoldPlanQualificationWorkerRemoteError extends Error {
  readonly code: string | undefined
  readonly line: number | undefined
  readonly column: number | undefined
  readonly start: number | undefined
  readonly end: number | undefined

  constructor(error: McpManifoldPlanQualificationError) {
    super(error.message)
    this.name = error.name
    this.code = error.code ?? undefined
    this.line = error.line ?? undefined
    this.column = error.column ?? undefined
    this.start = error.start ?? undefined
    this.end = error.end ?? undefined
  }
}

export interface ManifoldPlanQualificationWorkerLaneOptions {
  readonly workerFactory?: (workerEpoch: number) => ManifoldPlanQualificationWorkerLike
  readonly startupTimeoutMs?: number
  readonly deadlineMs?: number
  readonly cancellationGraceMs?: number
}

export interface ManifoldPlanQualificationWorkerLaneSnapshot {
  readonly activeWorkerEpoch: number | null
  readonly activeWorkerStarted: boolean
  readonly lastTerminatedWorkerEpoch: number
  readonly workersStarted: number
  readonly workersTerminated: number
}

const MAX_DEADLINE_MS = 120_000
const MAX_GRACE_MS = 1_000

function duration(value: number, label: string, maximum: number, allowZero = false): number {
  if (!Number.isSafeInteger(value) || value < (allowZero ? 0 : 1) || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${allowZero ? 0 : 1} and ${maximum}`)
  }
  return value
}

function defaultWorkerFactory(_workerEpoch: number): ManifoldPlanQualificationWorkerLike {
  return new Worker(new URL('../workers/manifoldPlanQualification.worker.ts', import.meta.url), {
    type: 'module',
  })
}

/**
 * Qualification-only browser shadow lane. It never publishes to the scene or
 * production registry: each result is correlated, validated, then its Worker
 * realm is terminated before the caller observes settlement.
 */
export class ManifoldPlanQualificationWorkerLane {
  private readonly workerFactory: (workerEpoch: number) => ManifoldPlanQualificationWorkerLike
  private readonly startupTimeoutMs: number
  private readonly defaultDeadlineMs: number
  private readonly cancellationGraceMs: number
  private nextEpoch = 1
  private nextJobId = 1
  private activeWorkerEpoch: number | null = null
  private activeWorkerStarted = false
  private lastTerminatedWorkerEpoch = 0
  private workersStarted = 0
  private workersTerminated = 0

  constructor(options: ManifoldPlanQualificationWorkerLaneOptions = {}) {
    this.workerFactory = options.workerFactory ?? defaultWorkerFactory
    this.startupTimeoutMs = duration(
      options.startupTimeoutMs ?? 10_000,
      'startupTimeoutMs',
      MAX_DEADLINE_MS,
    )
    this.defaultDeadlineMs = duration(options.deadlineMs ?? 30_000, 'deadlineMs', MAX_DEADLINE_MS)
    this.cancellationGraceMs = duration(
      options.cancellationGraceMs ?? 25,
      'cancellationGraceMs',
      MAX_GRACE_MS,
      true,
    )
  }

  snapshot(): ManifoldPlanQualificationWorkerLaneSnapshot {
    return Object.freeze({
      activeWorkerEpoch: this.activeWorkerEpoch,
      activeWorkerStarted: this.activeWorkerStarted,
      lastTerminatedWorkerEpoch: this.lastTerminatedWorkerEpoch,
      workersStarted: this.workersStarted,
      workersTerminated: this.workersTerminated,
    })
  }

  async evaluate(
    source: string,
    quality: GeometryQuality = 'full',
    options: { signal?: AbortSignal; deadlineMs?: number } = {},
  ): Promise<GeometryEvaluationResult> {
    if (this.activeWorkerEpoch !== null) {
      throw new ManifoldPlanQualificationWorkerLaneError(
        'E_MANIFOLD_PLAN_WORKER_BUSY',
        'Manifold plan qualification Worker lane already has an active realm',
        this.activeWorkerEpoch,
      )
    }
    const deadlineMs = duration(
      options.deadlineMs ?? this.defaultDeadlineMs,
      'deadlineMs',
      MAX_DEADLINE_MS,
    )
    const workerEpoch = this.nextEpoch++
    const jobId = this.nextJobId++
    const request: McpManifoldPlanQualificationRequest = Object.freeze({
      protocolVersion: MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
      type: 'evaluate',
      workerEpoch,
      jobId,
      source,
      sourceSha256: sha256Hex(source),
      quality,
      identity: MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
    })
    if (!isMcpManifoldPlanQualificationRequest(request)) {
      throw new TypeError('Invalid Manifold plan qualification Worker request')
    }
    if (options.signal?.aborted) {
      throw new ManifoldPlanQualificationWorkerLaneError(
        'E_MANIFOLD_PLAN_WORKER_CANCELLED',
        'Manifold plan qualification Worker was cancelled before admission',
        null,
      )
    }

    const worker = this.workerFactory(workerEpoch)
    this.activeWorkerEpoch = workerEpoch
    this.activeWorkerStarted = false
    this.workersStarted++

    return new Promise<GeometryEvaluationResult>((resolve, reject) => {
      let settling = false
      let started = false
      let stopReason: 'cancelled' | 'startup' | 'deadline' | null = null
      let startupTimer: ReturnType<typeof setTimeout> | undefined
      let deadlineTimer: ReturnType<typeof setTimeout> | undefined
      let graceTimer: ReturnType<typeof setTimeout> | undefined

      const terminate = () => {
        worker.terminate()
        this.lastTerminatedWorkerEpoch = workerEpoch
        this.workersTerminated++
        if (this.activeWorkerEpoch === workerEpoch) {
          this.activeWorkerEpoch = null
          this.activeWorkerStarted = false
        }
      }
      const cleanup = () => {
        if (startupTimer !== undefined) clearTimeout(startupTimer)
        if (deadlineTimer !== undefined) clearTimeout(deadlineTimer)
        if (graceTimer !== undefined) clearTimeout(graceTimer)
        options.signal?.removeEventListener('abort', onAbort)
        worker.removeEventListener('message', onMessage)
        worker.removeEventListener('error', onError)
        worker.removeEventListener('messageerror', onMessageError)
      }
      const settle = (
        terminal: McpManifoldPlanQualificationTerminal | null,
        failure: ManifoldPlanQualificationWorkerLaneError | null,
      ) => {
        if (settling) return
        settling = true
        cleanup()
        terminate()
        if (stopReason !== null) {
          reject(new ManifoldPlanQualificationWorkerLaneError(
            stopReason === 'cancelled'
              ? 'E_MANIFOLD_PLAN_WORKER_CANCELLED'
              : stopReason === 'startup'
                ? 'E_MANIFOLD_PLAN_WORKER_STARTUP'
                : 'E_MANIFOLD_PLAN_WORKER_DEADLINE',
            stopReason === 'cancelled'
              ? 'Manifold plan qualification Worker evaluation was cancelled'
              : stopReason === 'startup'
                ? 'Manifold plan qualification Worker did not acknowledge startup in time'
                : 'Manifold plan qualification Worker evaluation exceeded its deadline',
            workerEpoch,
          ))
        } else if (failure !== null) {
          reject(failure)
        } else if (terminal?.status === 'failed') {
          reject(new ManifoldPlanQualificationWorkerRemoteError(terminal.error))
        } else if (terminal?.status === 'succeeded') {
          resolve(terminal.result)
        } else {
          reject(new ManifoldPlanQualificationWorkerLaneError(
            'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
            'Manifold plan qualification Worker settled without a terminal',
            workerEpoch,
          ))
        }
      }
      const requestStop = (reason: 'cancelled' | 'startup' | 'deadline') => {
        if (settling || stopReason !== null) return
        stopReason = reason
        const cancel: McpManifoldPlanQualificationCancel = Object.freeze({
          protocolVersion: MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
          type: 'cancel',
          workerEpoch,
          jobId,
          sourceSha256: request.sourceSha256,
          reason: reason === 'cancelled' ? 'cancelled' : 'deadline',
          identity: MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
        })
        try { worker.postMessage(cancel) } catch { /* terminate below */ }
        graceTimer = setTimeout(() => settle(null, null), this.cancellationGraceMs)
      }
      const onAbort = () => requestStop('cancelled')
      const onMessage = (event: MessageEvent<unknown>) => {
        if (settling) return
        if (stopReason !== null) {
          settle(null, null)
        } else if (isMcpManifoldPlanQualificationStarted(event.data, request)) {
          if (started) {
            settle(null, new ManifoldPlanQualificationWorkerLaneError(
              'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
              'Manifold plan qualification Worker acknowledged startup more than once',
              workerEpoch,
            ))
            return
          }
          started = true
          this.activeWorkerStarted = true
          if (startupTimer !== undefined) clearTimeout(startupTimer)
          deadlineTimer = setTimeout(() => requestStop('deadline'), deadlineMs)
        } else if (isMcpManifoldPlanQualificationTerminal(event.data, request)) {
          if (!started) {
            settle(null, new ManifoldPlanQualificationWorkerLaneError(
              'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
              'Manifold plan qualification Worker returned a terminal before startup acknowledgement',
              workerEpoch,
            ))
          } else {
            settle(event.data, null)
          }
        } else {
          settle(null, new ManifoldPlanQualificationWorkerLaneError(
            'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
            'Manifold plan qualification Worker returned an invalid or uncorrelated terminal',
            workerEpoch,
          ))
        }
      }
      const onError = (event: ErrorEvent) => {
        if (settling) return
        settle(null, new ManifoldPlanQualificationWorkerLaneError(
          'E_MANIFOLD_PLAN_WORKER_CRASH',
          'Manifold plan qualification Worker crashed before a valid terminal',
          workerEpoch,
          { cause: event.error },
        ))
      }
      const onMessageError = (event: MessageEvent<unknown>) => {
        if (settling) return
        settle(null, new ManifoldPlanQualificationWorkerLaneError(
          'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
          'Manifold plan qualification Worker message could not be deserialized',
          workerEpoch,
          { cause: event.data },
        ))
      }

      worker.addEventListener('message', onMessage)
      worker.addEventListener('error', onError)
      worker.addEventListener('messageerror', onMessageError)
      options.signal?.addEventListener('abort', onAbort, { once: true })
      if (options.signal?.aborted) {
        requestStop('cancelled')
        return
      }
      startupTimer = setTimeout(() => requestStop('startup'), this.startupTimeoutMs)
      try {
        worker.postMessage(request)
      } catch (error) {
        settle(null, new ManifoldPlanQualificationWorkerLaneError(
          'E_MANIFOLD_PLAN_WORKER_PROTOCOL',
          'Manifold plan qualification request could not be posted',
          workerEpoch,
          { cause: error },
        ))
      }
    })
  }
}
