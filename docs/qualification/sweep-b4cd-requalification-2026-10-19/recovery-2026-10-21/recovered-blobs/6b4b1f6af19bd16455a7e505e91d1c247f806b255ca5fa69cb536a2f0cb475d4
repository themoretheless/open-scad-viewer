import { Worker } from 'node:worker_threads'
import type { GeometryQuality } from '../core/build'
import { sha256Hex } from '../core/sha256'
import {
  planGeometrySourceExecution,
  type GeometryBuildPurpose,
  type GeometryEngineRegistrySnapshot,
} from '../core/geometryExecution'
import {
  type GeometryBuildResult,
} from '../services/geometryBuildEngine'
import type { McpGeometryBuildRuntime } from './geometryService'
import { GeometryBusyError, GeometryDeadlineExceededError } from './geometryService'
import {
  DIRECT_GEOMETRY_IDENTITY,
  DIRECT_GEOMETRY_PROTOCOL_VERSION,
  isDirectGeometryNodeTerminal,
  isDirectGeometryRequest,
  isDirectGeometryStarted,
  reconstructDirectGeometryRemoteError,
  type DirectGeometryCancel,
  type DirectGeometryRequest,
  type DirectGeometryTerminal,
} from './directGeometryProtocol'

export const DIRECT_GEOMETRY_MAX_ADMITTED_JOBS = 8
export const DIRECT_GEOMETRY_JOB_DEADLINE_MS = 30_000
export const DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS = 10_000
export const DIRECT_GEOMETRY_CANCEL_GRACE_MS = 25
export const DIRECT_GEOMETRY_JOIN_TIMEOUT_MS = 5_000

const MAX_STARTUP_TIMEOUT_MS = 30_000
const MAX_JOB_DEADLINE_MS = 120_000
const MAX_CANCEL_GRACE_MS = 1_000
const MAX_JOIN_TIMEOUT_MS = 10_000
const MAX_DRAINED_CHILD_OUTPUT_BYTES = 8 * 1024

export type DirectGeometrySupervisorErrorCode =
  | 'E_DIRECT_GEOMETRY_STARTUP'
  | 'E_DIRECT_GEOMETRY_CRASH'
  | 'E_DIRECT_GEOMETRY_PROTOCOL'
  | 'E_DIRECT_GEOMETRY_JOIN'
  | 'E_DIRECT_GEOMETRY_QUARANTINED'
  | 'E_DIRECT_GEOMETRY_CLOSED'

export class DirectGeometrySupervisorError extends Error {
  constructor(
    readonly code: DirectGeometrySupervisorErrorCode,
    message: string,
    readonly workerEpoch: number | null,
    options: { cause?: unknown } = {},
  ) {
    super(message, options)
    this.name = 'DirectGeometrySupervisorError'
  }
}

export class DirectGeometryStartupError extends DirectGeometrySupervisorError {
  constructor(workerEpoch: number | null, options: { cause?: unknown } = {}) {
    super(
      'E_DIRECT_GEOMETRY_STARTUP',
      'The disposable geometry worker did not complete its startup handshake.',
      workerEpoch,
      options,
    )
    this.name = 'DirectGeometryStartupError'
  }
}

export class DirectGeometryWorkerCrashError extends DirectGeometrySupervisorError {
  constructor(workerEpoch: number, options: { cause?: unknown } = {}) {
    super(
      'E_DIRECT_GEOMETRY_CRASH',
      'The disposable geometry worker exited before returning a valid terminal.',
      workerEpoch,
      options,
    )
    this.name = 'DirectGeometryWorkerCrashError'
  }
}

export class DirectGeometryProtocolError extends DirectGeometrySupervisorError {
  constructor(workerEpoch: number, message = 'The disposable geometry worker violated its protocol.') {
    super('E_DIRECT_GEOMETRY_PROTOCOL', message, workerEpoch)
    this.name = 'DirectGeometryProtocolError'
  }
}

export class DirectGeometryJoinError extends DirectGeometrySupervisorError {
  constructor(workerEpoch: number, joinTimeoutMs: number, options: { cause?: unknown } = {}) {
    super(
      'E_DIRECT_GEOMETRY_JOIN',
      `The disposable geometry worker did not terminate and join within ${joinTimeoutMs} ms.`,
      workerEpoch,
      options,
    )
    this.name = 'DirectGeometryJoinError'
  }
}

