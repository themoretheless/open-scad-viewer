import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const repositoryRoot = resolve(import.meta.dirname, '..')
const fingerprintPath = resolve(
  repositoryRoot,
  'docs/qualification/g0-toolchain-fingerprints-v2.json',
)
const fingerprintV1Path = resolve(
  repositoryRoot,
  'docs/qualification/g0-toolchain-fingerprints-v1.json',
)
const statusPath = resolve(
  repositoryRoot,
  'docs/qualification/g0-contract-pack-status-v1.json',
)
const inventoryPath = resolve(
  repositoryRoot,
  'docs/qualification/g2a-predicate-inventory-v1.json',
)

type FingerprintArtifact = {
  readonly id: string
  readonly path: string
  readonly sha256: string
  readonly byteLength: number
}

type FingerprintDoc = {
  readonly schema: string
  readonly fingerprintId: string
  readonly artifacts: readonly FingerprintArtifact[]
}

type StatusDoc = {
  readonly schema: string
  readonly packId: string
  readonly gate: { readonly closed: boolean }
  readonly items: readonly { readonly id: string; readonly status: string }[]
}

function sha256(bytes: Buffer): string {
  return createHash('sha256').update(bytes).digest('hex')
}

describe('G0 toolchain fingerprints', () => {
  const doc = JSON.parse(readFileSync(fingerprintPath, 'utf8')) as FingerprintDoc

  it('uses the frozen G0.2 schema id', () => {
    expect(doc.schema).toBe('open-scad-viewer/g0-toolchain-fingerprints')
    expect(doc.fingerprintId).toBe('g0-toolchain-fingerprints-v2')
  })

  it('keeps fingerprints v1 as immutable historical evidence', () => {
    const v1 = JSON.parse(readFileSync(fingerprintV1Path, 'utf8')) as FingerprintDoc
    expect(v1.fingerprintId).toBe('g0-toolchain-fingerprints-v1')
    expect(v1.artifacts.length).toBeGreaterThan(0)
  })

  it('matches exact bytes for every listed artifact', () => {
    expect(doc.artifacts.length).toBeGreaterThan(0)
    for (const artifact of doc.artifacts) {
      const bytes = readFileSync(resolve(repositoryRoot, artifact.path))
      expect(bytes.byteLength, artifact.path).toBe(artifact.byteLength)
      expect(sha256(bytes), artifact.path).toBe(artifact.sha256)
    }
  })
})

describe('G0 contract pack status', () => {
  const doc = JSON.parse(readFileSync(statusPath, 'utf8')) as StatusDoc

  it('remains explicitly open until Definition of Done', () => {
    expect(doc.schema).toBe('open-scad-viewer/g0-contract-pack-status')
    expect(doc.packId).toBe('g0-contract-pack-status-v1')
    expect(doc.gate.closed).toBe(false)
  })

  it('tracks every G0.1–G0.14 backlog row', () => {
    const ids = doc.items.map((item) => item.id)
    expect(ids).toEqual([
      'G0.1',
      'G0.2',
      'G0.3',
      'G0.4',
      'G0.5',
      'G0.6',
      'G0.7',
      'G0.8',
      'G0.9',
      'G0.10',
      'G0.11',
      'G0.12',
      'G0.13',
      'G0.14',
    ])
  })
})

describe('G2a predicate inventory', () => {
  it('freezes the G0.7 finite predicate set', () => {
    const inventory = JSON.parse(readFileSync(inventoryPath, 'utf8')) as {
      readonly inventoryId: string
      readonly predicates: readonly { readonly id: string }[]
      readonly shewchukPolicy: { readonly selectedMode: string }
    }
    expect(inventory.inventoryId).toBe('g2a-predicate-inventory-v1')
    expect(inventory.predicates.map((p) => p.id)).toEqual([
      'orient2d',
      'orient3d',
      'compare_squared_distance',
    ])
    expect(inventory.shewchukPolicy.selectedMode).toBe('original_from_paper')
  })
})
