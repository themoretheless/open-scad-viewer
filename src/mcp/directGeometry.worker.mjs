import { register } from 'tsx/esm/api'

// Node 20 does not activate `--import tsx` inside worker threads because the
// preload runs outside the main thread. Register the TypeScript loader inside
// this disposable realm before importing its real entrypoint.
register()
await import('./directGeometry.worker.ts')
