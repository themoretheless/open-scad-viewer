import { register } from 'tsx/esm/api'
import { setFlagsFromString } from 'node:v8'

// Node 20 ARM64 can leave uncancellable optimizing WASM jobs in V8's shared
// pool after worker termination, blocking process exit. Disposable workers on
// this host use Liftoff-only compilation; newer Node and other hosts retain JIT.
if (process.versions.node.startsWith('20.') && process.arch === 'arm64') {
  setFlagsFromString('--liftoff-only')
}

// Keep qualification workers runnable on every supported Node version. The
// loader must be registered from inside the worker realm on Node 20.
register()
await import('./manifoldPlanQualification.worker.ts')
