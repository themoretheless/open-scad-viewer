import {createHash} from 'node:crypto'
import {describe, expect, it} from 'vitest'
import {compareRuntimeGeometryIdentity} from '../scripts/audit-runtime-geometry-identity.mts'
import {ACTIVE_GEOMETRY_MANIFEST_VERSIONS, GEOMETRY_MANIFEST_ARCHIVE} from '../src/core/geometryExecution'

describe('runtime geometry identity audit', () => {
  const bytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])
  const fingerprint = `sha256:${createHash('sha256').update(bytes).digest('hex')}`
  const mesh = GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh]
  const brep = GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep]

  it('defaults to the actual active mesh and B-rep bindings', () => {
    const report = compareRuntimeGeometryIdentity(bytes)
    expect(report.records.map(record => record.capabilityManifestVersion))
      .toEqual(Object.values(ACTIVE_GEOMETRY_MANIFEST_VERSIONS))
    expect(report.records.every(record => record.observedFingerprint === fingerprint)).toBe(true)
    expect(report.qualificationClaim).toBe('none')
  })

  it('reports every mismatch without changing archived manifests', () => {
    const before = JSON.stringify([mesh, brep])
    const report = compareRuntimeGeometryIdentity(bytes, [{...mesh, kernelFingerprint: fingerprint}, brep])
    expect(report.matches).toBe(false)
    expect(report.records.map(record => record.matches)).toEqual([true, false])
    expect(JSON.stringify([mesh, brep])).toBe(before)
    expect(report.records[1].expectedFingerprint).toBe(brep.kernelFingerprint)
  })

  it('does not convert a matching fingerprint into a qualification claim', () => {
    const pending = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v2']
    const report = compareRuntimeGeometryIdentity(bytes, [{...pending, kernelFingerprint: fingerprint}])
    expect(report.matches).toBe(true)
    expect(report.records[0].qualificationStatus).toBe('baseline-pending')
    expect(report.qualificationClaim).toBe('none')
    expect(() => compareRuntimeGeometryIdentity(bytes, [])).toThrow('No runtime bindings')
  })
})
