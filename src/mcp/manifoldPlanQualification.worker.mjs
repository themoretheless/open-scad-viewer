import { register } from 'tsx/esm/api'

// Keep qualification workers runnable on every supported Node version. The
// loader must be registered from inside the worker realm on Node 20.
register()
await import('./manifoldPlanQualification.worker.ts')
