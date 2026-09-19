import { EventEmitter } from 'node:events'
import { PassThrough } from 'node:stream'
import type { Worker } from 'node:worker_threads'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { GeometryQuality } from '../src/core/build'
import {
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS,
  GEOMETRY_ENGINE_ROUTES,
  GEOMETRY_MANIFEST_ARCHIVE,
  planGeometrySourceExecution,
  type GeometryBuildPurpose,
  type GeometryEngineRegistrySnapshot,
} from '../src/core/geometryExecution'
import { OpenSCADParseError } from '../src/services/openscadErrors'
import {
  DIRECT_GEOMETRY_IDENTITY,
  DIRECT_GEOMETRY_PROTOCOL_VERSION,
  isDirectGeometryNodeTerminal,
  type DirectGeometryRequest,
  type DirectGeometryTerminal,
} from '../src/mcp/directGeometryProtocol'
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
  DirectGeometrySupervisor,
  DirectGeometryWorkerCrashError,
  type DirectGeometrySupervisorOptions,
} from '../src/mcp/directGeometrySupervisor'
import { GeometryBusyError, GeometryDeadlineExceededError } from '../src/mcp/geometryService'

interface Deferred<T> {
  promise: Promise<T>
  resolve(value: T): void
  reject(reason: unknown): void
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}

class ControlledWorker extends EventEmitter {
  readonly stdout = new PassThrough()
  readonly stderr = new PassThrough()
  readonly posted: unknown[] = []
  terminateCalls = 0
  unrefCalls = 0
  private terminateResult: () => Promise<number> = async () => 1

  constructor(readonly epoch: number) {
    super()
  }

  postMessage(message: unknown): void {
    this.posted.push(message)
  }

  terminate(): Promise<number> {
    this.terminateCalls++
    return this.terminateResult()
  }

  unref(): void {
    this.unrefCalls++
  }

  setTerminateResult(result: () => Promise<number>): void {
    this.terminateResult = result
  }

  request(): DirectGeometryRequest {
    const request = this.posted.find((value): value is DirectGeometryRequest => (
      value !== null && typeof value === 'object'
      && ((value as { type?: unknown }).type === 'build'
        || (value as { type?: unknown }).type === 'capabilities')
    ))
    if (!request) throw new Error('Worker has not received a direct geometry request')
    return request
  }

  sendStarted(): void {
    const request = this.request()
    this.emit('message', {
      ...eventEnvelope(request),
      status: 'started',
    })
  }

  sendBuildSuccess(): void {
    const request = this.request()
    if (request.type !== 'build') throw new Error('Expected a build request')
    this.emit('message', buildSuccess(request))
  }

  sendCapabilitiesSuccess(): void {
    const request = this.request()
    if (request.type !== 'capabilities') throw new Error('Expected a capabilities request')
    this.emit('message', {
      ...eventEnvelope(request),
      status: 'succeeded',
      kind: 'capabilities',
      capabilities: capabilitiesSnapshot(),
    } satisfies DirectGeometryTerminal)
  }
}

function eventEnvelope(request: DirectGeometryRequest) {
  return {
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    workerEpoch: request.workerEpoch,
    jobId: request.jobId,
    sourceSha256: request.sourceSha256,
    quality: request.quality,
    purpose: request.purpose,
    identity: DIRECT_GEOMETRY_IDENTITY,
    kind: request.type,
  } as const
}

function buildSuccess(request: Extract<DirectGeometryRequest, { type: 'build' }>): DirectGeometryTerminal {
  const execution = {
    ...planGeometrySourceExecution(request.source, {
      quality: request.quality,
      purpose: request.purpose,
    }),
    evidence: 'runtime' as const,
  }
  return {
    ...eventEnvelope(request),
    status: 'succeeded',
    kind: 'build',
    built: {
      execution,
      result: {
        meshes: [],
        warnings: [],
        volume: 0,
        surfaceArea: 0,
        quality: request.quality,
        reduced: false,
        timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
      },
    },
  }
}

