import {spawnSync} from 'node:child_process'
import {mkdirSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {resolve} from 'node:path'
import {packLanguageKernel} from './pack-language-kernel.mjs'
import {optimizeWasm} from './wasm-optimize.mjs'
import {reproducibleCargo} from './reproducible-cargo.mjs'
// The OpenSCAD and RushGraph frontends ship separately from the geometry kernel: a session that never
// builds source never downloads them, and the geometry module stays under the browsers' main-thread
// instantiation ceiling. Same transport and symbol policy as build-geometry-kernels.mjs.
const root=fileURLToPath(new URL('../',import.meta.url)),output=resolve(root,'src/generated/language-kernel')
const publicWasm=resolve(root,'public/wasm')
const cargoTarget=resolve(root,'crates/target')
const reproducible=reproducibleCargo(root)
const result=spawnSync('cargo',['build','--locked','--release','--config','profile.release.strip="symbols"',...reproducible.args,'--target','wasm32-unknown-unknown','--manifest-path','crates/languages-wasm/Cargo.toml'],{cwd:root,stdio:'inherit',env:{...process.env,...reproducible.env,CARGO_TARGET_DIR:cargoTarget}})
if(result.error)throw result.error;if(result.status!==0)process.exit(result.status??1)
mkdirSync(output,{recursive:true})
mkdirSync(publicWasm,{recursive:true})
const built=resolve(cargoTarget,'wasm32-unknown-unknown/release/languages_wasm.wasm')
const wasm=optimizeWasm(built)
packLanguageKernel(wasm, output, publicWasm)
