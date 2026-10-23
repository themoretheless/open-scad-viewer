import { register } from 'tsx/esm/api'
import { setFlagsFromString } from 'node:v8'

// Node 20 ARM64 can leave uncancellable optimizing WASM jobs in V8's shared
// pool after worker termination, blocking process exit. Disposable workers on
// this host use Liftoff-only compilation; newer Node and other hosts retain JIT.
if (process.versions.node.startsWith('20.') && process.arch === 'arm64') {
  setFlagsFromString('--liftoff-only')
}

// Node 20 does not activate `--import tsx` inside worker threads because the
// preload runs outside the main thread. Register the TypeScript loader inside
// this disposable realm before importing its real entrypoint.
register()
await import('./directGeometry.worker.ts')
