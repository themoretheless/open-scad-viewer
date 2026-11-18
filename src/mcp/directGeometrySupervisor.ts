import { Worker } from 'node:worker_threads'
import type { GeometryQuality } from '../core/build'
import {
  planGeometrySourceExecution,
  type GeometryBuildPurpose,
  type GeometryEngineRegistrySnapshot,
} from '../core/geometryExecution'
import type { GeometryBuildResult } from '../kernel/geometryBuildEngine'
import type { McpGeometryBuildRuntime } from './geometryService'
import { GeometryBusyError, GeometryDeadlineExceededError } from './geometryService'
import {
  isDirectGeometryNodeTerminal,
  isDirectGeometryRequest,
  isDirectGeometryStarted,
  reconstructDirectGeometryRemoteError,
  type DirectGeometryTerminal,
} from './directGeometryProtocol'
import {
  DIRECT_GEOMETRY_CANCEL_GRACE_MS,
  DIRECT_GEOMETRY_JOB_DEADLINE_MS,
  DIRECT_GEOMETRY_JOIN_TIMEOUT_MS,
  DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
  DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS,
  DirectGeometryClosedError,
  DirectGeometryJoinError,
  DirectGeometryProtocolError,
  DirectGeometryQuarantinedError,
  DirectGeometryStartupError,
  DirectGeometrySupervisorError,
  DirectGeometryWorkerCrashError,
  MAX_CANCEL_GRACE_MS,
  MAX_JOB_DEADLINE_MS,
  MAX_JOIN_TIMEOUT_MS,
  MAX_STARTUP_TIMEOUT_MS,
  boundedInteger,
  type ActiveJob,
  type AdmittedJob,
  type DirectGeometrySupervisorErrorCode,
  type DirectGeometrySupervisorOptions,
  type DirectGeometrySupervisorSnapshot,
  type StopReason,
} from './directGeometrySupervisor/errorsAndTypes'
import {
  abortError,
  buildWorkerRequest,
  createActiveJobHandle,
  sendCancelToWorker,
  terminateAndJoinWorker,
} from './directGeometrySupervisor/workerExecution'

export {
  DIRECT_GEOMETRY_CANCEL_GRACE_MS,
  DIRECT_GEOMETRY_JOB_DEADLINE_MS,
  DIRECT_GEOMETRY_JOIN_TIMEOUT_MS,
  DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
  DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS,
  DirectGeometryClosedError,
  DirectGeometryJoinError,
  DirectGeometryProtocolError,
  DirectGeometryQuarantinedError,
  DirectGeometryStartupError,
  DirectGeometrySupervisorError,
  DirectGeometryWorkerCrashError,
}
export type { DirectGeometrySupervisorErrorCode, DirectGeometrySupervisorOptions, DirectGeometrySupervisorSnapshot }

function defaultWorkerFactory(_workerEpoch: number): Worker {
  return new Worker(new URL('./directGeometry.worker.mjs', import.meta.url), {
    // The bootstrap owns its loader; do not inherit a second --import tsx realm.
    execArgv: [],
    // Geometry source never needs host credentials or configuration. An empty
    // environment keeps accidental dependency reads from inheriting secrets.
    env: {},
    stdout: true,
    stderr: true,
    resourceLimits: { maxOldGenerationSizeMb: 256, maxYoungGenerationSizeMb: 32, stackSizeMb: 8 },
  })
}

