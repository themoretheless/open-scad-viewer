import {workerData} from 'node:worker_threads'
import {register} from 'tsx/esm/api'

register()
const {defaultGeometryKernel} = await import('../../src/services/cadGeometryKernel.ts')
const warm = defaultGeometryKernel.warm.bind(defaultGeometryKernel)
let pending
defaultGeometryKernel.warm = () => {
  pending ??= new Promise(resolve => setTimeout(resolve, workerData.delayMs)).then(warm)
  return pending
}
await import('../../src/mcp/directGeometry.worker.ts')
