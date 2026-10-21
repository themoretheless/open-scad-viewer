import {defineConfig,mergeConfig} from 'vitest/config'
import {resolve} from 'node:path'
import base from './vitest.config.ts'
const stage=resolve('output/qualification/runout-ui-wasm-2026-10-16/src/generated')
/** Qualify UI code on the new kernel before promoting the viewer artifacts. */
export default mergeConfig(base,defineConfig({resolve:{alias:[
 {find:'../../generated/geometry-kernels/bytes',replacement:resolve(stage,'geometry-kernels/bytes.ts')},
 {find:'../../generated/geometry-kernels/identity',replacement:resolve(stage,'geometry-kernels/identity.ts')},
 {find:'../generated/wasm-brotli/bytes',replacement:resolve(stage,'wasm-brotli/bytes.ts')},
]},test:{maxWorkers:1}}))
