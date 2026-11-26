import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import {mkdirSync,rmSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {resolve} from 'node:path'
import {buildWasmBrotli} from './build-wasm-brotli.mjs'
import {compactWasmTables} from './compact-wasm-tables.mjs'
import {optimizeWasm} from './wasm-optimize.mjs'
import {packGeometryKernel} from './pack-geometry-kernel.mjs'
import {geometryKernelCargo} from './geometry-kernel-cargo.mjs'
const root=fileURLToPath(new URL('../',import.meta.url)),output=resolve(root,'src/generated/geometry-kernels')
const publicWasm=resolve(root,'public/wasm')
const cargoTarget=resolve(root,'crates/target')
buildWasmBrotli(root,cargoTarget)
// Debug symbol names are not used by the browser bridge. The existing
// native optimization profile remains unchanged. WASM defaults to the size profile;
// explicit hot-kernel package profiles remain authoritative. BRep uses the size profile
// only in this WASM build; native release retains its measured hot-loop profile.
// The JSON/ABI bridge also uses the size profile; its native profile is unchanged.
// Pin CARGO_TARGET_DIR so an inherited sandbox/cache target cannot pack a stale wasm.
// Remap source paths so every machine packs the same bytes (see reproducible-cargo.mjs).
const reproducible=geometryKernelCargo(root)
// Favor size for these kernels in the browser bundle. Native release keeps its
// existing hot-loop settings.
const browserSizePackages = ['planar-geometry', 'sdf-core', 'subdivision-core', 'mesh-io', 'cad-predicates', 'mesh-section', 'mesh-query', 'mesh-topology', 'polygon-core']
const browserSizeFlags = browserSizePackages.flatMap(name => ['--config', `profile.release.package.${name}.opt-level="z"`])
const result=spawnSync('cargo',['build','--locked','--release','--config','profile.release.opt-level="z"','--config','profile.release.strip="symbols"','--config','profile.release.package.brep-core.opt-level="z"','--config','profile.release.package.geometry-bridge.opt-level="z"','--config','profile.release.package.nurbs-core.opt-level="z"','--config','profile.release.package.nurbs-numerics.opt-level="z"','--config','profile.release.package.nurbs-geometry.opt-level="z"','--config','profile.release.package.nurbs-foundation.opt-level="z"',...browserSizeFlags,...reproducible.args,'--target','wasm32-unknown-unknown','--manifest-path','crates/geometry-wasm/Cargo.toml'],{cwd:root,stdio:'inherit',env:{...process.env,...reproducible.env,CARGO_TARGET_DIR:cargoTarget}})
if(result.error)throw result.error;if(result.status!==0)process.exit(result.status??1)
mkdirSync(output,{recursive:true})
mkdirSync(publicWasm,{recursive:true})
for(const file of ['kernel.js','kernel.d.ts','kernel_bg.wasm.d.ts'])rmSync(resolve(output,file),{force:true})
const built=resolve(cargoTarget,'wasm32-unknown-unknown/release/geometry_wasm.wasm')
const optimized = optimizeWasm(built)
let wasm = compactWasmTables(optimized)
if (wasm.length > 20 * 1024 * 1024) {
  const second = optimizeWasm(built, {input: optimized})
  const next = compactWasmTables(second)
  if (next.length < wasm.length) wasm = next
  if (wasm.length > 20 * 1024 * 1024) {
    const merged = compactWasmTables(optimizeWasm(built, {input: second, mergeSimilar: true}))
    if (merged.length < wasm.length) wasm = merged
    if (wasm.length > 20 * 1024 * 1024) {
      const folded = compactWasmTables(optimizeWasm(built, {input: second, mergeSimilar: true, foldCode: true}))
      if (folded.length < wasm.length) wasm = folded
    }
  }
}
console.log(`geometry-kernels artifact: sha256=${createHash('sha256').update(wasm).digest('hex')} bytes=${wasm.byteLength}`)
// Skip WebAssembly.Module compilation, brotli quality 11 and base85 encoding
// when the optimized wasm is byte-identical to the last packaged run.
packGeometryKernel(wasm,output,publicWasm)
