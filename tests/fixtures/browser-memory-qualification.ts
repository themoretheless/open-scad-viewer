import { ManifoldPlanQualificationWorkerLane } from '../../src/services/manifoldPlanQualificationWorkerLane'

interface BrowserMemoryJobResult {
  readonly workerEpoch: number
  readonly workersStarted: number
  readonly workersTerminated: number
}

interface BrowserMemoryQualificationController {
  readonly runJob: () => Promise<BrowserMemoryJobResult>
}

declare global {
  interface Window {
    __manifoldPlanMemoryQualification?: BrowserMemoryQualificationController
  }
}

const output = document.querySelector<HTMLElement>('[data-testid="memory-qualification-status"]')
if (output === null) throw new Error('Missing browser memory qualification status element')

const lane = new ManifoldPlanQualificationWorkerLane({
  startupTimeoutMs: 10_000,
  deadlineMs: 10_000,
  cancellationGraceMs: 0,
})

function animationFrame(): Promise<void> {
  return new Promise(resolve => requestAnimationFrame(() => resolve()))
}

async function runJob(): Promise<BrowserMemoryJobResult> {
  const before = lane.snapshot()
  const result = await lane.evaluate('cube(1);', 'full')
  const settled = lane.snapshot()
  if (result.volume !== 1 || result.surfaceArea !== 6 || result.meshes.length !== 1) {
    throw new Error('Qualification Worker returned an unexpected cube result')
  }
  if (settled.activeWorkerEpoch !== null
    || settled.workersStarted !== before.workersStarted + 1
    || settled.workersTerminated !== before.workersTerminated + 1
    || settled.lastTerminatedWorkerEpoch <= before.lastTerminatedWorkerEpoch) {
    throw new Error('Qualification Worker did not terminate before promise settlement')
  }

  // The Node-side RSS sampler runs only after this promise resolves. These
  // waits are therefore part of the frozen post-termination sampling point.
  await animationFrame()
  await animationFrame()
  await new Promise<void>(resolve => setTimeout(resolve, 0))

  return {
    workerEpoch: settled.lastTerminatedWorkerEpoch,
    workersStarted: settled.workersStarted,
    workersTerminated: settled.workersTerminated,
  }
}

window.__manifoldPlanMemoryQualification = Object.freeze({ runJob })
output.dataset.status = 'ready'
output.textContent = 'ready'
