#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdir, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import Module from 'manifold-3d/manifold.js'
import { cpuFixtures, buildFixture } from './fixtures.mjs'

function optionsFromArgs(args) {
  const options = {
    iterations: 9,
    warmups: 2,
    fixtures: cpuFixtures.map(item => item.id),
    out: null,
  }
  for (let index = 0; index < args.length; index++) {
    const arg = args[index]
    if (arg === '--help' || arg === '-h') return { help: true }
    if (arg === '--quick') {
      options.iterations = 3
      options.warmups = 1
      continue
    }
    const [key, inline] = arg.split('=', 2)
    const value = inline ?? args[++index]
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${key}`)
    if (key === '--iterations') options.iterations = Number(value)
    else if (key === '--warmups') options.warmups = Number(value)
    else if (key === '--fixtures') options.fixtures = value.split(',')
    else if (key === '--out') options.out = path.resolve(value)
    else throw new Error(`Unknown argument ${key}`)
  }
  assert.ok(Number.isSafeInteger(options.iterations) && options.iterations >= 1 && options.iterations <= 100)
  assert.ok(Number.isSafeInteger(options.warmups) && options.warmups >= 0 && options.warmups <= 20)
  for (const id of options.fixtures) {
    assert.ok(cpuFixtures.some(item => item.id === id), `Unknown fixture ${id}`)
  }
  return options
}

function stats(values) {
  const sorted = [...values].sort((a, b) => a - b)
  const at = percentile => sorted[Math.max(0, Math.ceil(percentile * sorted.length) - 1)]
  return { n: sorted.length, min: sorted[0], p50: at(0.5), p95: at(0.95), max: sorted.at(-1) }
}

const options = optionsFromArgs(process.argv.slice(2))
if (options.help) {
  console.log('Usage: node bench-cpu.mjs [--quick] [--iterations N] [--warmups N] [--fixtures id,id] [--out file.json]')
  process.exit(0)
}

const wasm = await Module()
wasm.setup()

const results = []
for (const id of options.fixtures) {
  const samples = []
  let triangles = 0
  for (let i = 0; i < options.warmups + options.iterations; i++) {
    const started = performance.now()
    const solid = buildFixture(wasm, id)
    const mesh = solid.getMesh()
    triangles = mesh.triVerts.length / 3
    solid.delete()
    const elapsed = performance.now() - started
    if (i >= options.warmups) samples.push(elapsed)
  }
  results.push({ id, triangles, ms: stats(samples) })
}

const report = {
  schemaVersion: 1,
  kind: 'sidecar-manifold-cpu',
  package: 'manifold-3d',
  version: '3.5.1',
  product: 'not-open-scad-viewer',
  startedAt: new Date().toISOString(),
  environment: {
    node: process.version,
    platform: process.platform,
    architecture: process.arch,
    osRelease: os.release(),
    cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))],
  },
  options,
  results,
}
if (options.out) {
  await mkdir(path.dirname(options.out), { recursive: true })
  await writeFile(options.out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
  console.log(options.out)
}
console.log(JSON.stringify(results, null, 2))
