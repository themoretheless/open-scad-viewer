import { createHash } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import {
  FROZEN_ARCHIVES, REFRESH_EXECUTORS, REFRESH_OUTPUTS,
  prepareQualificationRefresh, publishQualificationRefresh, refreshBinaryInputPaths,
  refreshInputPaths,
} from '../scripts/refresh-qualification-fingerprints.mjs'
import {
  G1_AMENDMENTS, g1RefreshInputPaths, prepareG1PlanRefresh,
} from '../scripts/refresh-g1-plan.mjs'

const repositoryRoot = resolve(import.meta.dirname, '..')
const fingerprintPath = resolve(
  repositoryRoot,
  'docs/qualification/g0-toolchain-fingerprints-v22.json',
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
  it('uses the frozen G0.2 schema id', () => {
    const doc = JSON.parse(readFileSync(fingerprintPath, 'utf8')) as FingerprintDoc
    expect(doc.schema).toBe('open-scad-viewer/g0-toolchain-fingerprints')
    expect(doc.fingerprintId).toBe('g0-toolchain-fingerprints-v22')
    expect(doc).toMatchObject({
      previousFingerprintId: 'g0-toolchain-fingerprints-v21',
      previousFingerprintSha256: '88206f45c60ca20698e20fd3c4a7a1dbca43d70793d7db7eace2c554c0bb97fe',
    })
  })

  it('keeps fingerprints v1 through v12 byte-immutable as historical evidence', () => {
    const v1 = JSON.parse(readFileSync(fingerprintV1Path, 'utf8')) as FingerprintDoc
    expect(v1.fingerprintId).toBe('g0-toolchain-fingerprints-v1')
    expect(v1.artifacts.length).toBeGreaterThan(0)
    expect(sha256(readFileSync(fingerprintV1Path)))
      .toBe('4c2a286ea9567ddef66d2a0335a4960bddc94921b662c9e34af063b480cb8af3')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v2.json'))))
      .toBe('d040f252ce41177e24b02491beef59ccb57771e99fc0efaf9f0e7e4b2a081e76')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v3.json'))))
      .toBe('c8c74e76f7e6ce9e85c8f1c4267e678ca8c243b16ec4e6dd050584b489c0c171')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v4.json'))))
      .toBe('38b9fac7b3e03a301b0dfdeadf514572f337feff37b186089d968f3afa9e5a46')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v5.json'))))
      .toBe('9da6b4921492e2f6592194b58f144d6c7460f3c6787717f1b311cf7040428040')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v6.json'))))
      .toBe('ef2f25017daec1d75fae2725a85a500fa59d36c448f8a5ba0b9407f90a291eaf')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v7.json'))))
      .toBe('6a364833e07040d59cd47564bfb791ff97f9bca812239b5f72ce19423a23dcbf')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v8.json'))))
      .toBe('8446a3193659cf9325579e9f1286d8367a517eb9af780950357d35121e65ec62')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v9.json'))))
      .toBe('e7476156b67f5e0b00ee98ff5f60740d64e17796067a2d018f9c7f63c856b60c')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v10.json'))))
      .toBe('cc27233347f9b177691ed08ae77e2eb7a38777133f6451e61b6f3ab160ca7ac3')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v11.json'))))
      .toBe('5ae685b005bbbeaf3e7c84cf06011f4136e997cf7f6dfab38e439c72ca34ec19')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v12.json'))))
      .toBe('1af08861746409347f5f2b931c7d794dffc9df3c0321c27cb534b93a3eac3be7')
  })

  it('matches exact bytes for every listed artifact', () => {
    const doc = JSON.parse(readFileSync(fingerprintPath, 'utf8')) as FingerprintDoc
    expect(doc.artifacts.length).toBeGreaterThan(0)
    for (const artifact of doc.artifacts) {
      const bytes = readFileSync(resolve(repositoryRoot, artifact.path))
      expect(bytes.byteLength, artifact.path).toBe(artifact.byteLength)
      expect(sha256(bytes), artifact.path).toBe(artifact.sha256)
    }
  })
})

