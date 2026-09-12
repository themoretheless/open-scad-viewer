import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')

const required = [
  'docs/adr/0005-semantic-program-v1.md',
  'docs/adr/0006-tolerance-predicates-evidence-v1.md',
  'docs/adr/0007-brep-topology-idl-v1.md',
  'docs/adr/0008-protocol-v6-geometry-scene-v2.md',
  'docs/adr/0009-resource-security-supervisor-v1.md',
  'docs/adr/0010-independent-reimplementation-policy-v1.md',
  'docs/qualification/g0-contract-pack.md',
  'docs/qualification/g0-contract-pack-status-v1.json',
  'docs/qualification/g2a-predicate-inventory-v1.json',
  'docs/qualification/resource-security-supervisor-v1.json',
  'docs/qualification/allowlisted-sources-v1.json',
  'docs/qualification/rights-inventory-v1.json',
  'docs/qualification/provenance/PROVENANCE.example.yml',
  'docs/qualification/provenance/legacy-golden-PROVENANCE.yml',
  'docs/qualification/geometry-scene-v2/geometry-scene-v2.schema.json',
  'docs/qualification/geometry-scene-v2/wire-fixtures-v1.json',
  'docs/qualification/geometry-scene-v2/v5-migration-matrix-v1.json',
  'docs/qualification/topology-idl/sibling-cow-stale-key-v1.json',
  'docs/qualification/g0-differential-comparator-v1.json',
  'docs/qualification/g0-legacy-golden-corpus-v1.json',
  'docs/qualification/g0-refusal-precedence-matrix-v1.json',
  'docs/qualification/g0-toolchain-fingerprints-v2.json',
  'docs/qualification/g0-spdx-sbom-fingerprints-v1.json',
  'docs/qualification/semantic-manifold-g1-plan-v17.json',
  'docs/qualification/g1-plan-file-digests-v1.json',
  'docs/qualification/sbom/g0-project-spdx-v1.json',
  'docs/qualification/adr-0005-acceptance-evidence-v1.json',
  'tests/support/referencePredicateOracleV1.ts',
  'tests/support/referenceGeometrySceneV2.ts',
] as const

describe('G0 contract pack artifact presence', () => {
  it('keeps the engineering artifact set readable', () => {
    for (const path of required) {
      const bytes = readFileSync(resolve(root, path))
      expect(bytes.byteLength, path).toBeGreaterThan(32)
    }
  })

  it('does not claim G0 closed without human acceptance', () => {
    const status = JSON.parse(
      readFileSync(resolve(root, 'docs/qualification/g0-contract-pack-status-v1.json'), 'utf8'),
    ) as { readonly gate: { readonly closed: boolean }; readonly claim: string | undefined }
    expect(status.gate.closed).toBe(false)
  })
})
