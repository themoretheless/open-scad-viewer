import {sha256Hex} from '../core/sha256'
import type {BrepSemanticDisplayPolicy} from './brepSemanticScene'
import {BREP_DIAGNOSTIC_IDENTITY, BREP_DIAGNOSTIC_LIMITS, isBrepDiagnosticMessage, isBrepDiagnosticRequest,
  type BrepDiagnosticMessage, type BrepDiagnosticReceipt, type BrepDiagnosticRequest} from './brepDiagnosticProtocol'

/** Host-specific mechanics only. No direct/in-process evaluation fallback. */
export interface BrepDiagnosticWorkerPort {
  postMessage(value: BrepDiagnosticRequest): void
  listen(callbacks: {message(value: unknown): void; error(error: unknown): void; exit(): void}): () => void
  /** Browser invokes the synchronous platform terminate API (no join acknowledgement); Node awaits join. */
  terminate(): void | Promise<void>
  /** A failed Node join must neither admit another job nor keep the host alive. */
  unref?(): void
}
export type BrepDiagnosticHostErrorCode = 'E_BREP_DIAGNOSTIC_INPUT' | 'E_BREP_DIAGNOSTIC_BUSY'
  | 'E_BREP_DIAGNOSTIC_CANCELLED' | 'E_BREP_DIAGNOSTIC_STARTUP' | 'E_BREP_DIAGNOSTIC_DEADLINE'
  | 'E_BREP_DIAGNOSTIC_PROTOCOL' | 'E_BREP_DIAGNOSTIC_CRASH' | 'E_BREP_DIAGNOSTIC_JOIN'
  | 'E_BREP_DIAGNOSTIC_QUARANTINED'
export class BrepDiagnosticHostError extends Error {
  constructor(readonly code: BrepDiagnosticHostErrorCode, message: string, readonly workerEpoch: number | null) {
    super(message); this.name = 'BrepDiagnosticHostError'
  }
}
export class BrepDiagnosticRemoteError extends Error {
  constructor(readonly code: string | null, name: string, message: string) {super(message); this.name = name}
}
export interface BrepDiagnosticExecutorOptions {
  readonly startupTimeoutMs?: number
  readonly deadlineMs?: number
  readonly joinTimeoutMs?: number
}
function duration(value: number, maximum: number): number {
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum) throw new RangeError(`Diagnostic timeout must be 1..${maximum} ms`)
  return value
}

