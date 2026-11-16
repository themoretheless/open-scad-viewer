import type { Worker } from 'node:worker_threads'
import type { GeometryQuality } from '../../core/build'
import type { GeometryBuildPurpose } from '../../core/geometryExecution'
import type { DirectGeometryRequest } from '../directGeometryProtocol'

export const DIRECT_GEOMETRY_MAX_ADMITTED_JOBS = 8
export const DIRECT_GEOMETRY_JOB_DEADLINE_MS = 30_000
export const DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS = 10_000
export const DIRECT_GEOMETRY_CANCEL_GRACE_MS = 25
export const DIRECT_GEOMETRY_JOIN_TIMEOUT_MS = 5_000

export const MAX_STARTUP_TIMEOUT_MS = 30_000
export const MAX_JOB_DEADLINE_MS = 120_000
export const MAX_CANCEL_GRACE_MS = 1_000
export const MAX_JOIN_TIMEOUT_MS = 10_000
export const MAX_DRAINED_CHILD_OUTPUT_BYTES = 8 * 1024

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

export type JobState = 'queued' | 'active' | 'terminal'
export type StopReason = 'cancelled' | 'deadline' | 'startup' | 'closed'

export interface AdmittedJob {
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

export interface ActiveJob {
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

export function boundedInteger(value: number, label: string, minimum: number, maximum: number): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}