export class DirectGeometryQuarantinedError extends DirectGeometrySupervisorError {
  constructor(workerEpoch: number | null, options: { cause?: unknown } = {}) {
    super(
      'E_DIRECT_GEOMETRY_QUARANTINED',
      'The direct geometry supervisor is quarantined after an unjoined worker.',
      workerEpoch,
      options,
    )
    this.name = 'DirectGeometryQuarantinedError'
  }
}

export class DirectGeometryClosedError extends DirectGeometrySupervisorError {
  constructor() {
    super('E_DIRECT_GEOMETRY_CLOSED', 'The direct geometry supervisor is closed.', null)
    this.name = 'DirectGeometryClosedError'
  }
}

export interface DirectGeometrySupervisorOptions {
  readonly workerFactory?: (workerEpoch: number) => Worker
  readonly maxAdmittedJobs?: number
  readonly jobDeadlineMs?: number
  readonly startupTimeoutMs?: number
  readonly cancelGraceMs?: number
  readonly joinTimeoutMs?: number
}

export interface DirectGeometrySupervisorSnapshot {
  readonly activeWorkerEpoch: number | null
  readonly queuedJobs: number
  readonly admittedJobs: number
  readonly lastJoinedWorkerEpoch: number
  readonly workersStarted: number
  readonly workersJoined: number
  readonly quarantined: boolean
  readonly closing: boolean
  readonly closed: boolean
}

type JobState = 'queued' | 'active' | 'terminal'
type StopReason = 'cancelled' | 'deadline' | 'startup' | 'closed'

interface AdmittedJob {
  readonly id: number
  readonly kind: 'build' | 'capabilities'
  readonly source: string | null
  readonly quality: GeometryQuality | null
  readonly purpose: GeometryBuildPurpose | null
  readonly signal: AbortSignal | undefined
  readonly resolve: (value: unknown) => void
  readonly reject: (reason: unknown) => void
  state: JobState
  deadlineTimer: ReturnType<typeof setTimeout> | undefined
  abortListener: (() => void) | undefined
}

interface ActiveJob {
  readonly job: AdmittedJob
  readonly request: DirectGeometryRequest
  readonly worker: Worker
  readonly epoch: number
  readonly done: Promise<void>
  readonly resolveDone: () => void
  readonly stopOutputDrains: () => void
  detachWorkerListeners: () => void
  started: boolean
  settling: boolean
  stopReason: StopReason | null
  startupTimer: ReturnType<typeof setTimeout> | undefined
  graceTimer: ReturnType<typeof setTimeout> | undefined
}

function boundedInteger(
  value: number,
  label: string,
  minimum: number,
  maximum: number,
): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}

function defaultWorkerFactory(_workerEpoch: number): Worker {
  return new Worker(new URL('./directGeometry.worker.mjs', import.meta.url), {
    // Geometry source never needs host credentials or configuration. An empty
    // environment keeps accidental dependency reads from inheriting secrets.
    env: {},
    stdout: true,
    stderr: true,
    resourceLimits: {
      maxOldGenerationSizeMb: 256,
      maxYoungGenerationSizeMb: 32,
      stackSizeMb: 8,
    },
  })
}

function attachBoundedOutputDrain(stream: Worker['stdout']): () => void {
  let countedBytes = 0
  const onData = (chunk: unknown) => {
    const byteLength = typeof chunk === 'string'
      ? Buffer.byteLength(chunk)
      : ArrayBuffer.isView(chunk) ? chunk.byteLength : 0
    countedBytes = Math.min(MAX_DRAINED_CHILD_OUTPUT_BYTES, countedBytes + byteLength)
  }
  try {
    stream.unpipe()
    stream.on('data', onData)
    stream.resume()
    return () => {
      try {
        stream.off('data', onData)
      } catch {
        // A joined child owns no further output; cleanup is best effort.
      }
    }
  } catch {
    // A real Worker pipe supports the operations above. Keep an injected test
    // double from violating the more important terminate-and-join invariant.
    try { stream.resume() } catch { /* no-op */ }
    return () => undefined
  }
}

function abortError(message: string): DOMException {
  return new DOMException(message, 'AbortError')
}

