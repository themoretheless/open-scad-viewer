// Thin wrapper kept for existing invocation points; the implementation lives
// in the shared gpu-probe-runner.mjs.
import { runGpuProbe } from './gpu-probe-runner.mjs'

await runGpuProbe('sdf-probe.html')
