import { createHash } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import {
  FROZEN_ARCHIVES, REFRESH_EXECUTORS, REFRESH_OUTPUTS,
  prepareQualificationRefresh, publishQualificationRefresh, refreshInputPaths,
} from '../scripts/refresh-qualification-fingerprints.mjs'
import {
  G1_AMENDMENTS, G1_EXECUTORS, g1RefreshInputPaths,
  prepareG1PlanRefresh, publishG1PlanRefresh,
} from '../scripts/refresh-g1-plan.mjs'

const repositoryRoot = resolve(import.meta.dirname, '..')
const fingerprintPath = resolve(
  repositoryRoot,
  'docs/qualification/g0-toolchain-fingerprints-v3.json',
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
    expect(doc.fingerprintId).toBe('g0-toolchain-fingerprints-v3')
    expect(doc).toMatchObject({
      previousFingerprintId: 'g0-toolchain-fingerprints-v2',
      previousFingerprintSha256: 'd040f252ce41177e24b02491beef59ccb57771e99fc0efaf9f0e7e4b2a081e76',
    })
  })

  it('keeps fingerprints v1 and v2 byte-immutable as historical evidence', () => {
    const v1 = JSON.parse(readFileSync(fingerprintV1Path, 'utf8')) as FingerprintDoc
    expect(v1.fingerprintId).toBe('g0-toolchain-fingerprints-v1')
    expect(v1.artifacts.length).toBeGreaterThan(0)
    expect(sha256(readFileSync(fingerprintV1Path)))
      .toBe('4c2a286ea9567ddef66d2a0335a4960bddc94921b662c9e34af063b480cb8af3')
    expect(sha256(readFileSync(resolve(repositoryRoot, 'docs/qualification/g0-toolchain-fingerprints-v2.json'))))
      .toBe('d040f252ce41177e24b02491beef59ccb57771e99fc0efaf9f0e7e4b2a081e76')
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
    for (const path of refreshInputPaths(repositoryRoot)) {
      mkdirSync(dirname(resolve(root, path)), { recursive: true })
      copyFileSync(resolve(repositoryRoot, path), resolve(root, path))
    }
    return root
  }
  function prepare(root: string) {
    return prepareQualificationRefresh({
      repositoryRoot: root, recordedAt: '2026-09-13',
      reason: 'Test preparation of the current SVG/BRep binding re-freeze with all clean work reset.',
      observedRustcVersionLine: 'rustc 1.100.0-nightly (a36d05efa 2026-09-09)',
    })
  }

  it('prepares deterministic new versions without creating any artifact or result', () => {
    const root = fixture()
    const first = prepare(root)
    const second = prepare(root)
    expect(first.artifactBytes).toEqual(second.artifactBytes)
    expect(first.plan.planId).toBe('semantic-manifold-g1-plan-v19')
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
    const resultPath = resolve(root, 'output/qualification/semantic-manifold-g1-candidate-run-v19/result.json')
    mkdirSync(dirname(resultPath), { recursive: true })
    writeFileSync(resultPath, '{"completedWorkUnits":1}\n')
    expect(() => prepare(root)).toThrow(/already has execution bookkeeping/u)
    expect(() => publishQualificationRefresh(prepared)).toThrow(/already has execution bookkeeping/u)
    for (const path of Object.values(REFRESH_OUTPUTS)) expect(existsSync(resolve(root, path))).toBe(false)
    expect(readFileSync(resultPath, 'utf8')).toBe('{"completedWorkUnits":1}\n')
  })

  it('historical v19 executors retain run-index admission without executing or creating evidence', () => {
    const root = fixture()
    const historicalStatus = JSON.parse(readFileSync(resolve(repositoryRoot,
      'docs/qualification/g0-v3-g1-v19-refreeze-status-v1.json'), 'utf8')) as {
        inputSnapshot: { files: { path: string; sha256: string }[] }
      }
    for (const path of REFRESH_EXECUTORS) {
      // Version-only retargeting is reversible; prove this fixture is the exact
      // recorded executor, rather than merely assuming the old behavior survived.
      const historical = readFileSync(resolve(root, path), 'utf8')
        .replaceAll('semantic-manifold-g1-plan-v20', 'semantic-manifold-g1-plan-v19')
        .replaceAll('semantic-manifold-g1-candidate-run-v20', 'semantic-manifold-g1-candidate-run-v19')
        .replace('G1 v20 clean-run', 'G1 v19 clean-run')
      expect(sha256(Buffer.from(historical)))
        .toBe(historicalStatus.inputSnapshot.files.find(item => item.path === path)?.sha256)
      writeFileSync(resolve(root, path), historical)
    }
    publishQualificationRefresh(prepare(root))
    // Removing this temporary archive proves neither executor falls back to v18.
    rmSync(resolve(root, 'docs/qualification/semantic-manifold-g1-plan-v18.json'))
    for (const path of REFRESH_EXECUTORS) {
      const result = spawnSync(process.execPath, [resolve(root, path),
        '--row', 'oracle-differential', '--env', 'ubuntu-node20', '--run-index', '999',
      ], { encoding: 'utf8', timeout: 5000 })
      expect(result.status, result.stderr).toBe(2)
      expect(result.stderr).toMatch(/exceeds cleanRunsRequired|Invalid run-index/u)
      expect(existsSync(resolve(root, 'output/qualification'))).toBe(false)
    }
  })
})

