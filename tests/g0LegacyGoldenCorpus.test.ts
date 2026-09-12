import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')

describe('G0.9 legacy golden corpus index', () => {
  const corpus = JSON.parse(
    readFileSync(resolve(root, 'docs/qualification/g0-legacy-golden-corpus-v1.json'), 'utf8'),
  ) as {
    readonly frozenOracle: {
      readonly path: string
      readonly fileSha256: string
      readonly casesCorpusSha256: string
    }
    readonly cancelTerminals: readonly unknown[]
    readonly mcpPathGoldens: readonly unknown[]
    readonly provenanceBundle: string
  }

  it('pins the immutable manifold-plan-oracle-v1 file and cases digests', () => {
    const bytes = readFileSync(resolve(root, corpus.frozenOracle.path))
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(corpus.frozenOracle.fileSha256)
    const manifest = JSON.parse(bytes.toString('utf8')) as { readonly corpusSha256: string }
    expect(manifest.corpusSha256).toBe(corpus.frozenOracle.casesCorpusSha256)
  })

  it('records cancel terminals, MCP goldens, and provenance', () => {
    expect(corpus.cancelTerminals.length).toBeGreaterThanOrEqual(3)
    expect(corpus.mcpPathGoldens.length).toBeGreaterThanOrEqual(3)
    expect(readFileSync(resolve(root, corpus.provenanceBundle), 'utf8')).toContain(
      'legacy-manifold-plan-golden-corpus',
    )
  })
})

describe('G0.10 differential comparator', () => {
  it('owns 18 mutations separate from the G1 plan document', () => {
    const comparator = JSON.parse(
      readFileSync(resolve(root, 'docs/qualification/g0-differential-comparator-v1.json'), 'utf8'),
    ) as {
      readonly comparatorId: string
      readonly mutations: readonly { readonly id: string }[]
      readonly bindings: { readonly liftedFrom: string }
    }
    expect(comparator.comparatorId).toBe('g0-differential-comparator-v1')
    expect(comparator.mutations).toHaveLength(18)
    expect(comparator.bindings.liftedFrom).toContain('semantic-manifold-g1-plan-v16.json')
  })
})

describe('G0.2 SPDX SBOM fingerprints', () => {
  it('matches exact bytes for every listed SPDX artifact', () => {
    const doc = JSON.parse(
      readFileSync(resolve(root, 'docs/qualification/g0-spdx-sbom-fingerprints-v1.json'), 'utf8'),
    ) as {
      readonly fingerprintId: string
      readonly parentFingerprintId: string
      readonly artifacts: readonly { readonly path: string; readonly sha256: string; readonly byteLength: number }[]
    }
    expect(doc.fingerprintId).toBe('g0-spdx-sbom-fingerprints-v1')
    expect(doc.parentFingerprintId).toBe('g0-toolchain-fingerprints-v1')
    for (const artifact of doc.artifacts) {
      const bytes = readFileSync(resolve(root, artifact.path))
      expect(bytes.byteLength, artifact.path).toBe(artifact.byteLength)
      expect(createHash('sha256').update(bytes).digest('hex'), artifact.path).toBe(artifact.sha256)
    }
  })
})
