import {spawnSync} from 'node:child_process'
import {existsSync,mkdtempSync,readFileSync,writeFileSync,rmSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {tmpdir} from 'node:os'
import {dirname,join} from 'node:path'
import {cachedWasmOptimization} from './wasm-opt-cache.mjs'
// Binaryen's size pass after cargo: measured -11% on the geometry kernel, which also keeps it under
// the 8 MB ceiling browsers place on main-thread instantiation. Required, so every build of a
// fingerprinted artifact produces the same bytes on every machine.
const wasmOpt=fileURLToPath(new URL('../node_modules/binaryen/bin/wasm-opt',import.meta.url))
export function optimizeWasm(file) {
  if(!existsSync(wasmOpt))throw new Error('wasm-opt (binaryen) is required to build the kernels: run `npm ci`.')
  const input = readFileSync(file)
  const flags = ['-Oz', '--enable-bulk-memory', '--enable-sign-ext', '--enable-nontrapping-float-to-int',
    '--enable-mutable-globals', '--enable-reference-types', '--enable-multivalue']
  return cachedWasmOptimization({ input, tool: readFileSync(wasmOpt), flags,
    cacheDir: join(dirname(file), '.osv-wasm-opt-cache'), optimize: () => optimizeSnapshot(input, flags) })
}

function optimizeSnapshot(input, flags) {
  const before = input.length
  // Cargo owns the input. Re-optimizing its cached output changes the build
  // recipe on every invocation, so publish bytes from a separate scratch file.
  const temporary = mkdtempSync(join(tmpdir(), 'osv-wasm-opt-'))
  const output = join(temporary, 'optimized.wasm')
  try {
    const snapshot = join(temporary, 'input.wasm')
    writeFileSync(snapshot, input)
    const result = spawnSync(process.execPath, [wasmOpt, ...flags, snapshot, '-o', output], {stdio: 'inherit'})
    if (result.error) throw result.error
    if (result.status !== 0) throw new Error(`wasm-opt failed (${result.signal ?? result.status})`)
    const bytes = readFileSync(output)
    const after = bytes.length
    console.log(`wasm-opt: ${before} -> ${after} bytes (${((1 - after / before) * 100).toFixed(1)}% smaller)`)
    return bytes
  } finally { rmSync(temporary, {recursive: true, force: true}) }
}
