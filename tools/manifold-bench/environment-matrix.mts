import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  booleanManifoldInKernel,
  checkManifoldInKernel,
  metricsManifoldInKernel,
  repairManifoldInKernel,
  type ManifoldBooleanOp,
} from '../../src/services/geometry/manifoldAnalysis'

interface MeshCase { id:string; positions:number[]; indices:number[] }
interface BooleanPair { id:string; a:string; b:string; operations:ManifoldBooleanOp[] }
interface Corpus { schemaVersion:1; cases:MeshCase[]; booleanPairs:BooleanPair[] }
interface MatrixRecord { id:string; operation:string; precision:'f32'|'f64'; result:unknown }
interface MatrixRun { schemaVersion:1; environment:Record<string,unknown>; results:MatrixRecord[] }

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const corpusBytes = await readFile(path.join(root, 'tools/manifold-bench/environment-corpus.json'))
const corpus = JSON.parse(corpusBytes.toString()) as Corpus
if (corpus.schemaVersion !== 1 || !corpus.cases.length || !corpus.booleanPairs.length) throw new Error('Invalid environment corpus')

function positions(values:number[], precision:'f32'|'f64') {
  return precision === 'f32' ? new Float32Array(values) : new Float64Array(values)
}
function plain(value:unknown):unknown {
  if (ArrayBuffer.isView(value)) return Array.from(value as unknown as ArrayLike<number>)
  if (Array.isArray(value)) return value.map(plain)
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, plain(entry)]))
  return value
}
function canonicalMeshResult(result:Record<string,unknown>) {
  const output = plain(result) as Record<string,unknown>
  const indices = output.indices as number[]
  if (!Array.isArray(indices) || indices.length % 3 !== 0) throw new Error('Invalid triangle indices')
  const canonical:number[] = []
  for (let offset = 0; offset < indices.length; offset += 3) {
    const triangle = indices.slice(offset, offset + 3)
    let start = 0
    for (let index = 1; index < 3; index++) if (triangle[index] < triangle[start]) start = index
    for (let index = 0; index < 3; index++) canonical.push(triangle[(start + index) % 3])
  }
  return { ...output, indices:canonical }
}
function withOutputMetrics(result:Record<string,unknown>) {
  const output = canonicalMeshResult(result)
  return {
    ...output,
    outputMetrics: plain(metricsManifoldInKernel(
      Float64Array.from(output.positions as number[]),
      Uint32Array.from(output.indices as number[]),
    )),
  }
}

function runWasm():MatrixRun {
  const results:MatrixRecord[] = []
  for (const precision of ['f64', 'f32'] as const) {
    for (const testCase of corpus.cases) {
      const p = positions(testCase.positions, precision), i = Uint32Array.from(testCase.indices)
      results.push({ id:testCase.id, operation:'check', precision, result:plain(checkManifoldInKernel(p, i)) })
      results.push({ id:testCase.id, operation:'metrics', precision, result:plain(metricsManifoldInKernel(p, i)) })
      results.push({ id:testCase.id, operation:'repair', precision, result:withOutputMetrics(repairManifoldInKernel(p, i, 0, true) as unknown as Record<string,unknown>) })
    }
    for (const pair of corpus.booleanPairs) {
      const a = corpus.cases.find(testCase => testCase.id === pair.a)!
      const b = corpus.cases.find(testCase => testCase.id === pair.b)!
      for (const operation of pair.operations) {
        const output = booleanManifoldInKernel(
          positions(a.positions, precision), Uint32Array.from(a.indices),
          positions(b.positions, precision), Uint32Array.from(b.indices), operation,
        )
        results.push({ id:pair.id, operation:`boolean-${operation}`, precision, result:withOutputMetrics(output as unknown as Record<string,unknown>) })
      }
    }
  }
  return { schemaVersion:1, environment:{ kind:'wasm', arch:'wasm32', os:'unknown' }, results }
}

function runNative():MatrixRun {
  const cargo = process.env.CARGO || 'cargo'
  const child = spawnSync(cargo, [
    'run', '--quiet', '--locked', '--manifest-path', 'crates/Cargo.toml',
    '-p', 'manifold-ops', '--example', 'environment_oracle',
  ], { cwd:root, input:corpusBytes, encoding:'utf8', maxBuffer:64 * 1024 * 1024 })
  if (child.error) throw child.error
  if (child.status !== 0) throw new Error(`Native environment oracle failed (${child.status}):\n${child.stderr}`)
  return JSON.parse(child.stdout) as MatrixRun
}