describe('G0/G1 re-freeze generator', () => {
  const temporaryRoots: string[] = []
  afterEach(() => {
    for (const root of temporaryRoots.splice(0)) rmSync(root, { recursive: true, force: true })
  })
  function fixture(): string {
    const root = mkdtempSync(join(tmpdir(), 'qualification-refreeze-'))
    temporaryRoots.push(root)
    for (const path of [
      ...refreshInputPaths(repositoryRoot),
      ...refreshBinaryInputPaths(repositoryRoot),
    ]) {
      mkdirSync(dirname(resolve(root, path)), { recursive: true })
      copyFileSync(resolve(repositoryRoot, path), resolve(root, path))
    }
    return root
  }
  function prepare(root: string) {
    return prepareQualificationRefresh({
      repositoryRoot: root, recordedAt: '2026-09-16',
      reason: 'Test preparation of the current SVG/BRep binding re-freeze with all clean work reset.',
      observedRustcVersionLine: 'rustc 1.100.0-nightly (a36d05efa 2026-09-09)',
    })
  }

  it('prepares deterministic new versions without creating any artifact or result', () => {
    const root = fixture()
    const first = prepare(root)
    const second = prepare(root)
    expect(first.artifactBytes).toEqual(second.artifactBytes)
    expect(first.plan.planId).toBe('semantic-manifold-g1-plan-v39')
    expect(first.status).toMatchObject({
      qualificationClaim: 'none', qualificationApproval: 'not-approved', g0Closed: false,
      completedWorkUnits: 0, completedCleanRuns: 0, plannedWorkUnits: 4740,
      priorEvidenceTreatment: 'discovery-only', priorResultsMayBeImported: false,
    })
    expect(first.status.pendingRows.every(row => row.completedWorkUnits === 0 && row.completedCleanRuns === 0)).toBe(true)
    for (const path of Object.values(REFRESH_OUTPUTS)) expect(existsSync(resolve(root, path))).toBe(false)
    expect(existsSync(resolve(root, 'output/qualification'))).toBe(false)
  })

  it('publishes exclusively, preserves every archive, and never carries forward results', () => {
    const root = fixture()
    const prepared = prepare(root)
    publishQualificationRefresh(prepared)
    for (const [id, path] of Object.entries(REFRESH_OUTPUTS)) {
      expect(readFileSync(resolve(root, path))).toEqual(prepared.artifactBytes[id])
    }
    for (const [path, hash] of Object.entries(FROZEN_ARCHIVES)) {
      expect(sha256(readFileSync(resolve(root, path))), path).toBe(hash)
    }
    expect(existsSync(resolve(root, 'output/qualification'))).toBe(false)
    expect(() => publishQualificationRefresh(prepared)).toThrow(/Refusing existing artifact/u)
  })

  it('rejects a source race before publishing and preserves an existing destination', () => {
    const root = fixture()
    const prepared = prepare(root)
    const candidatePath = resolve(root, 'src/services/openscadSemanticLowerer.ts')
    writeFileSync(candidatePath, `${readFileSync(candidatePath, 'utf8')}\n`)
    expect(() => publishQualificationRefresh(prepared)).toThrow(/changed during re-freeze/u)
    for (const path of Object.values(REFRESH_OUTPUTS)) expect(existsSync(resolve(root, path))).toBe(false)
    const refreshed = prepare(root)
    const existing = resolve(root, REFRESH_OUTPUTS.plan)
    writeFileSync(existing, 'existing destination must survive\n')
    expect(() => publishQualificationRefresh(refreshed)).toThrow(/Refusing existing artifact/u)
    expect(readFileSync(existing, 'utf8')).toBe('existing destination must survive\n')
    expect(existsSync(resolve(root, REFRESH_OUTPUTS.fingerprint))).toBe(false)
  })

  it('cannot refresh a historical archive or silently replace the frozen oracle', () => {
    for (const path of [
      'docs/qualification/g0-toolchain-fingerprints-v1.json',
      'docs/qualification/semantic-manifold-g1-plan-v18.json',
      'tests/fixtures/manifold-plan-oracle-v1.json',
    ]) {
      const root = fixture()
      const target = resolve(root, path)
      writeFileSync(target, `${readFileSync(target, 'utf8')}\n`)
      expect(() => prepare(root)).toThrow(/Historical archive changed|cannot be amended/u)
      for (const output of Object.values(REFRESH_OUTPUTS)) expect(existsSync(resolve(root, output))).toBe(false)
    }
  })

  it('refuses a zero-counter freeze when the new candidate already has result bookkeeping', () => {
    const root = fixture()
    const prepared = prepare(root)
    const resultPath = resolve(root, 'output/qualification/semantic-manifold-g1-candidate-run-v39/result.json')
    mkdirSync(dirname(resultPath), { recursive: true })
    writeFileSync(resultPath, '{"completedWorkUnits":1}\n')
    expect(() => prepare(root)).toThrow(/already has execution bookkeeping/u)
    expect(() => publishQualificationRefresh(prepared)).toThrow(/already has execution bookkeeping/u)
    for (const path of Object.values(REFRESH_OUTPUTS)) expect(existsSync(resolve(root, path))).toBe(false)
    expect(readFileSync(resultPath, 'utf8')).toBe('{"completedWorkUnits":1}\n')
  })

  it('historical v24 executors retain run-index admission without executing or creating evidence', () => {
    const root = fixture()
    const historicalStatus = JSON.parse(readFileSync(resolve(repositoryRoot,
      'docs/qualification/g0-v7-g1-v24-refreeze-status-v1.json'), 'utf8')) as {
        inputSnapshot: { files: { path: string; sha256: string }[] }
      }
    for (const path of REFRESH_EXECUTORS.slice(-2)) {
      // Version-only retargeting is reversible; prove this fixture is the exact
      // recorded executor, rather than merely assuming the old behavior survived.
      const historical = readFileSync(resolve(root, path), 'utf8')
        .replaceAll('semantic-manifold-g1-plan-v39', 'semantic-manifold-g1-plan-v24')
        .replaceAll('semantic-manifold-g1-candidate-run-v39', 'semantic-manifold-g1-candidate-run-v24')
        .replace('G1 v39 clean-run', 'G1 v24 clean-run')
      expect(sha256(Buffer.from(historical)))
        .toBe(historicalStatus.inputSnapshot.files.find(item => item.path === path)?.sha256)
      writeFileSync(resolve(root, path), historical)
    }
    publishQualificationRefresh(prepare(root))
    // Removing v23 proves the archived executors select v24 directly.
    rmSync(resolve(root, 'docs/qualification/semantic-manifold-g1-plan-v23.json'))
    for (const path of REFRESH_EXECUTORS.slice(-2)) {
      const result = spawnSync(process.execPath, [resolve(root, path),
        '--row', 'oracle-differential', '--env', 'ubuntu-node20', '--run-index', '999',
      ], { encoding: 'utf8', timeout: 5000 })
      expect(result.status, result.stderr).toBe(2)
      expect(result.stderr).toMatch(/exceeds cleanRunsRequired|Invalid run-index/u)
      expect(existsSync(resolve(root, 'output/qualification'))).toBe(false)
    }
  })
})

describe('historical G1 v20-only re-freeze generator', () => {
  const temporaryRoots: string[] = []
  afterEach(() => {
    for (const root of temporaryRoots.splice(0)) rmSync(root, { recursive: true, force: true })
  })
  function fixture(): string {
    const root = mkdtempSync(join(tmpdir(), 'g1-v20-refreeze-'))
    temporaryRoots.push(root)
    for (const path of g1RefreshInputPaths(20, repositoryRoot)) {
      mkdirSync(dirname(resolve(root, path)), { recursive: true })
      copyFileSync(resolve(repositoryRoot, path), resolve(root, path))
    }
    return root
  }
  it('refuses current G0 drift instead of rewriting the frozen v20 candidate', () => {
    const root = fixture()
    expect(() => prepareG1PlanRefresh({
      version: 20, repositoryRoot: root, recordedAt: '2026-09-16',
      reason: G1_AMENDMENTS[20].reason,
    })).toThrow(/G0 binding drift/u)
    expect(() => prepareG1PlanRefresh({version:21,repositoryRoot:root,recordedAt:'2026-09-13',
      reason:'Unreviewed future version must be rejected'})).toThrow(/explicit reviewed amendment/u)
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