describe('G1 v20 append-only re-freeze generator', () => {
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
  function prepare(root: string) {
    return prepareG1PlanRefresh({ version: 20, repositoryRoot: root, recordedAt: '2026-09-13',
      reason: G1_AMENDMENTS[20].reason })
  }

  it('preserves G0 v3 and every archive while starting a deterministic empty v20 candidate', () => {
    const root = fixture()
    const prepared = prepare(root)
    expect(prepared.artifactBytes).toEqual(prepare(root).artifactBytes)
    expect(Object.keys(prepared.artifactBytes)).toEqual(['plan', 'status'])
    expect(prepared.plan).toMatchObject({planId:'semantic-manifold-g1-plan-v20',
      processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v19',
        previousPlanSha256:G1_AMENDMENTS[20].previousPlanSha256,qualificationClaim:'none'}})
    expect(prepared.status).toMatchObject({completedWorkUnits:0,completedCleanRuns:0,
      plannedWorkUnits:4740,qualificationClaim:'none',qualificationApproval:'not-approved'})
    expect(prepared.status.archives).toHaveLength(23)
    for (const path of Object.values(prepared.outputs)) expect(existsSync(resolve(root,path))).toBe(false)
    publishG1PlanRefresh(prepared)
    for (const archive of prepared.status.archives) {
      expect(sha256(readFileSync(resolve(root, archive.path)))).toBe(archive.sha256)
    }
    expect(existsSync(resolve(root, 'output/qualification'))).toBe(false)
    expect(() => publishG1PlanRefresh(prepared)).toThrow(/Refusing existing artifact/u)
  })

  it('refuses source races and cannot silently advance a G0 binding or historical snapshot', () => {
    const root = fixture()
    const prepared = prepare(root)
    const source = resolve(root,'src/services/semanticProgramExecutor.ts')
    writeFileSync(source, `${readFileSync(source,'utf8')}\n`)
    expect(() => publishG1PlanRefresh(prepared)).toThrow(/changed during re-freeze/u)
    for (const path of Object.values(prepared.outputs)) expect(existsSync(resolve(root,path))).toBe(false)
    for (const path of ['package.json',G1_AMENDMENTS[20].previousStatus,
      'tests/fixtures/manifold-plan-oracle-v1.json']) {
      const root = fixture()
      writeFileSync(resolve(root,path), `${readFileSync(resolve(root,path),'utf8')}\n`)
      expect(() => prepare(root)).toThrow(/G0 binding drift|Historical archive changed|cannot be amended/u)
    }
    expect(() => prepareG1PlanRefresh({version:21,repositoryRoot:root,recordedAt:'2026-09-13',
      reason:'Unreviewed future version must be rejected'})).toThrow(/explicit reviewed amendment/u)
  })

  it('rejects existing v20 counters even if they appear after preparation', () => {
    const root = fixture()
    const prepared = prepare(root)
    const resultPath = resolve(root,'output/qualification/semantic-manifold-g1-candidate-run-v20/result.json')
    mkdirSync(dirname(resultPath),{recursive:true})
    writeFileSync(resultPath,'{"completedWorkUnits":1}\n')
    expect(() => prepare(root)).toThrow(/already has execution bookkeeping/u)
    expect(() => publishG1PlanRefresh(prepared)).toThrow(/already has execution bookkeeping/u)
    expect(readFileSync(resultPath,'utf8')).toBe('{"completedWorkUnits":1}\n')
  })

  it('active executors read v20 and reject invalid work without importing v19 evidence', () => {
    const root = fixture()
    publishG1PlanRefresh(prepare(root))
    rmSync(resolve(root,'docs/qualification/semantic-manifold-g1-plan-v19.json'))
    for (const path of G1_EXECUTORS) {
      const result = spawnSync(process.execPath,[resolve(root,path),
        '--row','oracle-differential','--env','ubuntu-node20','--run-index','999'],
      {encoding:'utf8',timeout:5000})
      expect(result.status,result.stderr).toBe(2)
      expect(result.stderr).toMatch(/exceeds cleanRunsRequired|Invalid run-index/u)
      expect(existsSync(resolve(root,'output/qualification'))).toBe(false)
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
