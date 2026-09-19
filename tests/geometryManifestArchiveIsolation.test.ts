import {expect, it, vi} from 'vitest'
import {GEOMETRY_MANIFEST_ARCHIVE} from '../src/core/geometryExecution'

it('keeps historical manifests byte-identical when current evidence changes', async () => {
  const oldMesh = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1']
  const oldBrep = GEOMETRY_MANIFEST_ARCHIVE['brep-closed-v1']
  expect(oldMesh.manifestDigest).toBe('c2cf439e9c8a983a8780d6b61bed117d2eb97d2b54e869eb7db3440cf2051885')
  expect(oldBrep.manifestDigest).toBe('62b04c37976f71323eeeb164eea5715b06bfb484769b048855f17b2c12ab9d0c')
  vi.resetModules()
  vi.doMock('../src/core/ownRustCadEvidence', () => ({OWN_RUST_CAD_EVIDENCE: {
    wasmSha256: '1'.repeat(64), noticesSha256: '2'.repeat(64),
    lockfileSha256: '3'.repeat(64), rustLockfileSha256: '4'.repeat(64),
  }}))
  try {
    const {GEOMETRY_MANIFEST_ARCHIVE: refreshed, computeGeometryManifestDigest} = await import('../src/core/geometryExecution')
    expect(refreshed['own-rust-node-v1']).toEqual(oldMesh)
    expect(refreshed['brep-closed-v1']).toEqual(oldBrep)
    for (const key of ['own-rust-node-v2', 'brep-closed-v2'] as const) {
      expect(refreshed[key].kernelFingerprint).toBe(`sha256:${'1'.repeat(64)}`)
      expect(refreshed[key].dependency.sbomSha256).toBe('2'.repeat(64))
      expect(refreshed[key].dependency.lockfileSha256).toBe('3'.repeat(64))
      expect(refreshed[key].qualification.status).toBe('baseline-pending')
      expect(computeGeometryManifestDigest(refreshed[key])).toBe(refreshed[key].manifestDigest)
    }
  } finally {
    vi.doUnmock('../src/core/ownRustCadEvidence')
    vi.resetModules()
  }
})
