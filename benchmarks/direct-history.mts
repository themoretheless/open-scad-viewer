import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import { DirectHistory, emptyDirectDocument } from '../src/services/directModeling'
import { createBrepGear, tessellateNurbsBrep } from '../src/services/geometry/brep'

const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const results = []
for (const teeth of [12, 32]) {
  const brep = createBrepGear({ module: 1, teeth, height: 8, bore: 3, helixAngle: 20, herringbone: true })
  const mesh = tessellateNurbsBrep(brep, 2)
  const document = emptyDirectDocument()
  document.bodies.push({ id: 'gear', name: 'Before', brep, mesh: { positions: mesh.positions, indices: mesh.indices } })
  const history = new DirectHistory(document)
  const before = JSON.stringify(history.document)
  const next = history.document
  next.bodies[0]!.name = 'After'
  history.commit(next)
  const after = JSON.stringify(history.document)
  const samples: Array<{ undoMs: number; redoMs: number }> = []
  for (let sample = -3; sample < 9; sample++) {
    const undoStart = performance.now()
    const undone = history.undo()
    const undoMs = performance.now() - undoStart
    assert.equal(JSON.stringify(undone), before)
    const redoStart = performance.now()
    const redone = history.redo()
    const redoMs = performance.now() - redoStart
    assert.equal(JSON.stringify(redone), after)
    if (sample >= 0) samples.push({ undoMs, redoMs })
  }
  results.push({ teeth, faces: brep.faces.length, documentCharacters: before.length,
    beforeSha256: hash(before), afterSha256: hash(after), samples,
    undoP50Ms: samples.map(row => row.undoMs).sort((a, b) => a - b)[4],
    redoP50Ms: samples.map(row => row.redoMs).sort((a, b) => a - b)[4],
  })
}
const report = { node: process.version, platform: process.platform, arch: process.arch, cpu: os.cpus()[0]?.model,
  warmups: 3, samples: 9, scope: 'DirectHistory undo/redo including returned defensive clone; construction, commit and correctness checks outside timing.',
  files: ['src/services/directModeling.ts', 'src/generated/geometry-kernels/kernel_bg.wasm'].map(path => ({ path, sha256: hash(readFileSync(path)) })), results,
}
const out = resolve(process.argv[2] ?? 'tmp/performance/direct-history.json')
mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, JSON.stringify(report, null, 2))
console.log(JSON.stringify(report))
