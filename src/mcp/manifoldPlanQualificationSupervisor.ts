import { Worker } from 'node:worker_threads'
import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import { sha256Hex } from '../core/sha256'
import {
  MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY,
  MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION,
  isMcpManifoldPlanQualificationNodeTerminal,
  isMcpManifoldPlanQualificationRequest,
  isMcpManifoldPlanQualificationStarted,
  type McpManifoldPlanQualificationCancel,
  type McpManifoldPlanQualificationError,
  type McpManifoldPlanQualificationRequest,
  type McpManifoldPlanQualificationTerminal,
} from '../services/manifoldPlanQualificationProtocol'

export type McpManifoldPlanSupervisorErrorCode =
  | 'E_MCP_MANIFOLD_PLAN_BUSY'
  | 'E_MCP_MANIFOLD_PLAN_CANCELLED'
  | 'E_MCP_MANIFOLD_PLAN_STARTUP'
  | 'E_MCP_MANIFOLD_PLAN_DEADLINE'
  | 'E_MCP_MANIFOLD_PLAN_PROTOCOL'
  | 'E_MCP_MANIFOLD_PLAN_CHILD_CRASH'
  | 'E_MCP_MANIFOLD_PLAN_JOIN'
  | 'E_MCP_MANIFOLD_PLAN_QUARANTINED'

export class McpManifoldPlanSupervisorError extends Error {
  constructor(
    readonly code: McpManifoldPlanSupervisorErrorCode,
    message: string,
    readonly workerEpoch: number | null,
    options: { cause?: unknown } = {},
  ) {
    super(message, options)
    this.name = 'McpManifoldPlanSupervisorError'
  }
}

