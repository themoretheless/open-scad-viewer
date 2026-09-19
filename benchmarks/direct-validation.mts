import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import { parseDirectDocument, type DirectDocument } from '../src/services/directModeling'
import { runExactSolidRequest } from '../src/services/solid/exactSolidRuntime'

const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const source = readFileSync('examples/modelgraph-text/planetary-spinner.mg', 'utf8')
const built = await runExactSolidRequest({ kind: 'exact-solid', version: 1, source })
assert(built.ok, built.ok ? '' : built.error.message)
const input = JSON.parse(built.document) as DirectDocument
input.bodies.forEach((body, index) => { body.id = `body-${index}` })
const text = JSON.stringify(input)
const keys = new Set(input.bodies.map(body => JSON.stringify(body.brep)))
const start = performance.now()
const expected = JSON.stringify(parseDirectDocument(text))
const firstValidationMs = performance.now() - start
const samplesMs: number[] = []
for (let sample = -3; sample < 9; sample++) {
  const start = performance.now()
  const result = parseDirectDocument(text)
  const duration = performance.now() - start
  assert.equal(JSON.stringify(result), expected)
  if (sample >= 0) samplesMs.push(duration)
}
const report = {
  node: process.version, platform: process.platform, arch: process.arch, cpu: os.cpus()[0]?.model,
  sourceSha256: hash(source), documentSha256: hash(expected),
  bodies: input.bodies.length, uniqueBreps: keys.size, documentCharacters: text.length,
  uniqueBrepCharacters: [...keys].reduce((sum, key) => sum + key.length, 0),
  firstValidationMs, warmups: 3, samplesMs, p50Ms: [...samplesMs].sort((a, b) => a - b)[4],
  scope: 'In-process synchronous document validation, including JSON parsing, normalization and owned result creation; fixture build and equality checks excluded from samples. First-validation timing additionally includes reference serialization. No browser paint or heap measurement.',
  files: ['src/services/directModeling.ts', 'src/services/brepInspectionCache.ts', 'src/generated/geometry-kernels/kernel_bg.wasm'].map(path => ({ path, sha256: existsSync(path) ? hash(readFileSync(path)) : null })),
}
const out = resolve(process.argv[2] ?? 'tmp/performance/direct-validation.json')
mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, JSON.stringify(report, null, 2))
console.log(JSON.stringify(report))
