import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

interface ParityManifest {
  version: number
  structuralBase: string
  donor: string
  policy: string
  items: Array<{
    id: string
    status: 'ported' | 'partial' | 'deferred' | 'rejected'
    decision: string
    evidence: string[]
  }>
}

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const manifest = JSON.parse(readFileSync(resolve(repositoryRoot, 'docs/feature-parity.json'), 'utf8')) as ParityManifest

describe('donor feature parity manifest', () => {
  it('keeps every decision explicit, unique, and evidence-backed', () => {
    expect(manifest.version).toBe(1)
    expect(manifest.policy).toContain('never merge')
    expect(manifest.donor).toMatch(/^origin\//)
    expect(manifest.items.length).toBeGreaterThanOrEqual(10)
    expect(new Set(manifest.items.map(item => item.id)).size).toBe(manifest.items.length)
    for (const item of manifest.items) {
      expect(['ported', 'partial', 'deferred', 'rejected']).toContain(item.status)
      expect(item.decision.length).toBeGreaterThan(10)
      expect(item.evidence.length).toBeGreaterThan(0)
      for (const path of item.evidence) expect(existsSync(resolve(repositoryRoot, path)), path).toBe(true)
    }
  })
})