function capabilitiesSnapshot(): GeometryEngineRegistrySnapshot {
  return {
    contractVersion: 1,
    sourceDirectedRouting: true,
    automaticFallback: false,
    routes: GEOMETRY_ENGINE_ROUTES.map(route => ({ ...route })),
    engines: [
      {
        ...GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh],
        availability: 'available',
        unavailableReason: null,
      },
      {
        ...GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep],
        availability: 'available',
        unavailableReason: null,
      },
    ],
  }
}

function createHarness(options: Omit<DirectGeometrySupervisorOptions, 'workerFactory'> = {}) {
  const workers: ControlledWorker[] = []
  const supervisor = new DirectGeometrySupervisor({
    cancelGraceMs: 1,
    ...options,
    workerFactory: epoch => {
      const worker = new ControlledWorker(epoch)
      workers.push(worker)
      return worker as unknown as Worker
    },
  })
  supervisors.add(supervisor)
  return { supervisor, workers }
}

function wait(milliseconds: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, milliseconds))
}

async function flushMicrotasks(): Promise<void> {
  for (let index = 0; index < 8; index++) await Promise.resolve()
}

function expectAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
}

const supervisors = new Set<DirectGeometrySupervisor>()

afterEach(async () => {
  await Promise.allSettled([...supervisors].map(supervisor => supervisor.close()))
  supervisors.clear()
  vi.restoreAllMocks()
})

