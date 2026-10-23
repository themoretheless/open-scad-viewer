import {mkdirSync, readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath} from 'node:url'
import {spawnSync} from 'node:child_process'
import {packGeometryKernel} from './pack-geometry-kernel.mjs'
import {packLanguageKernel} from './pack-language-kernel.mjs'
import {packPhotogrammetry} from './pack-photogrammetry.mjs'
import {buildWasmBrotli} from './build-wasm-brotli.mjs'

// Semantic qualification executes the committed release bytes. Rebuilding CAD
// kernels on another host would change tracked assets and invalidate clean-source
// admission; native compilation is covered by the separate Rust CI.
const root=fileURLToPath(new URL('../',import.meta.url))
const published=resolve(root,'public/wasm')
for (const [file,folder,pack] of [
  ['geometry-kernel.wasm','geometry-kernels',packGeometryKernel],
  ['language-kernel.wasm','language-kernel',packLanguageKernel],
  ['photogrammetry.wasm','photogrammetry',packPhotogrammetry],
]) {
  const output=resolve(root,'src/generated',folder)
  mkdirSync(output,{recursive:true})
  pack(readFileSync(resolve(published,file)),output,published)
}
buildWasmBrotli(root,resolve(root,'crates/target'))
for (const script of ['build-vr.mjs','pack-harfbuzz.mjs']) {
  const result=spawnSync(process.execPath,[resolve(root,'scripts',script)],{cwd:root,stdio:'inherit'})
  if(result.error)throw result.error
  if(result.status!==0)throw new Error(`${script} failed (${result.status})`)
}
