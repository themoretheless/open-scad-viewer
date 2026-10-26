#!/usr/bin/env node
import { readFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import Module from 'manifold-3d/manifold.js'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const corpus = JSON.parse(await readFile(new URL('./differential-corpus.json', import.meta.url), 'utf8'))
if (corpus.schemaVersion !== 1 || !Array.isArray(corpus.cases) || corpus.cases.length === 0) {
  throw new Error('Invalid differential corpus')
}

const cargo = process.env.CARGO || 'cargo'
const rust = spawnSync(cargo, [
  'run', '--quiet', '--locked', '--manifest-path', 'crates/Cargo.toml',
  '-p', 'manifold-ops', '--example', 'differential_oracle',
], {
  cwd: root,
  input: JSON.stringify(corpus),
  encoding: 'utf8',
  maxBuffer: 16 * 1024 * 1024,
})
if (rust.error) throw rust.error
if (rust.status !== 0) {
  throw new Error(`Rust differential adapter failed (${rust.status}):\n${rust.stderr}`)
}
const actual = JSON.parse(rust.stdout)
if (actual.schemaVersion !== 1 || actual.results.length !== corpus.cases.length * 3) {
  throw new Error('Rust differential adapter returned an incomplete result set')
}
const actualByKey = new Map(actual.results.map(result => [`${result.id}:${result.op}`, result]))

const wasm = await Module()
wasm.setup()
const operations = [
  ['union', (a, b) => a.add(b)],
  ['intersection', (a, b) => a.intersect(b)],
  ['difference', (a, b) => a.subtract(b)],
]
const mismatches = []
let comparisons = 0

function compareNumber(id, op, metric, own, reference) {
  const tolerance = 1e-7 * Math.max(1, Math.abs(own), Math.abs(reference))
  comparisons++
  if (!Number.isFinite(own) || Math.abs(own - reference) > tolerance) {
    mismatches.push(`${id} ${op} ${metric}: own=${own}, manifold-3d=${reference}, tolerance=${tolerance}`)
  }
}

for (const testCase of corpus.cases) {
  const a = wasm.Manifold.cube(testCase.a.size, false).translate(testCase.a.origin)
  const b = wasm.Manifold.cube(testCase.b.size, false).translate(testCase.b.origin)
  for (const [op, evaluate] of operations) {
    const own = actualByKey.get(`${testCase.id}:${op}`)
    if (!own) {
      mismatches.push(`${testCase.id} ${op}: missing Rust result`)
      continue
    }
    let reference
    try {
      const solid = evaluate(a, b)
      reference = { volume: solid.volume(), surface_area: solid.surfaceArea(), genus: solid.genus() }
      solid.delete()
    } catch (error) {
      reference = { error: String(error) }
    }
    if (own.error || reference.error) {
      if (own.error !== reference.error) {
        mismatches.push(`${testCase.id} ${op}: own error=${own.error || 'none'}, manifold-3d error=${reference.error || 'none'}`)
      }
      continue
    }
    compareNumber(testCase.id, op, 'volume', own.volume, reference.volume)
    compareNumber(testCase.id, op, 'surface area', own.surface_area, reference.surface_area)
    comparisons++
    if (own.genus !== reference.genus) {
      mismatches.push(`${testCase.id} ${op} genus: own=${own.genus}, manifold-3d=${reference.genus}`)
    }
  }
  a.delete()
  b.delete()
}

if (mismatches.length > 0) {
  throw new Error(`Differential failures are bugs; no snapshot was updated:\n${mismatches.join('\n')}`)
}
console.log(`manifold-3d differential: ${corpus.cases.length} cases, ${actual.results.length} booleans, ${comparisons} metric comparisons passed`)