describe('DirectGeometrySupervisor', () => {
  it('exports the exact production admission and watchdog limits', () => {
    expect({
      max: DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
      deadline: DIRECT_GEOMETRY_JOB_DEADLINE_MS,
      startup: DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS,
      grace: DIRECT_GEOMETRY_CANCEL_GRACE_MS,
      join: DIRECT_GEOMETRY_JOIN_TIMEOUT_MS,
    }).toEqual({ max: 8, deadline: 30_000, startup: 5_000, grace: 25, join: 1_000 })
  })

  it('runs the real direct evaluator and capability probe in disposable production workers', async () => {
    const supervisor = new DirectGeometrySupervisor({
      jobDeadlineMs: 10_000,
      startupTimeoutMs: 5_000,
    })
    supervisors.add(supervisor)

    await expect(supervisor.build('cube(2);', 'full', 'analysis')).resolves.toMatchObject({
      execution: {
        semanticProgramVersion: 'legacy-direct-evaluator-v1',
        purpose: 'analysis',
        evidence: 'runtime',
      },
      result: { quality: 'full', volume: 8, surfaceArea: 24 },
    })
    await expect(supervisor.capabilities()).resolves.toMatchObject({
      contractVersion: 1,
      sourceDirectedRouting: true,
      automaticFallback: false,
      engines: [
        { engineClass: 'mesh', availability: 'available' },
        { engineClass: 'brep', availability: 'available' },
      ],
    })
    expect(supervisor.snapshot()).toMatchObject({ workersStarted: 2, workersJoined: 2 })
  }, 30_000)

  it('joins a real failed build before admitting a fresh successful worker', async () => {
    const supervisor = new DirectGeometrySupervisor({ jobDeadlineMs: 10_000, startupTimeoutMs: 5_000 })
    supervisors.add(supervisor)
    await expect(supervisor.build('assert(false, "join failure path");', 'full', 'analysis')).rejects.toThrow('join failure path')
    await expect(supervisor.build('cube(2);', 'full', 'analysis')).resolves.toMatchObject({ result: { volume: 8 } })
    expect(supervisor.snapshot()).toMatchObject({
      workersStarted: 2, workersJoined: 2, quarantined: false, admittedJobs: 0,
    })
  }, 30_000)

  it('runs one disposable worker at a time and joins before settlement or the next FIFO job', async () => {
    const { supervisor, workers } = createHarness()
    const joinGate = deferred<number>()
    const first = supervisor.build('cube(1);', 'full', 'analysis')
    const second = supervisor.capabilities()

    expect(workers).toHaveLength(1)
    expect(supervisor.snapshot()).toMatchObject({ queuedJobs: 1, admittedJobs: 2 })
    workers[0].setTerminateResult(() => joinGate.promise)
    workers[0].sendStarted()
    workers[0].sendBuildSuccess()
    let firstSettled = false
    void first.then(() => { firstSettled = true }, () => { firstSettled = true })
    await flushMicrotasks()

    expect(firstSettled).toBe(false)
    expect(workers).toHaveLength(1)
    expect(workers[0].terminateCalls).toBe(1)

    joinGate.resolve(1)
    await expect(first).resolves.toMatchObject({ result: { quality: 'full' } })
    await flushMicrotasks()
    expect(workers).toHaveLength(2)
    expect(workers[1].epoch).toBe(2)

    workers[1].sendStarted()
    workers[1].sendCapabilitiesSuccess()
    await expect(second).resolves.toMatchObject({ contractVersion: 1 })
    expect(supervisor.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      queuedJobs: 0,
      lastJoinedWorkerEpoch: 2,
      workersStarted: 2,
      workersJoined: 2,
    })
  })

  it('bounds active plus queued admission at eight jobs', async () => {
    const { supervisor, workers } = createHarness()
    const jobs = [supervisor.build('cube(1);', 'preview', 'preview')]
    for (let index = 1; index < DIRECT_GEOMETRY_MAX_ADMITTED_JOBS; index++) {
      jobs.push(supervisor.build(`cube(${index + 1});`, 'preview', 'preview'))
    }
    const settlements = Promise.allSettled(jobs)

    expect(workers).toHaveLength(1)
    expect(supervisor.snapshot()).toMatchObject({ admittedJobs: 8, queuedJobs: 7 })
    await expect(supervisor.capabilities()).rejects.toBeInstanceOf(GeometryBusyError)

    await expect(supervisor.close()).resolves.toBeUndefined()
    const outcomes = await settlements
    expect(outcomes.every(outcome => outcome.status === 'rejected'
      && expectAbort(outcome.reason))).toBe(true)
  })

  it('rejects aborted or invalid direct runtime inputs before Worker admission', async () => {
    const { supervisor, workers } = createHarness()
    const controller = new AbortController()
    controller.abort()

    await expect(supervisor.build(
      'cube(1);',
      'full',
      'analysis',
      controller.signal,
    )).rejects.toSatisfy(expectAbort)
    await expect(supervisor.build(
      'x'.repeat(250_001),
      'full',
      'analysis',
    )).rejects.toMatchObject({ name: 'GeometryLanguageContractError' })
    await expect(supervisor.build(
      'cube(1);',
      'full',
      'invalid' as GeometryBuildPurpose,
    )).rejects.toBeInstanceOf(TypeError)
    expect(workers).toHaveLength(0)
  })

  it('includes FIFO queue time in each admission deadline', async () => {
    const { supervisor, workers } = createHarness({
      jobDeadlineMs: 35,
      startupTimeoutMs: 200,
      cancelGraceMs: 60,
    })
    const first = supervisor.build('cube(1);', 'full', 'analysis')
    workers[0].sendStarted()
    await wait(5)
    const queued = supervisor.capabilities()

    await expect(queued).rejects.toBeInstanceOf(GeometryDeadlineExceededError)
    expect(workers).toHaveLength(1)
    await expect(first).rejects.toMatchObject({
      name: 'GeometryDeadlineExceededError',
      deadlineMs: 35,
    })
    expect(supervisor.snapshot()).toMatchObject({ admittedJobs: 0, workersJoined: 1 })
  })

  it('removes an aborted queued job without disturbing the active worker', async () => {
    const { supervisor, workers } = createHarness()
    const first = supervisor.build('cube(1);', 'full', 'analysis')
    workers[0].sendStarted()
    const controller = new AbortController()
    const queued = supervisor.build('cube(2);', 'full', 'analysis', controller.signal)
    controller.abort()

    await expect(queued).rejects.toSatisfy(expectAbort)
    expect(workers).toHaveLength(1)
    expect(supervisor.snapshot()).toMatchObject({ admittedJobs: 1, queuedJobs: 0 })

    workers[0].sendBuildSuccess()
    await expect(first).resolves.toMatchObject({ result: { quality: 'full' } })
  })

  it('hard-terminates an active cancellation and waits for join before rejecting', async () => {
    const { supervisor, workers } = createHarness({ cancelGraceMs: 1 })
    const joinGate = deferred<number>()
    const controller = new AbortController()
    const build = supervisor.build('cube(1);', 'full', 'analysis', controller.signal)
    workers[0].setTerminateResult(() => joinGate.promise)
    workers[0].sendStarted()
    controller.abort()
    await wait(5)

    expect(workers[0].posted).toContainEqual(expect.objectContaining({
      type: 'cancel',
      reason: 'cancelled',
      workerEpoch: 1,
    }))
    expect(workers[0].terminateCalls).toBe(1)
    let settled = false
    void build.then(() => { settled = true }, () => { settled = true })
    await flushMicrotasks()
    expect(settled).toBe(false)

    joinGate.resolve(1)
    await expect(build).rejects.toSatisfy(expectAbort)
    expect(supervisor.snapshot()).toMatchObject({ workersJoined: 1, admittedJobs: 0 })
  })

  it('times out a missing started handshake, joins, and recovers in a new epoch', async () => {
    const { supervisor, workers } = createHarness({
      startupTimeoutMs: 10,
      jobDeadlineMs: 500,
      cancelGraceMs: 0,
    })
    await expect(supervisor.build('cube(1);', 'full', 'analysis'))
      .rejects.toBeInstanceOf(DirectGeometryStartupError)
    expect(supervisor.snapshot()).toMatchObject({ workersJoined: 1, quarantined: false })

    const recovered = supervisor.build('cube(2);', 'full', 'analysis')
    expect(workers).toHaveLength(2)
    workers[1].sendStarted()
    workers[1].sendBuildSuccess()
    await expect(recovered).resolves.toMatchObject({ result: { quality: 'full' } })
    expect(supervisor.snapshot().lastJoinedWorkerEpoch).toBe(2)
  })

  it('joins protocol failures and worker crashes, fencing their epochs from recovery', async () => {
    const { supervisor, workers } = createHarness()
    const invalid = supervisor.build('cube(1);', 'full', 'analysis')
    workers[0].emit('message', buildSuccess(workers[0].request() as Extract<DirectGeometryRequest, { type: 'build' }>))
    await expect(invalid).rejects.toBeInstanceOf(DirectGeometryProtocolError)
    expect(supervisor.snapshot().lastJoinedWorkerEpoch).toBe(1)

    const crashed = supervisor.build('cube(2);', 'full', 'analysis')
    workers[1].sendStarted()
    workers[1].emit('error', new Error('private fixture crash'))
    await expect(crashed).rejects.toBeInstanceOf(DirectGeometryWorkerCrashError)
    expect(supervisor.snapshot()).toMatchObject({
      lastJoinedWorkerEpoch: 2,
      workersJoined: 2,
      quarantined: false,
    })

    const recovered = supervisor.build('cube(3);', 'full', 'analysis')
    workers[2].sendStarted()
    workers[2].sendBuildSuccess()
    await expect(recovered).resolves.toMatchObject({ result: { quality: 'full' } })
  })

  it('reconstructs a validated child parse error only after joining its worker', async () => {
    const { supervisor, workers } = createHarness()
    const source = 'cube(;'
    const build = supervisor.build(source, 'full', 'analysis')
    const request = workers[0].request()
    if (request.type !== 'build') throw new Error('Expected a build request')
    workers[0].sendStarted()
    const execution = {
      ...planGeometrySourceExecution(source, { quality: 'full', purpose: 'analysis' }),
      evidence: 'runtime' as const,
    }
    const failure: DirectGeometryTerminal = {
      ...eventEnvelope(request),
      status: 'failed',
      kind: 'build',
      execution,
      error: {
        category: 'openscad-parse',
        name: 'OpenSCADParseError',
        message: 'Expected expression',
        code: null,
        line: 1,
        column: 6,
        start: 5,
        end: 6,
        reportedContract: null,
        availabilityCause: null,
        missingCapabilities: [],
      },
    }
    expect(isDirectGeometryNodeTerminal(failure, request)).toBe(true)
    workers[0].emit('message', failure)

    let error: unknown
    try {
      await build
    } catch (cause) {
      error = cause
    }
    expect(error).toBeInstanceOf(OpenSCADParseError)
    expect(error).toMatchObject({ start: 5, end: 6, line: 1, column: 6 })
    expect(supervisor.snapshot().workersJoined).toBe(1)
  })

  it('quarantines permanently when termination cannot join and rejects its FIFO queue', async () => {
    const { supervisor, workers } = createHarness({ joinTimeoutMs: 10 })
    const neverJoins = new Promise<number>(() => undefined)
    const first = supervisor.build('cube(1);', 'full', 'analysis')
    const queued = supervisor.capabilities()
    workers[0].setTerminateResult(() => neverJoins)
    workers[0].sendStarted()
    workers[0].sendBuildSuccess()

    await expect(first).rejects.toBeInstanceOf(DirectGeometryJoinError)
    await expect(queued).rejects.toBeInstanceOf(DirectGeometryQuarantinedError)
    expect(supervisor.snapshot()).toMatchObject({
      quarantined: true,
      queuedJobs: 0,
      workersStarted: 1,
      workersJoined: 0,
    })
    expect(workers[0].unrefCalls).toBe(1)
    await expect(supervisor.capabilities()).rejects.toBeInstanceOf(DirectGeometryQuarantinedError)
    await expect(supervisor.close()).rejects.toBeInstanceOf(DirectGeometryJoinError)
  })

  it('closes idempotently, aborting queued and active jobs after the active join', async () => {
    const { supervisor, workers } = createHarness({ cancelGraceMs: 1 })
    const active = supervisor.build('cube(1);', 'full', 'analysis')
    const queued = supervisor.capabilities()
    workers[0].sendStarted()
    const firstClose = supervisor.close()
    const secondClose = supervisor.close()

    expect(secondClose).toBe(firstClose)
    await expect(queued).rejects.toSatisfy(expectAbort)
    await expect(active).rejects.toSatisfy(expectAbort)
    await expect(firstClose).resolves.toBeUndefined()
    expect(supervisor.snapshot()).toMatchObject({ closed: true, admittedJobs: 0, workersJoined: 1 })
    await expect(supervisor.capabilities()).rejects.toBeInstanceOf(DirectGeometryClosedError)
  })

  it('drains noisy private child streams without forwarding them to MCP stdio', async () => {
    const stdout = vi.spyOn(process.stdout, 'write')
    const stderr = vi.spyOn(process.stderr, 'write')
    const { supervisor, workers } = createHarness()
    const build = supervisor.build('cube(1);', 'full', 'analysis')
    workers[0].stdout.write(`DIRECT_CHILD_STDOUT:${'o'.repeat(128 * 1024)}`)
    workers[0].stderr.write(`DIRECT_CHILD_STDERR:${'e'.repeat(128 * 1024)}`)
    workers[0].sendStarted()
    workers[0].sendBuildSuccess()
    await expect(build).resolves.toMatchObject({ result: { quality: 'full' } })

    expect(stdout.mock.calls.map(call => String(call[0])).join('')).not.toContain('DIRECT_CHILD_STDOUT')
    expect(stderr.mock.calls.map(call => String(call[0])).join('')).not.toContain('DIRECT_CHILD_STDERR')
    expect(supervisor.snapshot().workersJoined).toBe(1)
  })

  it('preserves terminate-and-join when an injected output drain throws during setup', async () => {
    const workers: ControlledWorker[] = []
    const joinGate = deferred<number>()
    const supervisor = new DirectGeometrySupervisor({
      workerFactory: epoch => {
        const worker = new ControlledWorker(epoch)
        Object.defineProperty(worker, 'stdout', {
          value: {
            unpipe: () => { throw new Error('fixture drain failure') },
            resume: () => { throw new Error('fixture resume failure') },
          },
        })
        workers.push(worker)
        return worker as unknown as Worker
      },
    })
    supervisors.add(supervisor)

    const build = supervisor.build('cube(1);', 'full', 'analysis')
    workers[0].setTerminateResult(() => joinGate.promise)
    workers[0].sendStarted()
    workers[0].sendBuildSuccess()
    await flushMicrotasks()
    expect(workers[0].terminateCalls).toBe(1)
    expect(supervisor.snapshot()).toMatchObject({ activeWorkerEpoch: 1, workersJoined: 0 })

    joinGate.resolve(1)
    await expect(build).resolves.toMatchObject({ result: { quality: 'full' } })
    expect(supervisor.snapshot()).toMatchObject({ activeWorkerEpoch: null, workersJoined: 1 })
  })
})
