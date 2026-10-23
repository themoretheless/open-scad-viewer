import {packPhotogrammetry} from './pack-photogrammetry.mjs'
import {spawnSync} from 'node:child_process'
import {mkdirSync,readFileSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {resolve} from 'node:path'
import {buildWasmBrotli} from './build-wasm-brotli.mjs'
import {reproducibleCargo} from './reproducible-cargo.mjs'
const root=fileURLToPath(new URL('../',import.meta.url))
const cargoTarget=resolve(root,'crates/target')
buildWasmBrotli(root,cargoTarget)
const reproducible=reproducibleCargo(root)
const result=spawnSync('cargo',['build','--locked','--release',...reproducible.args,'--target','wasm32-unknown-unknown','--manifest-path','crates/photogrammetry-wasm/Cargo.toml'],{cwd:root,stdio:'inherit',env:{...process.env,...reproducible.env,CARGO_TARGET_DIR:cargoTarget}})
if(result.error)throw result.error
if(result.status!==0)process.exit(result.status??1)
const bytes=readFileSync(resolve(root,'crates/target/wasm32-unknown-unknown/release/photogrammetry_wasm.wasm'))
const folder=resolve(root,'src/generated/photogrammetry');mkdirSync(folder,{recursive:true})
const publicWasm=resolve(root,'public/wasm');mkdirSync(publicWasm,{recursive:true})
packPhotogrammetry(bytes, folder, publicWasm)
