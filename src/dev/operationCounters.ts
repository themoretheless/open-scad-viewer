/**
 * Cheap process-wide operation counters for browser smoke budgets.
 *
 * Counting is always on (one Map increment per event); the counters are only
 * exposed on `window` when the test seam asks for it (see `testSeam.ts`).
 */
export type OperationCounter =
  | 'solidWorkerRequests'
  | 'geometryWorkerRequests'
  | 'documentClones'
  | 'historyCommits'
  | 'historyRestores'
  | 'viewportSceneUploads'
  | 'viewportFrames'

const counts = new Map<OperationCounter, number>()

export function countOperation(name: OperationCounter, by = 1): void {
  counts.set(name, (counts.get(name) ?? 0) + by)
}

export function operationCounts(): Record<OperationCounter, number> {
  return {
    solidWorkerRequests: counts.get('solidWorkerRequests') ?? 0,
    geometryWorkerRequests: counts.get('geometryWorkerRequests') ?? 0,
    documentClones: counts.get('documentClones') ?? 0,
    historyCommits: counts.get('historyCommits') ?? 0,
    historyRestores: counts.get('historyRestores') ?? 0,
    viewportSceneUploads: counts.get('viewportSceneUploads') ?? 0,
    viewportFrames: counts.get('viewportFrames') ?? 0,
  }
}

export function resetOperationCounts(): void {
  counts.clear()
}