/**
 * Production MCP hard boundary for the legacy direct evaluator. Every job owns
 * a disposable Node Worker. A Worker terminal is published only after join and
 * no subsequent job starts before join. If join itself times out, the caller
 * receives only the supervisor failure while the realm is unref'ed and the
 * permanently quarantined supervisor admits no more work.
 */
export class DirectGeometrySupervisor implements McpGeometryBuildRuntime {
  private readonly workerFactory: (workerEpoch: number) => Worker
  private readonly maxAdmittedJobs: number
  private readonly jobDeadlineMs: number
  private readonly startupTimeoutMs: number
  private readonly cancelGraceMs: number
  private readonly joinTimeoutMs: number
  private readonly queue: AdmittedJob[] = []
  private active: ActiveJob | null = null
  private nextJobId = 1
  private nextWorkerEpoch = 1
  private lastJoinedWorkerEpoch = 0
  private workersStarted = 0
  private workersJoined = 0
  private quarantined = false
  private quarantineEpoch: number | null = null
  private closing = false
  private closed = false
  private closePromise: Promise<void> | null = null
  private lastJoinFailure: DirectGeometryJoinError | null = null

  constructor(options: DirectGeometrySupervisorOptions = {}) {
    this.workerFactory = options.workerFactory ?? defaultWorkerFactory
    this.maxAdmittedJobs = boundedInteger(
      options.maxAdmittedJobs ?? DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
      'maxAdmittedJobs',
      1,
      DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
    )
    this.jobDeadlineMs = boundedInteger(
      options.jobDeadlineMs ?? DIRECT_GEOMETRY_JOB_DEADLINE_MS,
      'jobDeadlineMs',
      1,
      MAX_JOB_DEADLINE_MS,
    )
    this.startupTimeoutMs = boundedInteger(
      options.startupTimeoutMs ?? DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS,
      'startupTimeoutMs',
      1,
      MAX_STARTUP_TIMEOUT_MS,
    )
    this.cancelGraceMs = boundedInteger(
      options.cancelGraceMs ?? DIRECT_GEOMETRY_CANCEL_GRACE_MS,
      'cancelGraceMs',
      0,
      MAX_CANCEL_GRACE_MS,
    )
    this.joinTimeoutMs = boundedInteger(
      options.joinTimeoutMs ?? DIRECT_GEOMETRY_JOIN_TIMEOUT_MS,
      'joinTimeoutMs',
      1,
      MAX_JOIN_TIMEOUT_MS,
    )
  }

  snapshot(): DirectGeometrySupervisorSnapshot {
    return Object.freeze({
      activeWorkerEpoch: this.active?.epoch ?? null,
      queuedJobs: this.queue.length,
      admittedJobs: this.queue.length + (this.active === null ? 0 : 1),
      lastJoinedWorkerEpoch: this.lastJoinedWorkerEpoch,
      workersStarted: this.workersStarted,
      workersJoined: this.workersJoined,
      quarantined: this.quarantined,
      closing: this.closing,
      closed: this.closed,
    })
  }

  build(
    source: string,
    quality: GeometryQuality,
    purpose: GeometryBuildPurpose,
    signal?: AbortSignal,
  ): Promise<GeometryBuildResult> {
    if (signal?.aborted) {
      return Promise.reject(abortError('Geometry evaluation was cancelled before admission'))
    }
    try {
      if (typeof source !== 'string') throw new TypeError('Geometry source must be a string')
      if (quality !== 'preview' && quality !== 'full') {
        throw new TypeError('Geometry quality must be preview or full')
      }
      if (purpose !== 'preview' && purpose !== 'full'
        && purpose !== 'analysis' && purpose !== 'export') {
        throw new TypeError('Geometry purpose is invalid')
      }
      // Bound and validate the source before admission and SHA-256. Production
      // HeadlessGeometryService already plans the same source, but the runtime
      // is exported and must remain safe when called directly.
      planGeometrySourceExecution(source, { quality, purpose })
    } catch (error) {
      return Promise.reject(error)
    }
    return this.admit<GeometryBuildResult>({
      kind: 'build',
      source,
      quality,
      purpose,
      signal,
    })
  }

  capabilities(): Promise<GeometryEngineRegistrySnapshot> {
    return this.admit<GeometryEngineRegistrySnapshot>({
      kind: 'capabilities',
      source: null,
      quality: null,
      purpose: null,
      signal: undefined,
    })
  }

