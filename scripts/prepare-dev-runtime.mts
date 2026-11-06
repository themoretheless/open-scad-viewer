import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { CURRENT_RUNTIME_GEOMETRY_EVIDENCE } from '../src/core/currentRuntimeGeometryEvidence'
// @ts-expect-error Packaging is a Node build script without a declaration file.
import { packGeometryKernel } from './pack-geometry-kernel.mjs'

// Use the exact archived artifact admitted by the runtime catalog. Local Rust
// candidates remain in crates/target; starting the UI does not qualify them.
const hash = CURRENT_RUNTIME_GEOMETRY_EVIDENCE.wasmSha256
const artifact = readFileSync(new URL(`../docs/qualification/artifacts/geometry-${hash}.wasm`, import.meta.url))
if (createHash('sha256').update(artifact).digest('hex') !== hash) {
  throw new Error('Archived development runtime does not match its qualification fingerprint')
}
packGeometryKernel(artifact,
  fileURLToPath(new URL('../src/generated/geometry-kernels', import.meta.url)),
  fileURLToPath(new URL('../public/wasm', import.meta.url)))
