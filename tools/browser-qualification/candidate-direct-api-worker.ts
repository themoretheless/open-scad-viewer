/// <reference lib="webworker" />
const workerScope = self as DedicatedWorkerGlobalScope
workerScope.onmessage = async event => {
  try {
    const {module, identity} = event.data
    ;(globalThis as typeof globalThis & {__G1_SHARED_GEOMETRY_MODULE__: boolean})
      .__G1_SHARED_GEOMETRY_MODULE__ = true
    const kernel = await import('../../src/kernel/geometry/kernel')
    await kernel.installGeometryKernelFromTrustedHost(module, identity)
    const {runCandidateFixtures} = await import('./candidate-direct-api-fixtures')
    workerScope.postMessage({ok: true, identity, result: runCandidateFixtures()})
  } catch (error) {
    workerScope.postMessage({ok: false, error: String(error)})
  }
}
export {}