/**
 * Production MCP hard boundary for the legacy direct evaluator. Every job owns
 * a disposable Node Worker and publishes its terminal only after join.
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

  isHealthy(): boolean {
    return !this.quarantined && !this.closing && !this.closed
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
    if (signal?.aborted) return Promise.reject(abortError('Geometry evaluation was cancelled before admission'))
    try {
      if (typeof source !== 'string') throw new TypeError('Geometry source must be a string')
      if (quality !== 'preview' && quality !== 'full') throw new TypeError('Geometry quality must be preview or full')
      if (purpose !== 'preview' && purpose !== 'full' && purpose !== 'analysis' && purpose !== 'export') {
        throw new TypeError('Geometry purpose is invalid')
      }
      planGeometrySourceExecution(source, { quality, purpose })
    } catch (error) {
      return Promise.reject(error)
    }
    return this.admit<GeometryBuildResult>({ kind: 'build', source, quality, purpose, signal })
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
      for (const job of this.queue.splice(0)) {
        this.finishJob(job, { error: abortError('Geometry evaluation cancelled during shutdown') })
      }
      if (this.active !== null) {
        const active = this.active
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
    if (input.signal?.aborted) return Promise.reject(abortError('Geometry evaluation was cancelled before admission'))
    if (this.quarantined) return Promise.reject(new DirectGeometryQuarantinedError(this.quarantineEpoch))
    if (this.closing || this.closed) return Promise.reject(new DirectGeometryClosedError())
    if (this.queue.length + (this.active === null ? 0 : 1) >= this.maxAdmittedJobs)
      return Promise.reject(new GeometryBusyError())

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
    const request = buildWorkerRequest(job, epoch)
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
    const active = createActiveJobHandle({
      job,
      request,
      worker,
      epoch,
      startupTimeoutMs: this.startupTimeoutMs,
      onMessage: (handle, value) => this.onWorkerMessage(handle, value),
      onSettleFailure: (handle, error) => {
        void this.settleActive(handle, null, error)
      },
      onStartupTimeout: handle => this.requestStop(handle, 'startup'),
    })
    this.active = active
    this.workersStarted++
    try {
      worker.postMessage(request)
    } catch (cause) {
      void this.settleActive(active, null, new DirectGeometryStartupError(epoch, { cause }))
    }
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
    if (this.active?.job === job) this.requestStop(this.active, 'cancelled')
  }

  private onJobDeadline(job: AdmittedJob): void {
    if (job.state === 'terminal') return
    if (job.state === 'queued') {
      const index = this.queue.indexOf(job)
      if (index >= 0) this.queue.splice(index, 1)
      this.finishJob(job, { error: new GeometryDeadlineExceededError(this.jobDeadlineMs) })
      return
    }
    if (this.active?.job === job) this.requestStop(this.active, 'deadline')
  }

  private requestStop(active: ActiveJob, reason: StopReason): void {
    if (this.active !== active || active.settling || active.stopReason !== null) return
    active.stopReason = reason
    sendCancelToWorker(active, reason)
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
    const joinFailure = await terminateAndJoinWorker(active, this.joinTimeoutMs)
    if (joinFailure === null) {
      this.lastJoinedWorkerEpoch = active.epoch
      this.workersJoined++
    }
    this.active = null
    if (joinFailure !== null) {
      this.quarantined = true
      this.quarantineEpoch = active.epoch
      this.lastJoinFailure = joinFailure
      this.finishJob(active.job, { error: joinFailure })
      for (const queued of this.queue.splice(0)) {
        this.finishJob(queued, { error: new DirectGeometryQuarantinedError(active.epoch, { cause: joinFailure }) })
      }
      active.resolveDone()
      return
    }
    if (active.stopReason !== null) {
      const error =
        active.stopReason === 'deadline'
          ? new GeometryDeadlineExceededError(this.jobDeadlineMs)
          : active.stopReason === 'startup'
            ? new DirectGeometryStartupError(active.epoch)
            : abortError(
                active.stopReason === 'closed'
                  ? 'Geometry evaluation cancelled during shutdown'
                  : 'Geometry evaluation was cancelled',
              )
      this.finishJob(active.job, { error })
    } else if (failure !== null) {
      this.finishJob(active.job, { error: failure })
    } else if (terminal?.status === 'failed') {
      this.finishJob(active.job, {
        error: reconstructDirectGeometryRemoteError(terminal.error, active.job.source ?? '', terminal.execution),
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

  private finishJob(job: AdmittedJob, outcome: { value: unknown } | { error: unknown }): void {
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
