import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { auditBundle } from '../scripts/audit-bundle.mjs'

test('counts emitted bytes separately from repeated original sources', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'bundle-audit-'))
  try {
    await mkdir(path.join(directory, 'assets'))
    await writeFile(path.join(directory, 'index.html'), 'test')
    for (const name of ['main', 'worker']) {
      await writeFile(path.join(directory, 'assets', `${name}.js`), 'abc')
      await writeFile(path.join(directory, 'assets', `${name}.js.map`), JSON.stringify({
        sources: ['../../src/shared.ts', '../../src/shared.ts', '../../src/transformed.ts'],
        sourcesContent: ['hello', 'hello', name],
      }))
    }
    const report = await auditBundle(directory)
    assert.equal(report.totalAssetBytes, 10)
    assert.equal(report.sourceMaps, 2)
    assert.equal(report.repeatedSources.length, 1)
    assert.equal(report.repeatedSources[0].sourceBytes, 5)
    assert.equal(report.repeatedSources[0].repeatedSourceBytes, 5)
    assert.deepEqual(report.repeatedSources[0].chunks, ['assets/main.js', 'assets/worker.js'])
    assert.ok(report.assets.every(asset => /^[a-f0-9]{64}$/.test(asset.sha256)))
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test('refuses a build without attribution rather than reporting no duplicates', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'bundle-audit-'))
  try {
    await assert.rejects(auditBundle(directory), /No JavaScript source maps/)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
