import {
  ManifoldPlanQualificationWorkerLane,
  ManifoldPlanQualificationWorkerLaneError,
  ManifoldPlanQualificationWorkerRemoteError,
} from '../../src/services/manifoldPlanQualificationWorkerLane'

interface Check {
  readonly name: string
  readonly passed: boolean
}

const checks: Check[] = []
const outputCandidate = document.querySelector<HTMLElement>('[data-testid="qualification-result"]')
if (outputCandidate === null) throw new Error('Missing qualification result element')
const output: HTMLElement = outputCandidate

function assert(name: string, condition: boolean): void {
  checks.push({ name, passed: condition })
  if (!condition) throw new Error(`Qualification check failed: ${name}`)
}

function fixtureWorker(): Worker {
  return new Worker(new URL('./browser-qualification.worker.ts', import.meta.url), { type: 'module' })
}

async function run(): Promise<void> {
  const defaultLane = new ManifoldPlanQualificationWorkerLane({
    startupTimeoutMs: 10_000,
    deadlineMs: 10_000,
  })
  const moduleName = 'x'.repeat(91)
  const exactBoundarySource = `module ${moduleName}(){cube(1);} ${moduleName}();`
  const exact = await defaultLane.evaluate(exactBoundarySource)
  assert('default Vite Worker exact 256 entity boundary', exact.meshes[0]?.entityId?.length === 256)
  assert('default Vite Worker realm terminated after success', defaultLane.snapshot().workersTerminated === 1)

  let defaultOversizedCode: string | undefined
  try {
    await defaultLane.evaluate('assert(true) if(true) let(x=2) cube(x);')
  } catch (error) {
    if (error instanceof ManifoldPlanQualificationWorkerRemoteError) {
      defaultOversizedCode = error.code
    }
  }
  assert('default Vite Worker rejects 257 identity before publication',
    defaultOversizedCode === 'QUALIFICATION_RESULT_UNPUBLISHABLE')
  const recovered = await defaultLane.evaluate('cube(1);')
  assert('default lane recovers in fresh realm', recovered.volume === 1
    && defaultLane.snapshot().workersStarted === 3
    && defaultLane.snapshot().workersTerminated === 3)

  const controlledLane = new ManifoldPlanQualificationWorkerLane({
    workerFactory: fixtureWorker,
    startupTimeoutMs: 2_000,
    deadlineMs: 80,
    cancellationGraceMs: 0,
  })
  let mainThreadPulse = false
  setTimeout(() => { mainThreadPulse = true }, 10)
  let hangCode: string | undefined
  try {
    await controlledLane.evaluate('hang')
  } catch (error) {
    if (error instanceof ManifoldPlanQualificationWorkerLaneError) hangCode = error.code
  }
  assert('non-cooperative Worker entered execution before deadline', hangCode === 'E_MANIFOLD_PLAN_WORKER_DEADLINE')
  assert('browser main thread remained responsive', mainThreadPulse)
  assert('wedged browser realm termination requested before settlement',
    controlledLane.snapshot().workersTerminated === 1)

  await controlledLane.evaluate('recovery')
  assert('post-quarantine request uses fresh browser realm',
    controlledLane.snapshot().workersStarted === 2
    && controlledLane.snapshot().workersTerminated === 2)

  for (const field of ['entityId', 'instanceId', 'operationId'] as const) {
    const exact = await controlledLane.evaluate(`identity-${field}-256`)
    const mesh = exact.meshes[0]
    const identities = {
      entityId: mesh?.entityId,
      instanceId: mesh?.provenance[0]?.source?.instanceId,
      operationId: mesh?.provenance[0]?.source?.operationId,
    }
    assert(`controlled ${field} length 256 crosses actual Vite Worker boundary`,
      identities[field]?.length === 256)
    for (const other of ['entityId', 'instanceId', 'operationId'] as const) {
      if (other !== field) {
        assert(`${field} exact case leaves ${other} below its boundary`,
          identities[other] !== undefined && identities[other].length < 256)
      }
    }

    let refusalCode: string | undefined
    try {
      await controlledLane.evaluate(`identity-${field}-257`)
    } catch (error) {
      if (error instanceof ManifoldPlanQualificationWorkerRemoteError) refusalCode = error.code
    }
    assert(`controlled ${field} length 257 is bounded before success publication`,
      refusalCode === 'CONTROLLED_RESULT_UNPUBLISHABLE')
    const refusedEpoch = controlledLane.snapshot().lastTerminatedWorkerEpoch
    await controlledLane.evaluate(`recovery-${field}`)
    assert(`${field} refusal recovers in a fresh Vite Worker realm`,
      controlledLane.snapshot().lastTerminatedWorkerEpoch === refusedEpoch + 1)
  }
  assert('all controlled identity realms terminated',
    controlledLane.snapshot().workersStarted === 11
    && controlledLane.snapshot().workersTerminated === 11)

  output.dataset.status = 'passed'
  output.textContent = JSON.stringify({ status: 'passed', checks }, null, 2)
}

void run().catch((error: unknown) => {
  output.dataset.status = 'failed'
  output.textContent = JSON.stringify({
    status: 'failed',
    checks,
    error: error instanceof Error ? { name: error.name, message: error.message } : String(error),
  }, null, 2)
})
