import {describe, expect, it} from 'vitest'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {assessRuntimeBinding} from '../scripts/audit-runtime-manifests.mts'
import {
  GEOMETRY_MANIFEST_ARCHIVE, computeGeometryManifestDigest,
  type GeometryExecutionDescriptor,
} from '../src/core/geometryExecution'

const manifest = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1']
const execution: GeometryExecutionDescriptor = {
  engineClass: manifest.engineClass, engineKey: manifest.engineKey,
  kernelFingerprint: manifest.kernelFingerprint,
  semanticProgramVersion: manifest.semanticProgramVersion,
  capabilityManifestVersion: manifest.capabilityManifestVersion,
  manifestDigest: manifest.manifestDigest,
  languageContract: 'legacy/current', requiredCapabilities: [],
  quality: 'full', purpose: 'analysis', representation: 'mesh', evidence: 'runtime',
  effectiveLimits: {...manifest.limits}, automaticFallback: false,
}
const hash = manifest.kernelFingerprint.slice('sha256:'.length)

describe('runtime artifact binding audit', () => {
  it('accepts matching artifact, immutable manifest and runtime descriptor without granting qualification', () => {
    expect(assessRuntimeBinding(manifest, execution, hash)).toMatchObject({
      matches: true, artifactMatchesManifest: true, manifestIntegrityMatches: true,
      descriptorMatchesManifest: true, runtimeEvidence: true,
    })
    const pending = {...manifest, qualification: {...manifest.qualification,
      status: 'baseline-pending' as const, recordId: null, corpusVersion: null}}
    pending.manifestDigest = computeGeometryManifestDigest(pending)
    expect(assessRuntimeBinding(pending, {...execution, manifestDigest: pending.manifestDigest}, hash))
      .toMatchObject({matches: true, declaredQualification: 'baseline-pending'})
  })

  it('refuses a qualified descriptor for different executed bytes', () => {
    expect(assessRuntimeBinding(manifest, execution, '1'.repeat(64))).toMatchObject({
      matches: false, declaredQualification: 'qualified', artifactMatchesManifest: false,
      manifestIntegrityMatches: true, descriptorMatchesManifest: true,
    })
  })

  it('requires descriptor identity, manifest integrity and runtime rather than planned evidence', () => {
    for (const field of ['engineClass', 'engineKey', 'kernelFingerprint', 'semanticProgramVersion',
      'capabilityManifestVersion', 'manifestDigest'] as const) {
      const changed = {...execution, [field]: 'different'} as GeometryExecutionDescriptor
      expect(assessRuntimeBinding(manifest, changed, hash).matches, field).toBe(false)
    }
    expect(assessRuntimeBinding({...manifest, displayName: 'tampered'}, execution, hash))
      .toMatchObject({matches: false, manifestIntegrityMatches: false})
    expect(assessRuntimeBinding(manifest, {...execution, evidence: 'planned'}, hash))
      .toMatchObject({matches: false, runtimeEvidence: false})
    expect(() => assessRuntimeBinding(manifest, execution, 'unobserved')).toThrow()
  })

  it('runs both real providers in a fresh process and returns failure exactly when bindings disagree', () => {
    const child = spawnSync(process.execPath, ['--import', 'tsx', 'scripts/audit-runtime-manifests.mts'], {
      cwd: fileURLToPath(new URL('../', import.meta.url)), encoding: 'utf8', timeout: 15_000,
    })
    expect(child.error).toBeUndefined()
    expect(child.signal).toBeNull()
    const report = JSON.parse(child.stdout)
    const artifact = readFileSync(new URL('../public/wasm/geometry-kernel.wasm', import.meta.url))
    expect(report).toMatchObject({
      schema: 1, qualificationClaim: 'none', compilerLoads: 1, publicArtifactMatches: true,
      artifactBytes: artifact.length,
      artifactSha256: createHash('sha256').update(artifact).digest('hex'),
    })
    expect(report.records.map((record: {engineClass: string}) => record.engineClass)).toEqual(['mesh', 'brep'])
    for (const record of report.records) {
      expect(record).toMatchObject({runtimeEvidence: true, meshCount: 1, volumeMm3: 1,
        observedKernelFingerprint: `sha256:${report.artifactSha256}`})
      expect(record.artifactMatchesManifest).toBe(record.declaredKernelFingerprint === record.observedKernelFingerprint)
    }
    expect(report.matches).toBe(report.records.every((record: {matches: boolean}) => record.matches))
    expect(child.status, child.stderr).toBe(report.matches ? 0 : 1)
  })
})
