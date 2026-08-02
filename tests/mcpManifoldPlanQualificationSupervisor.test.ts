import { Worker } from 'node:worker_threads'
import { describe, expect, it, vi } from 'vitest'
import {
  McpManifoldPlanQualificationSupervisor,
  McpManifoldPlanRemoteError,
  McpManifoldPlanSupervisorError,
} from '../src/mcp/manifoldPlanQualificationSupervisor'

const fixtureUrl = new URL('./fixtures/mcp-manifold-plan-qualification.worker.mjs', import.meta.url)
const webWorkerHarnessUrl = new URL('./fixtures/web-worker-node-harness.mjs', import.meta.url)
const controlledIdentityWorkerUrl = new URL(
  './fixtures/browser-qualification.worker.ts',
  import.meta.url,
)

function fixtureSupervisor(
  options: {
    startupTimeoutMs?: number
    deadlineMs?: number
    cancellationGraceMs?: number
    joinTimeoutMs?: number
  } = {},
): McpManifoldPlanQualificationSupervisor {
  return new McpManifoldPlanQualificationSupervisor({
    workerFactory: () => new Worker(fixtureUrl, { stdout: true, stderr: true }),
    startupTimeoutMs: options.startupTimeoutMs ?? 1_000,
    deadlineMs: options.deadlineMs ?? 1_000,
    cancellationGraceMs: options.cancellationGraceMs ?? 5,
    joinTimeoutMs: options.joinTimeoutMs ?? 1_000,
  })
}

function controlledIdentitySupervisor(): McpManifoldPlanQualificationSupervisor {
  return new McpManifoldPlanQualificationSupervisor({
    workerFactory: () => new Worker(webWorkerHarnessUrl, {
      workerData: { entryUrl: controlledIdentityWorkerUrl.href, announceReady: false },
      stdout: true,
      stderr: true,
    }),
    startupTimeoutMs: 10_000,
    deadlineMs: 10_000,
    cancellationGraceMs: 10,
    joinTimeoutMs: 10_000,
  })
}