  close(): Promise<void> {
    this.closePromise ??= (async () => {
      this.closing = true
      const queued = this.queue.splice(0)
      for (const job of queued) {
        this.finishJob(job, { error: abortError('Geometry evaluation cancelled during shutdown') })
      }
      const active = this.active
      if (active !== null) {
        this.requestStop(active, 'closed')
        await active.done
      }
      this.closed = true
      this.closing = false
      if (this.lastJoinFailure !== null) throw this.lastJoinFailure
    })()
    return this.closePromise
  }

  private admit<T>(input: {
    kind: AdmittedJob['kind']
    source: string | null
    quality: GeometryQuality | null
    purpose: GeometryBuildPurpose | null
    signal: AbortSignal | undefined
  }): Promise<T> {
    if (input.signal?.aborted) {
      return Promise.reject(abortError('Geometry evaluation was cancelled before admission'))
    }
    if (this.quarantined) {
      return Promise.reject(new DirectGeometryQuarantinedError(this.quarantineEpoch))
    }
    if (this.closing || this.closed) return Promise.reject(new DirectGeometryClosedError())
    if (this.queue.length + (this.active === null ? 0 : 1) >= this.maxAdmittedJobs) {
      return Promise.reject(new GeometryBusyError())
    }

    return new Promise<T>((resolve, reject) => {
      const job: AdmittedJob = {
        id: this.nextJobId++,
        kind: input.kind,
        source: input.source,
        quality: input.quality,
        purpose: input.purpose,
        signal: input.signal,
        resolve: value => resolve(value as T),
        reject,
        state: 'queued',
        deadlineTimer: undefined,
        abortListener: undefined,
      }
      job.deadlineTimer = setTimeout(() => this.onJobDeadline(job), this.jobDeadlineMs)
      if (job.signal !== undefined) {
        job.abortListener = () => this.onJobAbort(job)
        job.signal.addEventListener('abort', job.abortListener, { once: true })
        if (job.signal.aborted) {
          this.finishJob(job, { error: abortError('Geometry evaluation was cancelled before admission') })
          return
        }
      }
      this.queue.push(job)
      this.pump()
    })
  }

  private pump(): void {
    if (this.active !== null || this.closing || this.closed || this.quarantined) return
    const job = this.queue.shift()
    if (job === undefined || job.state !== 'queued') return
    job.state = 'active'
    const epoch = this.nextWorkerEpoch++
    const request = this.requestFor(job, epoch)
    if (!isDirectGeometryRequest(request)) {
      this.finishJob(job, {
        error: new DirectGeometryProtocolError(epoch, 'The parent constructed an invalid direct geometry request.'),
      })
      queueMicrotask(() => this.pump())
      return
    }

    let worker: Worker
    try {
      worker = this.workerFactory(epoch)
    } catch (cause) {
      this.finishJob(job, { error: new DirectGeometryStartupError(epoch, { cause }) })
      queueMicrotask(() => this.pump())
      return
    }

    let resolveDone!: () => void
    const done = new Promise<void>(resolve => { resolveDone = resolve })
    const stopStdout = attachBoundedOutputDrain(worker.stdout)
    const stopStderr = attachBoundedOutputDrain(worker.stderr)
    const stopOutputDrains = () => {
      stopStdout()
      stopStderr()
    }

    const active: ActiveJob = {
      job,
      request,
      worker,
      epoch,
      done,
      resolveDone,
      stopOutputDrains,
      detachWorkerListeners: () => undefined,
      started: false,
      settling: false,
      stopReason: null,
      startupTimer: undefined,
      graceTimer: undefined,
    }
    this.active = active
    this.workersStarted++

    const onMessage = (value: unknown) => this.onWorkerMessage(active, value)
    const onMessageError = (cause: Error) => {
      void this.settleActive(
        active,
        null,
        new DirectGeometryProtocolError(epoch, `Worker message deserialization failed: ${cause.name}`),
      )
    }
    const onError = (cause: Error) => {
      void this.settleActive(active, null, new DirectGeometryWorkerCrashError(epoch, { cause }))
    }
    const onExit = (_code: number) => {
      void this.settleActive(active, null, new DirectGeometryWorkerCrashError(epoch))
    }
    active.detachWorkerListeners = () => {
      worker.off('message', onMessage)
      worker.off('messageerror', onMessageError)
      worker.off('error', onError)
      worker.off('exit', onExit)
    }
    worker.on('message', onMessage)
    worker.on('messageerror', onMessageError)
    worker.on('error', onError)
    worker.on('exit', onExit)
    active.startupTimer = setTimeout(() => this.requestStop(active, 'startup'), this.startupTimeoutMs)

    try {
      worker.postMessage(request)
    } catch (cause) {
      void this.settleActive(active, null, new DirectGeometryStartupError(epoch, { cause }))
    }
  }