interface Difference { path:string; native:unknown; current:unknown; delta?:number }
function compareValue(native:unknown, current:unknown, tolerance:number, path:string, differences:Difference[]) {
  let maxDelta = 0, numbers = 0
  const visit = (a:unknown, b:unknown, at:string) => {
    if (typeof a === 'number' && typeof b === 'number') {
      numbers++
      if (!Number.isFinite(a) || !Number.isFinite(b)) {
        if (!Object.is(a, b)) differences.push({ path:at, native:a, current:b })
        return
      }
      const delta = Math.abs(a - b)
      maxDelta = Math.max(maxDelta, delta)
      if (delta > tolerance * Math.max(1, Math.abs(a), Math.abs(b))) differences.push({ path:at, native:a, current:b, delta })
      return
    }
    if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) differences.push({ path:`${at}.length`, native:a.length, current:b.length })
      for (let index = 0; index < Math.min(a.length, b.length); index++) visit(a[index], b[index], `${at}[${index}]`)
      return
    }
    if (a && b && typeof a === 'object' && typeof b === 'object') {
      const ak = Object.keys(a).sort(), bk = Object.keys(b).sort()
      if (JSON.stringify(ak) !== JSON.stringify(bk)) differences.push({ path:`${at}.keys`, native:ak, current:bk })
      for (const key of ak) if (Object.hasOwn(b, key)) visit((a as Record<string,unknown>)[key], (b as Record<string,unknown>)[key], `${at}.${key}`)
      return
    }
    if (!Object.is(a, b)) differences.push({ path:at, native:a, current:b })
  }
  visit(native, current, path)
  return { maxDelta, numbers }
}

const key = (record:MatrixRecord) => `${record.id}/${record.operation}/${record.precision}`
function byKey(run:MatrixRun) {
  if (run.schemaVersion !== 1 || !Array.isArray(run.results)) throw new Error('Invalid matrix result')
  const map = new Map(run.results.map(record => [key(record), record]))
  if (map.size !== run.results.length) throw new Error('Duplicate matrix result')
  return map
}
const native = runNative(), wasm = runWasm()
const nativeByKey = byKey(native), wasmByKey = byKey(wasm)
if (nativeByKey.size !== wasmByKey.size || [...nativeByKey.keys()].some(key => !wasmByKey.has(key))) {
  throw new Error(`Native/WASM result sets differ: native=${nativeByKey.size}, wasm=${wasmByKey.size}`)
}
const comparisons:Record<string,unknown>[] = []
let failed = 0
for (const [recordKey, nativeRecord] of nativeByKey) {
  const wasmRecord = wasmByKey.get(recordKey)!
  const differences:Difference[] = []
  const measured = compareValue(nativeRecord.result, wasmRecord.result, 1e-9, recordKey, differences)
  failed += differences.length
  comparisons.push({ kind:'native-wasm', id:recordKey, tolerance:1e-9, ...measured, differences })
  if (nativeRecord.precision !== 'f64') continue
  for (const [label, run] of [['native', nativeByKey], ['wasm', wasmByKey]] as const) {
    const otherKey = recordKey.replace(/\/f64$/, '/f32')
    const other = run.get(otherKey)
    if (!other) throw new Error(`Missing ${label} precision counterpart: ${otherKey}`)
    const precisionDifferences:Difference[] = []
    const precisionMeasured = compareValue(nativeRecord.result, other.result, 2e-5, `${label}/${otherKey}`, precisionDifferences)
    const diagnosticDifferences = precisionDifferences.filter(difference => difference.path.endsWith('.kernel.work'))
    const differences = precisionDifferences.filter(difference => !difference.path.endsWith('.kernel.work'))
    failed += differences.length
    comparisons.push({ kind:`${label}-f64-f32`, id:otherKey, tolerance:2e-5, ...precisionMeasured, differences, diagnosticDifferences })
  }
}
const artifact = await readFile(path.join(root, 'public/wasm/geometry-kernel.wasm'))
const output = path.resolve(process.argv[2] || path.join(root, 'tools/manifold-bench/tmp/environment-matrix.json'))
const report = {
  schemaVersion:1,
  status:failed ? 'failed' : 'passed',
  native:native.environment,
  wasm:{ ...wasm.environment, artifactSha256:createHash('sha256').update(artifact).digest('hex') },
  corpusSha256:createHash('sha256').update(corpusBytes).digest('hex'),
  records:native.results.length,
  comparisons,
  failedDifferences:failed,
}
await mkdir(path.dirname(output), { recursive:true })
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`)
console.log(`manifold environment matrix: ${native.results.length} native + ${wasm.results.length} WASM records, ${comparisons.length} comparisons, ${failed} differences; report=${output}`)
if (failed) throw new Error(`Environment matrix has ${failed} differences outside tolerance`)
