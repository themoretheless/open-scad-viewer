#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdir, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import Module from 'manifold-3d/manifold.js'
import { buildFixture } from './fixtures.mjs'

assert.equal(typeof global.gc, 'function', 'Run node --expose-gc bench-memory.mjs')
const root = fileURLToPath(new URL('.', import.meta.url))
const out = process.argv[2] ?? path.join(root, 'tmp', `memory-${new Date().toISOString().replace(/[:.]/g, '-')}.json`)

const wasm = await Module()
wasm.setup()

const rows = []
let triangles = 0
for (let iteration = 0; iteration < 15; iteration++) {
  const solid = buildFixture(wasm, 'dense-sphere')
  const mesh = solid.getMesh()
  triangles = mesh.triVerts.length / 3
  solid.delete()
  global.gc()
  await new Promise(resolve => setImmediate(resolve))
  global.gc()
  rows.push({ iteration, memory: process.memoryUsage() })
}

const report = {
  schemaVersion: 1,
  kind: 'sidecar-manifold-memory',
  package: 'manifold-3d',
  version: '3.5.1',
  product: 'not-open-scad-viewer',
  fixture: 'dense-sphere',
  triangles,
  environment: {
    node: process.version,
    platform: process.platform,
    architecture: process.arch,
    osRelease: os.release(),
  },
  rows,
}
await mkdir(path.dirname(path.resolve(out)), { recursive: true })
await writeFile(out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
console.log(JSON.stringify({
  report: path.resolve(out),
  first: rows[0],
  last: rows.at(-1),
  triangles,
}, null, 2))