  private requestFor(job: AdmittedJob, workerEpoch: number): DirectGeometryRequest {
    const base = {
      protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
      workerEpoch,
      jobId: job.id,
      identity: DIRECT_GEOMETRY_IDENTITY,
    } as const
    return job.kind === 'build'
      ? Object.freeze({
          ...base,
          type: 'build' as const,
          source: job.source!,
          sourceSha256: sha256Hex(job.source!),
          quality: job.quality!,
          purpose: job.purpose!,
        })
      : Object.freeze({
          ...base,
          type: 'capabilities' as const,
          sourceSha256: null,
          quality: null,
          purpose: null,
        })
  }

  private onWorkerMessage(active: ActiveJob, value: unknown): void {
    if (this.active !== active || active.settling) return
    if (active.stopReason !== null) {
      void this.settleActive(active, null, null)
      return
    }
    if (!active.started) {
      if (!isDirectGeometryStarted(value, active.request)) {
        void this.settleActive(
          active,
          null,
          new DirectGeometryProtocolError(active.epoch, 'Worker did not return its exact started handshake.'),
        )
        return
      }
      active.started = true
      if (active.startupTimer !== undefined) {
        clearTimeout(active.startupTimer)
        active.startupTimer = undefined
      }
      return
    }
    if (!isDirectGeometryNodeTerminal(value, active.request)) {
      void this.settleActive(
        active,
        null,
        new DirectGeometryProtocolError(active.epoch, 'Worker returned an invalid or uncorrelated terminal.'),
      )
      return
    }
    void this.settleActive(active, value, null)
  }

  private onJobAbort(job: AdmittedJob): void {
    if (job.state === 'terminal') return
    if (job.state === 'queued') {
      const index = this.queue.indexOf(job)
      if (index >= 0) this.queue.splice(index, 1)
      this.finishJob(job, { error: abortError('Geometry evaluation was cancelled') })
      return
    }
    const active = this.active
    if (active?.job === job) this.requestStop(active, 'cancelled')
  }

  private onJobDeadline(job: AdmittedJob): void {
    if (job.state === 'terminal') return
    if (job.state === 'queued') {
      const index = this.queue.indexOf(job)
      if (index >= 0) this.queue.splice(index, 1)
      this.finishJob(job, { error: new GeometryDeadlineExceededError(this.jobDeadlineMs) })
      return
    }
    const active = this.active
    if (active?.job === job) this.requestStop(active, 'deadline')
  }

  private requestStop(active: ActiveJob, reason: StopReason): void {
    if (this.active !== active || active.settling || active.stopReason !== null) return
    active.stopReason = reason
    const cancel: DirectGeometryCancel = Object.freeze({
      protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
      type: 'cancel',
      workerEpoch: active.request.workerEpoch,
      jobId: active.request.jobId,
      sourceSha256: active.request.sourceSha256,
      quality: active.request.quality,
      purpose: active.request.purpose,
      reason: reason === 'closed' ? 'closed' : reason === 'cancelled' ? 'cancelled' : 'deadline',
      identity: DIRECT_GEOMETRY_IDENTITY,
    })
    try {
      active.worker.postMessage(cancel)
    } catch {
      // The child is still hard-terminated and joined below.
    }
    active.graceTimer = setTimeout(() => {
      void this.settleActive(active, null, null)
    }, this.cancelGraceMs)
  }

