import type { StepRawArtifact } from './stepRawArtifact'
import type { ParserInputBudgets, ParserPreflightStats } from '../language/parserInputBudgets'

export type UntrustedParserJob =
  | { readonly kind: 'step-raw'; readonly text: string; readonly budgets?: ParserInputBudgets }
  | { readonly kind: 'iges-preflight'; readonly text: string; readonly budgets?: ParserInputBudgets }
  | { readonly kind: 'stl-preflight'; readonly bytes: Uint8Array; readonly budgets?: ParserInputBudgets }

export type UntrustedParserResult<K extends UntrustedParserJob['kind'] = UntrustedParserJob['kind']> =
  K extends 'step-raw' ? StepRawArtifact : ParserPreflightStats

export type UntrustedParserRequest = { readonly version: 1; readonly id: number; readonly job: UntrustedParserJob }
export type UntrustedParserResponse = { readonly version: 1; readonly id: number } & (
  | { readonly ok: true; readonly result: UntrustedParserResult }
  | { readonly ok: false; readonly error: { readonly name: string; readonly code: string; readonly message: string } }
)

export const PARSER_ISOLATION_BOUNDARY = Object.freeze({
  kind: 'worker-api-isolation',
  sandbox: false,
  terminatesOnTimeout: true,
  terminatesOnCancellation: true,
  resourceLimits: 'shared lexical parser budgets; no OS memory/CPU sandbox',
  note: 'A separate worker realm contains parser failures and permits termination. It is not a security sandbox.',
})

export interface UntrustedParserPort {
  onmessage: ((event: MessageEvent) => void) | null
  onerror: ((event: ErrorEvent) => void) | null
  onmessageerror: ((event: MessageEvent) => void) | null
  postMessage(request: UntrustedParserRequest): void
  terminate(): void
}

export interface UntrustedParserRunOptions {
  readonly signal?: AbortSignal
  readonly timeoutMs?: number
}

export class UntrustedParserWorkerError extends Error {
  constructor(readonly code: string, message: string, name = 'UntrustedParserWorkerError') {
    super(message)
    this.name = name
  }
}

const cancelled = () => new UntrustedParserWorkerError('PARSER_CANCELLED', 'Parser operation cancelled', 'AbortError')

/** Same terminate-on-cancel lifecycle as the established CAD worker client. */
export class UntrustedParserWorkerClient {
  private worker: UntrustedParserPort | null = null
  private active: (() => void) | null = null
  private nextId = 1

  constructor(private readonly factory: () => UntrustedParserPort) {}

  run<K extends UntrustedParserJob['kind']>(
    job: Extract<UntrustedParserJob, { kind: K }>,
    options: UntrustedParserRunOptions = {},
  ): Promise<UntrustedParserResult<K>> {
    if (options.signal?.aborted) return Promise.reject(cancelled())
    const timeoutMs = options.timeoutMs ?? 30_000
    if (!Number.isFinite(timeoutMs) || timeoutMs < 1 || timeoutMs > 120_000) {
      return Promise.reject(new RangeError('Parser timeout must be between 1 and 120000 ms'))
    }
    this.cancel()
    let worker: UntrustedParserPort
    try { worker = this.worker ??= this.factory() }
    catch { return Promise.reject(new UntrustedParserWorkerError('PARSER_STARTUP', 'Could not start parser worker')) }
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      let settled = false
      let timer: ReturnType<typeof setTimeout> | undefined
      const finish = (error?: Error, result?: UntrustedParserResult<K>, discard = false) => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        options.signal?.removeEventListener('abort', abort)
        worker.onmessage = null
        worker.onerror = null
        worker.onmessageerror = null
        if (this.active === fail) this.active = null
        if (discard) { if (this.worker === worker) this.worker = null; worker.terminate() }
        if (error) reject(error); else resolve(result!)
      }
      const fail = (error: Error, discard = true) => finish(error, undefined, discard)
      const abort = () => fail(cancelled())
      worker.onmessage = event => {
        const data = event.data as UntrustedParserResponse | null
        if (!data || data.version !== 1 || data.id !== id) {
          fail(new UntrustedParserWorkerError('PARSER_PROTOCOL', 'Invalid parser worker response')); return
        }
        if (data.ok) finish(undefined, data.result as UntrustedParserResult<K>)
        else fail(new UntrustedParserWorkerError(data.error.code, data.error.message, data.error.name), false)
      }
      worker.onerror = event => fail(new UntrustedParserWorkerError('PARSER_CRASH', event.message || 'Parser worker failed'))
      worker.onmessageerror = () => fail(new UntrustedParserWorkerError('PARSER_TRANSPORT', 'Parser response could not be received'))
      this.active = () => fail(cancelled())
      options.signal?.addEventListener('abort', abort, { once: true })
      timer = setTimeout(() => fail(new UntrustedParserWorkerError('PARSER_TIMEOUT', 'Parser operation exceeded its time limit')), timeoutMs)
      try { worker.postMessage({ version: 1, id, job }) }
      catch { fail(new UntrustedParserWorkerError('PARSER_TRANSPORT', 'Parser request could not be sent')) }
    })
  }

  cancel(): void { this.active?.() }
  dispose(): void { this.cancel(); this.worker?.terminate(); this.worker = null }
}
