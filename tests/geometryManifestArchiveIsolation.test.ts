import { CURRENT_RUNTIME_GEOMETRY_EVIDENCE } from '../src/core/currentRuntimeGeometryEvidence'
import { expect, it, vi } from 'vitest'
import { GEOMETRY_MANIFEST_ARCHIVE } from '../src/core/geometryExecution'

it('keeps historical manifests byte-identical when current evidence changes', async () => {
  const oldMesh = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1']
  const oldBrep = GEOMETRY_MANIFEST_ARCHIVE['brep-closed-v1']
  expect(oldMesh.manifestDigest).toBe('c2cf439e9c8a983a8780d6b61bed117d2eb97d2b54e869eb7db3440cf2051885')
  expect(oldBrep.manifestDigest).toBe('62b04c37976f71323eeeb164eea5715b06bfb484769b048855f17b2c12ab9d0c')
  vi.resetModules()
  vi.doMock('../src/core/ownRustCadEvidence', () => ({
    OWN_RUST_CAD_EVIDENCE: {
      wasmSha256: '1'.repeat(64),
      noticesSha256: '2'.repeat(64),
      lockfileSha256: '3'.repeat(64),
      rustLockfileSha256: '4'.repeat(64),
    },
  }))
  try {
    const { GEOMETRY_MANIFEST_ARCHIVE: refreshed, computeGeometryManifestDigest } =
      await import('../src/core/geometryExecution')
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

it('keeps v6 through v47 runtime records immutable when current v48 evidence changes', async () => {
  const oldMesh = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v6']
  const oldBrep = GEOMETRY_MANIFEST_ARCHIVE['brep-closed-v6']
  const oldMeshV7 = GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v7']
  const oldBrepV7 = GEOMETRY_MANIFEST_ARCHIVE['brep-closed-v7']
  vi.resetModules()
  vi.doMock('../src/core/currentRuntimeGeometryEvidence', () => ({
    CURRENT_RUNTIME_GEOMETRY_EVIDENCE: {
      ...CURRENT_RUNTIME_GEOMETRY_EVIDENCE,
      recordId: 'candidate-test',
      corpusVersion: 'candidate-test',
    },
  }))
  try {
    const { GEOMETRY_MANIFEST_ARCHIVE: refreshed } = await import('../src/core/geometryExecution')
    expect(refreshed['own-rust-node-v6']).toEqual(oldMesh)
    expect(refreshed['brep-closed-v6']).toEqual(oldBrep)
    expect(refreshed['own-rust-node-v7']).toEqual(oldMeshV7)
    expect(refreshed['brep-closed-v7']).toEqual(oldBrepV7)
    expect(refreshed['own-rust-node-v8'].manifestDigest).toBe(
      'ff7de6cc61a30366ce33abc090cac42538abb228efc715210602e986859cf1ac',
    )
    expect(refreshed['brep-closed-v8'].manifestDigest).toBe(
      '84044b5663d7dfd790d01ac79de118270574699fe0c059ac8b1776e2f1a8fe58',
    )
    expect(refreshed['own-rust-node-v9'].manifestDigest).toBe(
      'b47594219e725a1416c426f76111e3b113fecee10025b94e41e30856902af1d4',
    )
    expect(refreshed['brep-closed-v9'].manifestDigest).toBe(
      'e0e66d367228efb219d171319c45bc0ac74b50b56cc6145f32a43ffb8ed6e9da',
    )
    expect(refreshed['own-rust-node-v10'].manifestDigest).toBe(
      'b28d2d479b3f762c96bd6cb34776c2c72d995049decfa155d249ab9e0199466f',
    )
    expect(refreshed['brep-closed-v10'].manifestDigest).toBe(
      '351e44c7c5417ddc14aca135eccaba13bc99544fa715f751600f8be6e8c663e5',
    )
    expect(refreshed['own-rust-node-v11'].manifestDigest).toBe(
      '18c89293122cf5cba9e3f32700f5cd27a5e4504ec790ab8635b6bdc6fe368dfb',
    )
    expect(refreshed['brep-closed-v11'].manifestDigest).toBe(
      '9de236fd8d7c4c4103fb383702b00f7173c23e4ff952700c9eed61b9159a8cab',
    )
    expect(refreshed['own-rust-node-v12'].manifestDigest).toBe(
      '1cebab15985c99b1c03538c942b4736f98a95e22a73c23c932f74bd87457c809',
    )
    expect(refreshed['brep-closed-v12'].manifestDigest).toBe(
      '54e27a0fc0189c541e0990e9f87d00b85214b399fbc5808b1bf48944577c56a8',
    )
    for (const id of ['own-rust-node-v13', 'brep-closed-v13'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    expect(refreshed['own-rust-node-v14']).toEqual(GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v14'])
    expect(refreshed['brep-closed-v14']).toEqual(GEOMETRY_MANIFEST_ARCHIVE['brep-closed-v14'])
    for (const id of ['own-rust-node-v15', 'brep-closed-v15'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v16', 'brep-closed-v16'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v17', 'brep-closed-v17'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v18', 'brep-closed-v18'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v19', 'brep-closed-v19'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v20', 'brep-closed-v20'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v21', 'brep-closed-v21'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v22', 'brep-closed-v22'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v23', 'brep-closed-v23'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v24', 'brep-closed-v24'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v25', 'brep-closed-v25'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v26', 'brep-closed-v26'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v27', 'brep-closed-v27'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v28', 'brep-closed-v28'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v29', 'brep-closed-v29'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v30', 'brep-closed-v30'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v31', 'brep-closed-v31'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v32', 'brep-closed-v32'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v33', 'brep-closed-v33'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v34', 'brep-closed-v34'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v35', 'brep-closed-v35'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v36', 'brep-closed-v36'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v37', 'brep-closed-v37'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v38', 'brep-closed-v38'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v39', 'brep-closed-v39'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v40', 'brep-closed-v40'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v41', 'brep-closed-v41'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v42', 'brep-closed-v42'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v43', 'brep-closed-v43'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v44', 'brep-closed-v44'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v45', 'brep-closed-v45'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v46', 'brep-closed-v46'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v47', 'brep-closed-v47'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    expect(refreshed['own-rust-node-v48'].qualification.recordId).toBe('candidate-test')
  } finally {
    vi.doUnmock('../src/core/currentRuntimeGeometryEvidence')
    vi.resetModules()
  }
})
