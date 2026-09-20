import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {compileOpenSCAD, TT} from '../src/services/openscadCompiler'
import {scadCompileRust, warmLanguageKernel} from '../src/services/languages/kernel'

const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const profile = 'openscad/stable-2021.01'
function wireAst(value: unknown): unknown {
  if (typeof value === 'number' && !Number.isFinite(value)) return {$number: String(value)}
  if (Array.isArray(value)) return value.map(wireAst)
  if (value !== null && typeof value === 'object') {
    const node = value as Record<string, unknown>
    return Object.fromEntries(Object.entries(node).flatMap(([key, item]) => {
      if (key === 'op' && (node.kind === 'binary' || node.kind === 'unary')) return [[key, TT[item as TT]]]
      if (item === undefined) return node.kind === 'literal' && key === 'value' ? [[key, null]] : []
      return [[key, wireAst(item)]]
    }))
  }
  return value
}

const startup = performance.now()
await warmLanguageKernel()
const warmupMs = performance.now() - startup
const samples = []
for (const count of [1, 100, 500]) {
  const source = Array.from({length: count}, (_, i) => `translate([${i},0,0]) cube([1+2,3,4]);`).join('\n')
  const expected = wireAst(compileOpenSCAD(source, {languageProfile: profile}))
  const ts = () => compileOpenSCAD(source, {languageProfile: profile})
  const rust = () => scadCompileRust(source, profile)
  const verifyRust = (result: ReturnType<typeof rust>) => {
    assert.equal(result.ok, true)
    if (result.ok) assert.deepEqual(result.ast, expected)
  }
  for (let i = 0; i < 5; i++) {
    assert.deepEqual(wireAst(ts()), expected)
    verifyRust(rust())
  }
  const times = {typescript: [] as number[], rustWasm: [] as number[]}
  for (let i = 0; i < 21; i++) {
    for (const variant of i % 2 ? ['rustWasm', 'typescript'] as const : ['typescript', 'rustWasm'] as const) {
      const start = performance.now()
      if (variant === 'typescript') {
        const result = ts()
        times.typescript.push(performance.now() - start)
        assert.deepEqual(wireAst(result), expected)
      } else {
        const result = rust()
        times.rustWasm.push(performance.now() - start)
        verifyRust(result)
      }
    }
  }
  const stats = (values: number[]) => ({medianMs: [...values].sort((a,b) => a-b)[10], samplesMs: values})
  samples.push({calls: count * 2, sourceLength: source.length, sourceSha256: hash(source),
    typescript: stats(times.typescript), rustWasm: stats(times.rustWasm)})
}
const inputs = ['src/services/openscadCompiler.ts', 'src/services/languages/kernel.ts',
  'src/generated/language-kernel/bytes.ts', 'benchmarks/openscad-frontend.mts']
console.log(JSON.stringify({node: process.version, arch: process.arch, platform: process.platform,
  method: '5 warmups, 21 alternating samples. Full AST parity outside timing. Rust includes request encoding, WASM parsing/serialization, response decoding and ABI frees. TS returns its native AST. Includes allocations/GC; excludes imports and cold initialization. Not a geometry or browser benchmark.',
  coldLanguageInitializationMs: warmupMs,
  fingerprints: Object.fromEntries(inputs.map(path => [path, hash(readFileSync(new URL(`../${path}`, import.meta.url)))])),
  samples}, null, 2))