  private async settleActive(
    active: ActiveJob,
    terminal: DirectGeometryTerminal | null,
    failure: Error | null,
  ): Promise<void> {
    if (this.active !== active || active.settling) return
    active.settling = true
    if (active.startupTimer !== undefined) clearTimeout(active.startupTimer)
    if (active.graceTimer !== undefined) clearTimeout(active.graceTimer)

    const joinFailure = await this.terminateAndJoin(active)
    this.active = null

    if (joinFailure !== null) {
      this.quarantined = true
      this.quarantineEpoch = active.epoch
      this.lastJoinFailure = joinFailure
      this.finishJob(active.job, { error: joinFailure })
      const queued = this.queue.splice(0)
      for (const job of queued) {
        this.finishJob(job, {
          error: new DirectGeometryQuarantinedError(active.epoch, { cause: joinFailure }),
        })
      }
      active.resolveDone()
      return
    }

    if (active.stopReason !== null) {
      const error = active.stopReason === 'deadline'
        ? new GeometryDeadlineExceededError(this.jobDeadlineMs)
        : active.stopReason === 'startup'
          ? new DirectGeometryStartupError(active.epoch)
          : abortError(active.stopReason === 'closed'
              ? 'Geometry evaluation cancelled during shutdown'
              : 'Geometry evaluation was cancelled')
      this.finishJob(active.job, { error })
    } else if (failure !== null) {
      this.finishJob(active.job, { error: failure })
    } else if (terminal?.status === 'failed') {
      this.finishJob(active.job, {
        error: reconstructDirectGeometryRemoteError(
          terminal.error,
          active.job.source ?? '',
          terminal.execution,
        ),
      })
    } else if (terminal?.status === 'succeeded' && terminal.kind === 'build') {
      this.finishJob(active.job, { value: terminal.built })
    } else if (terminal?.status === 'succeeded' && terminal.kind === 'capabilities') {
      this.finishJob(active.job, { value: terminal.capabilities })
    } else {
      this.finishJob(active.job, {
        error: new DirectGeometryProtocolError(active.epoch, 'Worker settled without a terminal.'),
      })
    }

    active.resolveDone()
    if (!this.closing && !this.closed && !this.quarantined) queueMicrotask(() => this.pump())
  }

  private async terminateAndJoin(active: ActiveJob): Promise<DirectGeometryJoinError | null> {
    type JoinOutcome =
      | { kind: 'joined' }
      | { kind: 'failed'; cause: unknown }
      | { kind: 'timeout' }
    let joinTimer: ReturnType<typeof setTimeout> | undefined
    const termination = Promise.resolve()
      .then(() => active.worker.terminate())
      .then<JoinOutcome, JoinOutcome>(
        () => ({ kind: 'joined' }),
        cause => ({ kind: 'failed', cause }),
      )
    const outcome = await Promise.race<JoinOutcome>([
      termination,
      new Promise(resolve => {
        joinTimer = setTimeout(() => resolve({ kind: 'timeout' }), this.joinTimeoutMs)
      }),
    ])
    if (joinTimer !== undefined) clearTimeout(joinTimer)
    if (outcome.kind !== 'joined') {
      // A Worker stuck inside native/WASM teardown must not keep the MCP host
      // alive after its supervisor is quarantined. No next job is admitted,
      // and a late successful join only releases remaining listeners.
      try { active.worker.unref() } catch { /* injected doubles may omit unref */ }
      try {
        active.stopOutputDrains()
        active.worker.stdout.destroy()
        active.worker.stderr.destroy()
      } catch {
        // Quarantine and Worker.unref() remain the authoritative containment.
      }
      if (outcome.kind === 'timeout') {
        void termination.then(lateOutcome => {
          if (lateOutcome.kind !== 'joined') return
          try {
            active.stopOutputDrains()
            active.detachWorkerListeners()
          } catch {
            // The realm is already joined; late cleanup is best effort.
          }
        })
      }
      return new DirectGeometryJoinError(
        active.epoch,
        this.joinTimeoutMs,
        outcome.kind === 'failed' ? { cause: outcome.cause } : {},
      )
    }
    active.stopOutputDrains()
    active.detachWorkerListeners()
    this.lastJoinedWorkerEpoch = active.epoch
    this.workersJoined++
    return null
  }

  private finishJob(
    job: AdmittedJob,
    outcome: { value: unknown } | { error: unknown },
  ): void {
    if (job.state === 'terminal') return
    job.state = 'terminal'
    if (job.deadlineTimer !== undefined) clearTimeout(job.deadlineTimer)
    job.deadlineTimer = undefined
    if (job.signal !== undefined && job.abortListener !== undefined) {
      job.signal.removeEventListener('abort', job.abortListener)
    }
    job.abortListener = undefined
    if ('error' in outcome) job.reject(outcome.error)
    else job.resolve(outcome.value)
  }
}