describe('MCP Manifold-plan qualification supervisor', () => {
  it('uses a distinct started-handshake timeout and recovers only after joining that child', async () => {
    const supervisor = fixtureSupervisor({
      startupTimeoutMs: 40,
      deadlineMs: 1_000,
      cancellationGraceMs: 5,
    })

    await expect(supervisor.evaluate('startup-hang')).rejects.toMatchObject({
      code: 'E_MCP_MANIFOLD_PLAN_STARTUP',
      workerEpoch: 1,
    })
    expect(supervisor.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 1,
      workersJoined: 1,
      quarantined: false,
    })
    await expect(supervisor.evaluate('success')).resolves.toMatchObject({ quality: 'full' })
    expect(supervisor.snapshot().lastJoinedWorkerEpoch).toBe(2)
  })

  it('rejects a correlated terminal sent before the exact started handshake', async () => {
    const supervisor = fixtureSupervisor()
    await expect(supervisor.evaluate('terminal-before-started')).rejects.toMatchObject({
      code: 'E_MCP_MANIFOLD_PLAN_PROTOCOL',
      workerEpoch: 1,
      message: 'MCP Manifold qualification child did not provide the exact started handshake',
    })
    expect(supervisor.snapshot()).toMatchObject({
      lastJoinedWorkerEpoch: 1,
      workersJoined: 1,
      quarantined: false,
    })
  })

  it('hard-kills and joins a non-cooperative child at its bounded deadline while the parent stays responsive', async () => {
    const supervisor = fixtureSupervisor({ deadlineMs: 40, cancellationGraceMs: 5 })
    let parentPing = false
    const ping = new Promise<void>(resolve => {
      setTimeout(() => {
        parentPing = true
        resolve()
      }, 5)
    })
    const startedAt = performance.now()
    const evaluation = supervisor.evaluate('hang')

    await ping
    expect(parentPing).toBe(true)
    await expect(evaluation).rejects.toMatchObject({
      code: 'E_MCP_MANIFOLD_PLAN_DEADLINE',
      workerEpoch: 1,
    })
    expect(performance.now() - startedAt).toBeLessThan(1_000)
    expect(supervisor.snapshot()).toEqual({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 1,
      workersStarted: 1,
      workersJoined: 1,
      quarantined: false,
    })
  })

  it('selects cancellation once, joins before releasing admission, and recovers in a fresh epoch', async () => {
    const supervisor = fixtureSupervisor({ cancellationGraceMs: 5 })
    const controller = new AbortController()
    const cancelled = supervisor.evaluate('slow-success', 'full', { signal: controller.signal })
    setTimeout(() => controller.abort(), 10)

    await expect(cancelled).rejects.toMatchObject({
      code: 'E_MCP_MANIFOLD_PLAN_CANCELLED',
      workerEpoch: 1,
    })
    expect(supervisor.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 1,
      workersStarted: 1,
      workersJoined: 1,
    })

    await expect(supervisor.evaluate('success')).resolves.toMatchObject({
      quality: 'full',
      meshes: [],
    })
    expect(supervisor.snapshot()).toEqual({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 2,
      workersStarted: 2,
      workersJoined: 2,
      quarantined: false,
    })
  })

  it('rejects uncorrelated identity and a child error only after the disposable realm is joined', async () => {
    const supervisor = fixtureSupervisor()
    await expect(supervisor.evaluate('mutated-identity')).rejects.toBeInstanceOf(
      McpManifoldPlanSupervisorError,
    )
    expect(supervisor.snapshot().lastJoinedWorkerEpoch).toBe(1)

    let failure: unknown
    try {
      await supervisor.evaluate('failure')
    } catch (error) {
      failure = error
    }
    expect(failure).toBeInstanceOf(McpManifoldPlanRemoteError)
    expect(failure).toMatchObject({
      name: 'FixtureKernelError',
      message: 'fixture failure',
      code: 'E_FIXTURE_KERNEL',
    })
    expect(supervisor.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 2,
      workersStarted: 2,
      workersJoined: 2,
    })
  })

  it('accepts only the first correlated terminal and fences a late second message by epoch teardown', async () => {
    const supervisor = fixtureSupervisor()
    let settlements = 0
    const result = await supervisor.evaluate('double-terminal').then(value => {
      settlements++
      return value
    }, error => {
      settlements++
      throw error
    })
    expect(result).toMatchObject({ quality: 'full', meshes: [] })
    await new Promise(resolve => setTimeout(resolve, 20))
    expect(settlements).toBe(1)
    expect(supervisor.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 1,
      workersJoined: 1,
    })
  })

  it('treats messageerror as a protocol failure and joins before rejection', async () => {
    const supervisor = new McpManifoldPlanQualificationSupervisor({
      workerFactory: () => {
        const worker = new Worker(fixtureUrl, { stdout: true, stderr: true })
        setTimeout(() => worker.emit('messageerror', new Error('fixture deserialize failure')), 15)
        return worker
      },
      startupTimeoutMs: 1_000,
      deadlineMs: 1_000,
      cancellationGraceMs: 5,
      joinTimeoutMs: 1_000,
    })

    await expect(supervisor.evaluate('hang')).rejects.toMatchObject({
      code: 'E_MCP_MANIFOLD_PLAN_PROTOCOL',
      message: 'MCP Manifold qualification child message could not be deserialized',
      workerEpoch: 1,
    })
    expect(supervisor.snapshot()).toMatchObject({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 1,
      workersJoined: 1,
      quarantined: false,
    })
  })

  it('drains noisy child stdout and stderr without forwarding either stream to the parent', async () => {
    const stdout = vi.spyOn(process.stdout, 'write')
    const stderr = vi.spyOn(process.stderr, 'write')
    try {
      await expect(fixtureSupervisor().evaluate('noisy-success')).resolves.toMatchObject({ quality: 'full' })
      const stdoutText = stdout.mock.calls.map(call => String(call[0])).join('')
      const stderrText = stderr.mock.calls.map(call => String(call[0])).join('')
      expect(stdoutText).not.toContain('MCP_CHILD_STDOUT_MARKER')
      expect(stderrText).not.toContain('MCP_CHILD_STDERR_MARKER')
    } finally {
      stdout.mockRestore()
      stderr.mockRestore()
    }
  })

  it('rejects results above the smaller qualification IPC mesh cap and recovers in a fresh child', async () => {
    const supervisor = fixtureSupervisor()
    await expect(supervisor.evaluate('oversized-ipc')).rejects.toMatchObject({
      code: 'E_MCP_MANIFOLD_PLAN_PROTOCOL',
      workerEpoch: 1,
    })
    expect(supervisor.snapshot().lastJoinedWorkerEpoch).toBe(1)
    await expect(supervisor.evaluate('success')).resolves.toMatchObject({ meshes: [] })
    expect(supervisor.snapshot().lastJoinedWorkerEpoch).toBe(2)
  })

  it('bounds join, permanently quarantines on timeout, and admits no further child', async () => {
    let cleanup: (() => Promise<number>) | undefined
    let factoryCalls = 0
    const supervisor = new McpManifoldPlanQualificationSupervisor({
      workerFactory: () => {
        factoryCalls++
        const worker = new Worker(fixtureUrl, { stdout: true, stderr: true })
        cleanup = worker.terminate.bind(worker)
        Object.defineProperty(worker, 'terminate', {
          configurable: true,
          value: () => new Promise<number>(() => undefined),
        })
        return worker
      },
      startupTimeoutMs: 1_000,
      deadlineMs: 1_000,
      cancellationGraceMs: 5,
      joinTimeoutMs: 25,
    })

    try {
      await expect(supervisor.evaluate('success')).rejects.toMatchObject({
        code: 'E_MCP_MANIFOLD_PLAN_JOIN',
        message: 'MCP Manifold qualification child did not join within 25 ms',
        workerEpoch: 1,
      })
      expect(supervisor.snapshot()).toEqual({
        activeWorkerEpoch: 1,
        lastJoinedWorkerEpoch: 0,
        workersStarted: 1,
        workersJoined: 0,
        quarantined: true,
      })
      await expect(supervisor.evaluate('success')).rejects.toMatchObject({
        code: 'E_MCP_MANIFOLD_PLAN_QUARANTINED',
        workerEpoch: null,
      })
      expect(factoryCalls).toBe(1)
    } finally {
      await cleanup?.()
    }
  })

  it('runs the real SemanticProgram-to-Manifold adapter in disposable MCP child realms', async () => {
    const supervisor = new McpManifoldPlanQualificationSupervisor({
      deadlineMs: 10_000,
      cancellationGraceMs: 10,
    })
    await expect(supervisor.evaluate('cube(1);')).resolves.toMatchObject({
      quality: 'full',
      meshes: [expect.objectContaining({ entityId: expect.stringMatching(/^entity:/) })],
      volume: 1,
      surfaceArea: 6,
    })
    await expect(supervisor.evaluate('cube(2);')).resolves.toMatchObject({
      quality: 'full',
      volume: 8,
      surfaceArea: 24,
    })
    expect(supervisor.snapshot()).toEqual({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 2,
      workersStarted: 2,
      workersJoined: 2,
      quarantined: false,
    })
  }, 30_000)

  it('preflights real plan-route identity 256/257 results inside the child and recovers in a fresh realm', async () => {
    const supervisor = new McpManifoldPlanQualificationSupervisor({
      deadlineMs: 10_000,
      cancellationGraceMs: 10,
    })
    const moduleName = 'x'.repeat(91)
    const exactBoundarySource = `module ${moduleName}(){cube(1);} ${moduleName}();`
    const exact = await supervisor.evaluate(exactBoundarySource)
    expect(exact.meshes[0].entityId).toHaveLength(256)
    expect(exact.meshes[0].provenance[0].source?.instanceId).toHaveLength(256)

    await expect(supervisor.evaluate(
      'assert(true) if(true) let(x=2) cube(x);',
    )).rejects.toMatchObject({
      name: 'ManifoldPlanQualificationProtocolError',
      code: 'QUALIFICATION_RESULT_UNPUBLISHABLE',
      message: 'Manifold plan result cannot be published by the qualification child',
    })

    await expect(supervisor.evaluate('cube(1);')).resolves.toMatchObject({
      meshes: [expect.objectContaining({ entityId: expect.stringMatching(/^entity:/) })],
      volume: 1,
    })
    expect(supervisor.snapshot()).toEqual({
      activeWorkerEpoch: null,
      lastJoinedWorkerEpoch: 3,
      workersStarted: 3,
      workersJoined: 3,
      quarantined: false,
    })
  }, 30_000)

  it.each(['entityId', 'instanceId', 'operationId'] as const)(
    'isolates controlled %s length 256/257 across the MCP child boundary',
    async field => {
      const supervisor = controlledIdentitySupervisor()
      const exact = await supervisor.evaluate(`identity-${field}-256`)
      const mesh = exact.meshes[0]
      const identities = {
        entityId: mesh.entityId,
        instanceId: mesh.provenance[0]?.source?.instanceId,
        operationId: mesh.provenance[0]?.source?.operationId,
      }
      expect(identities[field]).toHaveLength(256)
      for (const other of ['entityId', 'instanceId', 'operationId'] as const) {
        if (other !== field) expect(identities[other]!.length).toBeLessThan(256)
      }

      await expect(supervisor.evaluate(`identity-${field}-257`)).rejects.toMatchObject({
        name: 'ControlledQualificationProtocolError',
        code: 'CONTROLLED_RESULT_UNPUBLISHABLE',
        message: 'Controlled result cannot cross the qualification Worker boundary',
      })
      expect(supervisor.snapshot()).toMatchObject({
        activeWorkerEpoch: null,
        lastJoinedWorkerEpoch: 2,
        workersStarted: 2,
        workersJoined: 2,
        quarantined: false,
      })

      await expect(supervisor.evaluate(`recovery-${field}`)).resolves.toMatchObject({
        meshes: [],
        quality: 'full',
      })
      expect(supervisor.snapshot()).toEqual({
        activeWorkerEpoch: null,
        lastJoinedWorkerEpoch: 3,
        workersStarted: 3,
        workersJoined: 3,
        quarantined: false,
      })
    },
    30_000,
  )
})
