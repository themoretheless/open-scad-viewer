import {spawnSync} from 'node:child_process'
import {statSync} from 'node:fs'
// Binaryen's size pass after cargo: measured -11% on the geometry kernel, which also keeps it under
// the 8 MB ceiling browsers place on main-thread instantiation. Required, so every build of a
// fingerprinted artifact produces the same bytes on every machine.
export function optimizeWasm(file) {
  const before = statSync(file).size
  const result = spawnSync('wasm-opt', [
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
  if (result.error?.code === 'ENOENT') {
    throw new Error('wasm-opt (binaryen) is required to build the kernels: install binaryen, or `cargo install wasm-opt`.')
  }
  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
  const after = statSync(file).size
  console.log(`wasm-opt: ${before} -> ${after} bytes (${((1 - after / before) * 100).toFixed(1)}% smaller)`)
}
