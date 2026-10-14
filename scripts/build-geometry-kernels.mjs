import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import {mkdirSync,rmSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {resolve} from 'node:path'
import {buildWasmBrotli} from './build-wasm-brotli.mjs'
import {optimizeWasm} from './wasm-optimize.mjs'
import {packGeometryKernel} from './pack-geometry-kernel.mjs'
import {reproducibleCargo} from './reproducible-cargo.mjs'
const root=fileURLToPath(new URL('../',import.meta.url)),output=resolve(root,'src/generated/geometry-kernels')
const publicWasm=resolve(root,'public/wasm')
const cargoTarget=resolve(root,'crates/target')
buildWasmBrotli(root,cargoTarget)
// Debug symbol names are not used by the browser bridge. The existing
// native optimization profile remains unchanged. BRep uses the size profile
// only in this WASM build; native release retains its measured hot-loop profile.
// The JSON/ABI bridge also uses the size profile; its native profile is unchanged.
// Pin CARGO_TARGET_DIR so an inherited sandbox/cache target cannot pack a stale wasm.
// Remap source paths so every machine packs the same bytes (see reproducible-cargo.mjs).
const reproducible=reproducibleCargo(root)
const result=spawnSync('cargo',['build','--locked','--release','--config','profile.release.strip="symbols"','--config','profile.release.package.brep-core.opt-level="z"','--config','profile.release.package.geometry-bridge.opt-level="z"',...reproducible.args,'--target','wasm32-unknown-unknown','--manifest-path','crates/geometry-wasm/Cargo.toml'],{cwd:root,stdio:'inherit',env:{...process.env,...reproducible.env,CARGO_TARGET_DIR:cargoTarget}})
if(result.error)throw result.error;if(result.status!==0)process.exit(result.status??1)
mkdirSync(output,{recursive:true})
mkdirSync(publicWasm,{recursive:true})
for(const file of ['kernel.js','kernel.d.ts','kernel_bg.wasm.d.ts'])rmSync(resolve(output,file),{force:true})
const built=resolve(cargoTarget,'wasm32-unknown-unknown/release/geometry_wasm.wasm')
const wasm=optimizeWasm(built)
console.log(`geometry-kernels artifact: sha256=${createHash('sha256').update(wasm).digest('hex')} bytes=${wasm.byteLength}`)
// Skip WebAssembly.Module compilation, brotli quality 11 and base85 encoding
// when the optimized wasm is byte-identical to the last packaged run.
packGeometryKernel(wasm,output,publicWasm)
