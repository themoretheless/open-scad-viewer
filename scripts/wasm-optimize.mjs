import {spawnSync} from 'node:child_process'
import {existsSync,statSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
// Binaryen's size pass after cargo: measured -11% on the geometry kernel, which also keeps it under
// the 8 MB ceiling browsers place on main-thread instantiation. Required, so every build of a
// fingerprinted artifact produces the same bytes on every machine.
const wasmOpt=fileURLToPath(new URL('../node_modules/binaryen/bin/wasm-opt',import.meta.url))
export function optimizeWasm(file) {
  if(!existsSync(wasmOpt))throw new Error('wasm-opt (binaryen) is required to build the kernels: run `npm ci`.')
  const before = statSync(file).size
  const result = spawnSync(process.execPath, [wasmOpt,
    '-Oz',
    '--enable-bulk-memory',
    '--enable-sign-ext',
    '--enable-nontrapping-float-to-int',
    '--enable-mutable-globals',
    '--enable-reference-types',
    '--enable-multivalue',
    file,
    '-o',
    file,
  ], {stdio: 'inherit'})
  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
  const after = statSync(file).size
  console.log(`wasm-opt: ${before} -> ${after} bytes (${((1 - after / before) * 100).toFixed(1)}% smaller)`)
}