/** One disposable realm per run. Diagnostic receipts are never published by the production coordinator. */
export class BrepDiagnosticExecutor {
  private nextEpoch = 1
  private activeEpoch: number | null = null
  private workersStarted = 0
  private workersTerminated = 0
  private quarantined = false
  private readonly startupMs: number
  private readonly deadlineMs: number
  private readonly joinMs: number
  constructor(private readonly factory: () => BrepDiagnosticWorkerPort, options: BrepDiagnosticExecutorOptions = {}) {
    this.startupMs = duration(options.startupTimeoutMs ?? BREP_DIAGNOSTIC_LIMITS.startupMs, BREP_DIAGNOSTIC_LIMITS.startupMs)
    this.deadlineMs = duration(options.deadlineMs ?? BREP_DIAGNOSTIC_LIMITS.deadlineMs, BREP_DIAGNOSTIC_LIMITS.deadlineMs)
    this.joinMs = duration(options.joinTimeoutMs ?? BREP_DIAGNOSTIC_LIMITS.joinMs, BREP_DIAGNOSTIC_LIMITS.joinMs)
  }
  snapshot() {return Object.freeze({activeWorkerEpoch: this.activeEpoch, workersStarted: this.workersStarted,
    workersTerminated: this.workersTerminated, quarantined: this.quarantined})}
  async evaluate(source: string, policy: BrepSemanticDisplayPolicy = {quality: 'preview', segments: 4},
    options: {signal?: AbortSignal; deadlineMs?: number} = {}): Promise<BrepDiagnosticReceipt> {
    const fail = (code: BrepDiagnosticHostErrorCode, message: string) => new BrepDiagnosticHostError(code, message, this.activeEpoch)
    if (this.quarantined) throw fail('E_BREP_DIAGNOSTIC_QUARANTINED', 'Diagnostic lane quarantined after failed worker teardown')
    if (this.activeEpoch !== null) throw fail('E_BREP_DIAGNOSTIC_BUSY', 'Diagnostic lane already owns an active worker')
    if (typeof source !== 'string' || source.length > BREP_DIAGNOSTIC_LIMITS.sourceCharacters) throw fail('E_BREP_DIAGNOSTIC_INPUT', 'Diagnostic source exceeds its input limit')
    const deadlineMs = duration(options.deadlineMs ?? this.deadlineMs, this.deadlineMs)
    const epoch = this.nextEpoch++
    const request: BrepDiagnosticRequest = Object.freeze({type: 'evaluate', workerEpoch: epoch, jobId: epoch,
      source, sourceSha256: sha256Hex(source), policy: Object.freeze({...policy}), identity: BREP_DIAGNOSTIC_IDENTITY})
    if (!isBrepDiagnosticRequest(request)) throw fail('E_BREP_DIAGNOSTIC_INPUT', 'Expected explicit brep-1 source and bounded diagnostic display policy')
    if (options.signal?.aborted) throw fail('E_BREP_DIAGNOSTIC_CANCELLED', 'Diagnostic evaluation cancelled before admission')
    const admittedAt = performance.now()
    this.activeEpoch = epoch
    let worker: BrepDiagnosticWorkerPort
    try {worker = this.factory()} catch {
      this.activeEpoch = null
      throw new BrepDiagnosticHostError('E_BREP_DIAGNOSTIC_CRASH', 'Diagnostic worker could not start', epoch)
    }
    this.workersStarted++
    return new Promise((resolve, reject) => {
      let settling = false, started = false
      let stopped: BrepDiagnosticHostError | null = null
      let startupTimer: ReturnType<typeof setTimeout> | undefined
      let deadlineTimer: ReturnType<typeof setTimeout> | undefined
      let detach = () => {}
      const error = (code: BrepDiagnosticHostErrorCode, message: string) => new BrepDiagnosticHostError(code, message, epoch)
      const finish = async (message?: BrepDiagnosticMessage, failure?: BrepDiagnosticHostError) => {
        if (settling) return
        settling = true
        clearTimeout(startupTimer); clearTimeout(deadlineTimer)
        let joinTimer: ReturnType<typeof setTimeout> | undefined
        try {
          await Promise.race([
            Promise.resolve().then(() => worker.terminate()),
            new Promise<never>((_, rejectJoin) => {joinTimer = setTimeout(() => rejectJoin(new Error('Worker join timeout')), this.joinMs)}),
          ])
        } catch {
          this.quarantined = true
          worker.unref?.()
          // Keep the error listener attached to an unjoined Node worker. It may still exit later.
          options.signal?.removeEventListener('abort', onAbort)
          reject(error('E_BREP_DIAGNOSTIC_JOIN', 'Diagnostic worker did not terminate within its teardown bound'))
          return
        } finally {clearTimeout(joinTimer)}
        detach()
        options.signal?.removeEventListener('abort', onAbort)
        this.activeEpoch = null
        this.workersTerminated++
        // Cancellation during asynchronous Node join also forbids publication.
        if (options.signal?.aborted) stopped ??= error('E_BREP_DIAGNOSTIC_CANCELLED', 'Diagnostic evaluation cancelled')
        if (performance.now() - admittedAt >= deadlineMs) stopped ??= error('E_BREP_DIAGNOSTIC_DEADLINE', 'Diagnostic evaluation exceeded its total deadline')
        if (stopped) reject(stopped)
        else if (failure) reject(failure)
        else if (message?.status === 'failed') reject(new BrepDiagnosticRemoteError(message.error.code, message.error.name, message.error.message))
        else if (message?.status === 'succeeded') resolve(Object.freeze(message))
        else reject(error('E_BREP_DIAGNOSTIC_PROTOCOL', 'Diagnostic worker did not produce a terminal'))
      }
      const stop = (code: BrepDiagnosticHostErrorCode, message: string) => {
        stopped ??= error(code, message)
        void finish()
      }
      const onAbort = () => stop('E_BREP_DIAGNOSTIC_CANCELLED', 'Diagnostic evaluation cancelled')
      try {
        detach = worker.listen({
          message: value => {
            if (settling) return
            if (!isBrepDiagnosticMessage(value, request)) {void finish(undefined, error('E_BREP_DIAGNOSTIC_PROTOCOL', 'Invalid or uncorrelated diagnostic worker message')); return}
            if (!started) {
              if (value.status !== 'started') {void finish(undefined, error('E_BREP_DIAGNOSTIC_PROTOCOL', 'Diagnostic terminal arrived before startup')); return}
              started = true; clearTimeout(startupTimer)
            } else if (value.status === 'started') void finish(undefined, error('E_BREP_DIAGNOSTIC_PROTOCOL', 'Diagnostic worker repeated startup'))
            else void finish(value)
          },
          error: () => {void finish(undefined, error('E_BREP_DIAGNOSTIC_CRASH', 'Diagnostic worker crashed or could not deserialize a message'))},
          exit: () => {void finish(undefined, error('E_BREP_DIAGNOSTIC_CRASH', 'Diagnostic worker exited before a terminal'))},
        })
        options.signal?.addEventListener('abort', onAbort, {once: true})
        if (options.signal?.aborted) {onAbort(); return}
        startupTimer = setTimeout(() => stop('E_BREP_DIAGNOSTIC_STARTUP', 'Diagnostic worker startup exceeded its bound'), this.startupMs)
        deadlineTimer = setTimeout(() => stop('E_BREP_DIAGNOSTIC_DEADLINE', 'Diagnostic evaluation exceeded its total deadline'), Math.max(0, deadlineMs - (performance.now() - admittedAt)))
        worker.postMessage(request)
      } catch {void finish(undefined, error('E_BREP_DIAGNOSTIC_PROTOCOL', 'Diagnostic request could not be sent'))}
    })
  }
}
