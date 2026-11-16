import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { OFFICIAL_OPENSCAD_RUNTIME_VERSION } from '../officialOpenScadRuntimePatch'
import {
  isOfficialOpenScadWireTerminal,
  type OfficialOpenScadWireRequest,
  type OfficialOpenScadWireTerminal,
} from '../officialOpenScadRuntimeProtocol'
import { compactChildError } from './defaultCacheRoot'
import {
  MAX_CHILD_STDERR_BYTES,
  MAX_CHILD_STDOUT_BYTES,
  OfficialOpenScadRemoteError,
  OfficialOpenScadSupervisorError,
  type ActiveChild,
  type OfficialOpenScadChildFactory,
} from './types'

export type SucceededOfficialWireTerminal = Extract<OfficialOpenScadWireTerminal, { status: 'succeeded' }>

function stopReasonError(
  stopReason: NonNullable<ActiveChild['stopReason']>,
  jobId: number,
  timeoutMs: number,
): OfficialOpenScadSupervisorError {
  if (stopReason === 'cancelled') {
    return new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_CANCELLED',
      'Official OpenSCAD execution was cancelled',
      jobId,
    )
  }
  if (stopReason === 'deadline') {
    return new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_DEADLINE',
      `Official OpenSCAD execution exceeded ${timeoutMs} ms`,
      jobId,
    )
  }
  if (stopReason === 'closed') {
    return new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_CLOSED',
      'Official OpenSCAD execution was stopped during shutdown',
      jobId,
    )
  }
  return new OfficialOpenScadSupervisorError(
    'E_OFFICIAL_OPENSCAD_PROTOCOL',
    'Official OpenSCAD child exceeded its output protocol limit',
    jobId,
  )
}

function parseSucceededChildTerminal(
  request: OfficialOpenScadWireRequest,
  stdout: Buffer[],
  stdoutBytes: number,
  exitCode: number | null,
  exitSignal: NodeJS.Signals | null,
): SucceededOfficialWireTerminal {
  let value: unknown
  try {
    value = JSON.parse(Buffer.concat(stdout, stdoutBytes).toString('utf8'))
  } catch (error) {
    throw new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_PROTOCOL',
      `Official OpenSCAD child returned invalid JSON (exit ${String(exitCode)}, signal ${String(exitSignal)})`,
      request.jobId,
      { cause: error },
    )
  }
  if (!isOfficialOpenScadWireTerminal(value, request) || value.runtimeVersion !== OFFICIAL_OPENSCAD_RUNTIME_VERSION) {
    throw new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_PROTOCOL',
      'Official OpenSCAD child returned an invalid or mismatched terminal',
      request.jobId,
    )
  }
  if (value.status === 'failed') {
    throw new OfficialOpenScadRemoteError(
      value.error.code,
      value.error.message,
      request.jobId,
      value.logs,
      value.durationMs,
    )
  }
  return value
}

export interface SpawnAndJoinOfficialChildParams {
  readonly request: OfficialOpenScadWireRequest
  readonly runnerPath: string
  readonly runtimePath: string
  readonly fontPath: string
  readonly fontSha256: string
  readonly timeoutMs: number
  readonly signal: AbortSignal | undefined
  readonly childFactory: OfficialOpenScadChildFactory
  readonly activeMap: Map<number, ActiveChild>
  readonly onStarted: () => void
  readonly onJoined: () => void
}

export function spawnAndJoinOfficialChild(
  params: SpawnAndJoinOfficialChildParams,
): Promise<SucceededOfficialWireTerminal> {
  const { request } = params
  let child: ChildProcessWithoutNullStreams
  try {
    child = params.childFactory({
      runnerPath: params.runnerPath,
      runtimePath: params.runtimePath,
      fontPath: params.fontPath,
      fontSha256: params.fontSha256,
    })
  } catch (error) {
    return Promise.reject(
      new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_CHILD_CRASH',
        'Unable to start the official OpenSCAD child process',
        request.jobId,
        { cause: error },
      ),
    )
  }
  params.onStarted()
  let resolveJoined!: () => void
  const joined = new Promise<void>(resolve => {
    resolveJoined = resolve
  })
  const active: ActiveChild = { child, joined, stopReason: null }
  params.activeMap.set(request.jobId, active)

  return new Promise((resolvePromise, rejectPromise) => {
    const stdout: Buffer[] = []
    let stdoutBytes = 0
    let stderrBytes = 0
    let spawnError: unknown = null
    let finished = false
    const deadline = setTimeout(() => {
      active.stopReason ??= 'deadline'
      child.kill('SIGKILL')
    }, params.timeoutMs)
    const onAbort = () => {
      active.stopReason ??= 'cancelled'
      child.kill('SIGKILL')
    }
    params.signal?.addEventListener('abort', onAbort, { once: true })

    child.stdout.on('data', (chunk: Buffer | string) => {
      const data = Buffer.from(chunk)
      stdoutBytes += data.byteLength
      if (stdoutBytes > MAX_CHILD_STDOUT_BYTES) {
        active.stopReason ??= 'output-limit'
        child.kill('SIGKILL')
        return
      }
      stdout.push(data)
    })
    child.stderr.on('data', (chunk: Buffer | string) => {
      // Drain forever, retain nothing, and keep only a saturating byte count.
      stderrBytes = Math.min(MAX_CHILD_STDERR_BYTES, stderrBytes + Buffer.byteLength(chunk))
    })
    child.on('error', error => {
      spawnError = error
    })
    child.stdin.on('error', () => undefined)

    const finish = (exitCode: number | null, exitSignal: NodeJS.Signals | null) => {
      if (finished) return
      finished = true
      clearTimeout(deadline)
      params.signal?.removeEventListener('abort', onAbort)
      params.activeMap.delete(request.jobId)
      params.onJoined()
      resolveJoined()

      if (active.stopReason !== null) {
        rejectPromise(stopReasonError(active.stopReason, request.jobId, params.timeoutMs))
        return
      }
      if (spawnError !== null) {
        rejectPromise(
          new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_CHILD_CRASH',
            compactChildError(spawnError),
            request.jobId,
            { cause: spawnError },
          ),
        )
        return
      }
      try {
        resolvePromise(parseSucceededChildTerminal(request, stdout, stdoutBytes, exitCode, exitSignal))
      } catch (error) {
        rejectPromise(error)
      }
    }
    child.once('close', finish)
    child.stdin.end(JSON.stringify(request))
  })
}