/** Error returned by the disposable child, reconstructed only after join. */
export class McpManifoldPlanRemoteError extends Error {
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

export interface McpManifoldPlanQualificationSupervisorOptions {
  readonly workerFactory?: (workerEpoch: number) => Worker
  readonly startupTimeoutMs?: number
  readonly deadlineMs?: number
  readonly cancellationGraceMs?: number
  readonly joinTimeoutMs?: number
}

export interface McpManifoldPlanQualificationRunOptions {
  readonly signal?: AbortSignal
  readonly startupTimeoutMs?: number
  readonly deadlineMs?: number
}

export interface McpManifoldPlanQualificationSupervisorSnapshot {
  readonly activeWorkerEpoch: number | null
  readonly lastJoinedWorkerEpoch: number
  readonly workersStarted: number
  readonly workersJoined: number
  readonly quarantined: boolean
}

const DEFAULT_STARTUP_TIMEOUT_MS = 5_000
const DEFAULT_DEADLINE_MS = 30_000
const DEFAULT_CANCELLATION_GRACE_MS = 25
const DEFAULT_JOIN_TIMEOUT_MS = 1_000
const MAX_STARTUP_TIMEOUT_MS = 30_000
const MAX_DEADLINE_MS = 120_000
const MAX_GRACE_MS = 1_000
const MAX_JOIN_TIMEOUT_MS = 10_000
const MAX_COUNTED_CHILD_OUTPUT_BYTES = 8 * 1024

function boundedDuration(value: number, label: string, maximum: number, allowZero = false): number {
  if (!Number.isSafeInteger(value) || value < (allowZero ? 0 : 1) || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${allowZero ? 0 : 1} and ${maximum}`)
  }
  return value
}

function defaultWorkerFactory(_workerEpoch: number): Worker {
  return new Worker(new URL('./manifoldPlanQualification.worker.ts', import.meta.url), {
    execArgv: ['--import', 'tsx'],
    stdout: true,
    stderr: true,
  })
}

function attachBoundedOutputDrain(stream: Worker['stdout']): () => void {
  // Keep the pipe flowing so a noisy child cannot deadlock on backpressure, but
  // retain no output bytes. The saturating counter is deliberately bounded and
  // exists only to keep this callback's own state finite.
  stream.unpipe()
  let countedBytes = 0
  const onData = (chunk: unknown) => {
    const byteLength = typeof chunk === 'string'
      ? Buffer.byteLength(chunk)
      : ArrayBuffer.isView(chunk) ? chunk.byteLength : 0
    countedBytes = Math.min(MAX_COUNTED_CHILD_OUTPUT_BYTES, countedBytes + byteLength)
  }
  stream.on('data', onData)
  return () => stream.off('data', onData)
}

type StopReason = 'cancelled' | 'startup' | 'deadline'

/**
 * Qualification-only MCP hard boundary. Every admitted evaluation owns a
 * disposable child realm. The parent accepts at most one correlated terminal,
 * terminates and joins the realm, and only then resolves/rejects the caller.
 * This class is deliberately not installed in the production provider registry.
 */
export class McpManifoldPlanQualificationSupervisor {
  private readonly workerFactory: (workerEpoch: number) => Worker
  private readonly defaultStartupTimeoutMs: number
  private readonly defaultDeadlineMs: number
  private readonly cancellationGraceMs: number
  private readonly joinTimeoutMs: number
  private nextEpoch = 1
  private nextJobId = 1
  private activeWorkerEpoch: number | null = null
  private lastJoinedWorkerEpoch = 0
  private workersStarted = 0
  private workersJoined = 0
  private quarantined = false

  constructor(options: McpManifoldPlanQualificationSupervisorOptions = {}) {
    this.workerFactory = options.workerFactory ?? defaultWorkerFactory
    this.defaultStartupTimeoutMs = boundedDuration(
      options.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT_MS,
      'startupTimeoutMs',
      MAX_STARTUP_TIMEOUT_MS,
    )
    this.defaultDeadlineMs = boundedDuration(
      options.deadlineMs ?? DEFAULT_DEADLINE_MS,
      'deadlineMs',
      MAX_DEADLINE_MS,
    )
    this.cancellationGraceMs = boundedDuration(
      options.cancellationGraceMs ?? DEFAULT_CANCELLATION_GRACE_MS,
      'cancellationGraceMs',
      MAX_GRACE_MS,
      true,
    )
    this.joinTimeoutMs = boundedDuration(
      options.joinTimeoutMs ?? DEFAULT_JOIN_TIMEOUT_MS,
      'joinTimeoutMs',
      MAX_JOIN_TIMEOUT_MS,
    )
  }

  snapshot(): McpManifoldPlanQualificationSupervisorSnapshot {
    return Object.freeze({
      activeWorkerEpoch: this.activeWorkerEpoch,
      lastJoinedWorkerEpoch: this.lastJoinedWorkerEpoch,
      workersStarted: this.workersStarted,
      workersJoined: this.workersJoined,
      quarantined: this.quarantined,
    })
  }

  async evaluate(
    source: string,
    quality: GeometryQuality = 'full',
    options: McpManifoldPlanQualificationRunOptions = {},
  ): Promise<GeometryEvaluationResult> {
    if (this.quarantined) {
      throw new McpManifoldPlanSupervisorError(
        'E_MCP_MANIFOLD_PLAN_QUARANTINED',
        'MCP Manifold qualification supervisor is quarantined after a failed child join',
        null,
      )
    }
    if (this.activeWorkerEpoch !== null) {
      throw new McpManifoldPlanSupervisorError(
        'E_MCP_MANIFOLD_PLAN_BUSY',
        'MCP Manifold qualification supervisor already has an active child',
        this.activeWorkerEpoch,
      )
    }
    const startupTimeoutMs = boundedDuration(
      options.startupTimeoutMs ?? this.defaultStartupTimeoutMs,
      'startupTimeoutMs',
      MAX_STARTUP_TIMEOUT_MS,
    )
    const deadlineMs = boundedDuration(
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
      throw new TypeError('Invalid MCP Manifold qualification request')
    }
    if (options.signal?.aborted) {
      throw new McpManifoldPlanSupervisorError(
        'E_MCP_MANIFOLD_PLAN_CANCELLED',
        'MCP Manifold qualification evaluation was cancelled before child admission',
        null,
      )
    }

    const worker = this.workerFactory(workerEpoch)
    const stopStdoutDrain = attachBoundedOutputDrain(worker.stdout)
    const stopStderrDrain = attachBoundedOutputDrain(worker.stderr)
    this.activeWorkerEpoch = workerEpoch
    this.workersStarted++

    return new Promise<GeometryEvaluationResult>((resolve, reject) => {
      let settling = false
      let started = false
      let stopReason: StopReason | null = null
      let startupTimer: ReturnType<typeof setTimeout> | undefined
      let deadlineTimer: ReturnType<typeof setTimeout> | undefined
      let graceTimer: ReturnType<typeof setTimeout> | undefined
      let outputDrainsStopped = false

      const stopOutputDrains = () => {
        if (outputDrainsStopped) return
        outputDrainsStopped = true
        stopStdoutDrain()
        stopStderrDrain()
      }

      const clearControl = () => {
        if (startupTimer !== undefined) clearTimeout(startupTimer)
        if (deadlineTimer !== undefined) clearTimeout(deadlineTimer)
        if (graceTimer !== undefined) clearTimeout(graceTimer)
        options.signal?.removeEventListener('abort', onAbort)
      }

      const terminateAndJoin = async (): Promise<void> => {
        type JoinOutcome = { kind: 'joined' } | { kind: 'failed'; error: unknown } | { kind: 'timeout' }
        let joinTimer: ReturnType<typeof setTimeout> | undefined
        const termination = Promise.resolve()
          .then(() => worker.terminate())
          .then<JoinOutcome, JoinOutcome>(
            () => ({ kind: 'joined' }),
            error => ({ kind: 'failed', error }),
          )
        const outcome = await Promise.race<JoinOutcome>([
          termination,
          new Promise(resolve => {
            joinTimer = setTimeout(() => resolve({ kind: 'timeout' }), this.joinTimeoutMs)
          }),
        ])
        if (joinTimer !== undefined) clearTimeout(joinTimer)

        if (outcome.kind !== 'joined') {
          this.quarantined = true
          throw new McpManifoldPlanSupervisorError(
            'E_MCP_MANIFOLD_PLAN_JOIN',
            outcome.kind === 'timeout'
              ? `MCP Manifold qualification child did not join within ${this.joinTimeoutMs} ms`
              : 'MCP Manifold qualification child could not be terminated and joined',
            workerEpoch,
            outcome.kind === 'failed' ? { cause: outcome.error } : {},
          )
        }
        stopOutputDrains()
        detachWorkerListeners()
        this.lastJoinedWorkerEpoch = workerEpoch
        this.workersJoined++
        if (this.activeWorkerEpoch === workerEpoch) this.activeWorkerEpoch = null
      }

      const settle = async (
        terminal: McpManifoldPlanQualificationTerminal | null,
        failure: McpManifoldPlanSupervisorError | null,
      ) => {
        if (settling) return
        settling = true
        clearControl()
        try {
          await terminateAndJoin()
        } catch (joinError) {
          reject(joinError)
          return
        }
        if (stopReason !== null) {
          if (stopReason === 'cancelled') {
            reject(new McpManifoldPlanSupervisorError(
              'E_MCP_MANIFOLD_PLAN_CANCELLED',
              'MCP Manifold qualification evaluation was cancelled',
              workerEpoch,
            ))
          } else if (stopReason === 'startup') {
            reject(new McpManifoldPlanSupervisorError(
              'E_MCP_MANIFOLD_PLAN_STARTUP',
              'MCP Manifold qualification child did not complete its started handshake',
              workerEpoch,
            ))
          } else {
            reject(new McpManifoldPlanSupervisorError(
              'E_MCP_MANIFOLD_PLAN_DEADLINE',
              'MCP Manifold qualification evaluation exceeded its deadline',
              workerEpoch,
            ))
          }
          return
        }
        if (failure !== null) {
          reject(failure)
          return
        }
        if (terminal?.status === 'failed') {
          reject(new McpManifoldPlanRemoteError(terminal.error))
          return
        }
        if (terminal?.status === 'succeeded') {
          resolve(terminal.result)
          return
        }
        reject(new McpManifoldPlanSupervisorError(
          'E_MCP_MANIFOLD_PLAN_PROTOCOL',
          'MCP Manifold qualification child settled without a terminal',
          workerEpoch,
        ))
      }

      const finishStop = () => {
        void settle(null, null)
      }

      const requestStop = (reason: StopReason) => {
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
        try {
          worker.postMessage(cancel)
        } catch {
          // A dead child is still joined below; cancellation/deadline remains
          // the already selected caller-visible terminal.
        }
        graceTimer = setTimeout(finishStop, this.cancellationGraceMs)
      }

      const onAbort = () => requestStop('cancelled')

      const onMessage = (value: unknown) => {
        if (settling) return
        if (stopReason !== null) {
          finishStop()
          return
        }
        if (!started) {
          if (!isMcpManifoldPlanQualificationStarted(value, request)) {
            void settle(null, new McpManifoldPlanSupervisorError(
              'E_MCP_MANIFOLD_PLAN_PROTOCOL',
              'MCP Manifold qualification child did not provide the exact started handshake',
              workerEpoch,
            ))
            return
          }
          started = true
          if (startupTimer !== undefined) {
            clearTimeout(startupTimer)
            startupTimer = undefined
          }
          deadlineTimer = setTimeout(() => requestStop('deadline'), deadlineMs)
          return
        }
        if (isMcpManifoldPlanQualificationStarted(value, request)) {
          void settle(null, new McpManifoldPlanSupervisorError(
            'E_MCP_MANIFOLD_PLAN_PROTOCOL',
            'MCP Manifold qualification child repeated its started handshake',
            workerEpoch,
          ))
          return
        }
        if (!isMcpManifoldPlanQualificationNodeTerminal(value, request)) {
          void settle(null, new McpManifoldPlanSupervisorError(
            'E_MCP_MANIFOLD_PLAN_PROTOCOL',
            'MCP Manifold qualification child returned an invalid or uncorrelated terminal',
            workerEpoch,
          ))
          return
        }
        void settle(value, null)
      }
      const onMessageError = (error: Error) => {
        if (settling) return
        void settle(null, new McpManifoldPlanSupervisorError(
          'E_MCP_MANIFOLD_PLAN_PROTOCOL',
          'MCP Manifold qualification child message could not be deserialized',
          workerEpoch,
          { cause: error },
        ))
      }
      const onError = (error: Error) => {
        if (settling) return
        if (stopReason !== null) {
          finishStop()
          return
        }
        void settle(null, new McpManifoldPlanSupervisorError(
          'E_MCP_MANIFOLD_PLAN_CHILD_CRASH',
          'MCP Manifold qualification child crashed before a valid terminal',
          workerEpoch,
          { cause: error },
        ))
      }
      const onExit = (code: number) => {
        if (settling) return
        if (stopReason !== null) {
          finishStop()
          return
        }
        void settle(null, new McpManifoldPlanSupervisorError(
          'E_MCP_MANIFOLD_PLAN_CHILD_CRASH',
          `MCP Manifold qualification child exited with code ${code} before a valid terminal`,
          workerEpoch,
        ))
      }
      const detachWorkerListeners = () => {
        worker.off('message', onMessage)
        worker.off('messageerror', onMessageError)
        worker.off('error', onError)
        worker.off('exit', onExit)
      }

      worker.on('message', onMessage)
      worker.on('messageerror', onMessageError)
      worker.on('error', onError)
      worker.on('exit', onExit)

      options.signal?.addEventListener('abort', onAbort, { once: true })
      if (options.signal?.aborted) {
        requestStop('cancelled')
        return
      }
      startupTimer = setTimeout(() => requestStop('startup'), startupTimeoutMs)
      try {
        worker.postMessage(request)
      } catch (error) {
        void settle(null, new McpManifoldPlanSupervisorError(
          'E_MCP_MANIFOLD_PLAN_PROTOCOL',
          'MCP Manifold qualification request could not be posted to its child',
          workerEpoch,
          { cause: error },
        ))
        return
      }
      if (settling) return
    })
  }
}
