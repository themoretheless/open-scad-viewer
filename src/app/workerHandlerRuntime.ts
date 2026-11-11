/**
 * Shared plumbing for request/response workers: validate the incoming message,
 * serialize concurrent jobs behind a busy flag, and convert thrown values into
 * the wire error shape. Protocol-specific envelopes stay with the caller.
 */
export interface WorkerErrorInfo {
  name: string
  message: string
  code?: string
}

/** Normalize any thrown value into the {name, message, code?} wire error shape. */
export function toWorkerErrorInfo(cause: unknown): WorkerErrorInfo {
  const error = cause instanceof Error ? cause : new Error(String(cause))
  return {
    name: error.name,
    message: error.message,
    ...('code' in error && typeof error.code === 'string' ? { code: error.code } : {}),
  }
}

export interface WorkerHandlerSpec<Req, Res, R> {
  /** Return the validated request, or null to ignore the message silently. */
  validate(value: unknown): Req | null
  /** Error posted when a second request arrives while a job is running. */
  busyError: WorkerErrorInfo
  /** Async hook inside the try block, before execute (e.g. kernel warmup). */
  beforeExecute?(request: Req): void | Promise<void>
  execute(request: Req): R | Promise<R>
  success(request: Req, result: R): { message: Res; transfer?: ArrayBuffer[] }
  failure(request: Req, error: WorkerErrorInfo): Res
}

export function createWorkerHandler<Req, Res, R>(
  post: (message: Res, transfer?: ArrayBuffer[]) => void,
  spec: WorkerHandlerSpec<Req, Res, R>,
): (value: unknown) => Promise<void> {
  let active = false
  return async (value: unknown) => {
    const request = spec.validate(value)
    if (!request) return
    if (active) { post(spec.failure(request, spec.busyError)); return }
    active = true
    try {
      await spec.beforeExecute?.(request)
      const { message, transfer } = spec.success(request, await spec.execute(request))
      post(message, transfer)
    } catch (cause) {
      post(spec.failure(request, toWorkerErrorInfo(cause)))
    } finally { active = false }
  }
}
