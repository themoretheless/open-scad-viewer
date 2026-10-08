import {spawnSync} from 'node:child_process'
import {existsSync,mkdtempSync,readFileSync,writeFileSync,rmSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {tmpdir} from 'node:os'
import {dirname,join,isAbsolute} from 'node:path'
import {cachedWasmOptimization} from './wasm-opt-cache.mjs'
// Binaryen's size pass after cargo: measured -11% on the geometry kernel, which also keeps it under
// the 8 MB ceiling browsers place on main-thread instantiation. Required, so every build of a
// fingerprinted artifact produces the same bytes on every machine.
const wasmOpt=fileURLToPath(new URL('../node_modules/binaryen/bin/wasm-opt',import.meta.url))
export function optimizeWasm(file, {nativePath = process.env.OSV_WASM_OPT_NATIVE} = {}) {
  if(!existsSync(wasmOpt))throw new Error('wasm-opt (binaryen) is required to build the kernels: run `npm ci`.')
  const driver = optimizerDriver(nativePath)
  const input = readFileSync(file)
  const flags = ['-Oz', '--enable-bulk-memory', '--enable-sign-ext', '--enable-nontrapping-float-to-int',
    '--enable-mutable-globals', '--enable-reference-types', '--enable-multivalue', '--enable-simd']
  return cachedWasmOptimization({ input, tool: readFileSync(driver.path), flags,
    cacheDir: join(dirname(file), '.osv-wasm-opt-cache'), optimize: () => optimizeSnapshot(input, flags, driver) })
}

function optimizerDriver(nativePath) {
  if (!nativePath) return {path: wasmOpt, executable: process.execPath, prefix: [wasmOpt]}
  if (!isAbsolute(nativePath) || !existsSync(nativePath)) throw new Error('OSV_WASM_OPT_NATIVE must name an existing absolute executable path')
  const expected = JSON.parse(readFileSync(fileURLToPath(new URL('../node_modules/binaryen/package.json',import.meta.url)), 'utf8')).version.split('.')[0]
  const version = spawnSync(nativePath, ['--version'], {encoding: 'utf8'})
  if (version.error) throw version.error
  const actual = /wasm-opt version (\d+)\b/.exec(version.stdout)?.[1]
  if (version.status !== 0 || actual !== expected) throw new Error(`Native wasm-opt version must match Binaryen ${expected}; received ${actual ?? 'unknown'}`)
  return {path: nativePath, executable: nativePath, prefix: []}
}

function optimizeSnapshot(input, flags, driver) {
  const before = input.length
  // Cargo owns the input. Re-optimizing its cached output changes the build
  // recipe on every invocation, so publish bytes from a separate scratch file.
  const temporary = mkdtempSync(join(tmpdir(), 'osv-wasm-opt-'))
  const output = join(temporary, 'optimized.wasm')
  try {
    const snapshot = join(temporary, 'input.wasm')
    writeFileSync(snapshot, input)
    const result = spawnSync(driver.executable, [...driver.prefix, ...flags, snapshot, '-o', output], {stdio: 'inherit'})
    if (result.error) throw result.error
    if (result.status !== 0) throw new Error(`wasm-opt failed (${result.signal ?? result.status})`)
    const bytes = readFileSync(output)
    const after = bytes.length
    console.log(`wasm-opt: ${before} -> ${after} bytes (${((1 - after / before) * 100).toFixed(1)}% smaller)`)
    return bytes
  } finally { rmSync(temporary, {recursive: true, force: true}) }
}
