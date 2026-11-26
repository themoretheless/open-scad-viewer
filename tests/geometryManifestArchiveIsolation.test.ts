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

it('keeps v6 through v204 runtime records immutable when current v205 evidence changes', async () => {
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
    for (const id of ['own-rust-node-v48', 'brep-closed-v48'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v49', 'brep-closed-v49'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v50', 'brep-closed-v50'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v51', 'brep-closed-v51'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v52', 'brep-closed-v52'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v53', 'brep-closed-v53'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v54', 'brep-closed-v54'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v55', 'brep-closed-v55'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v56', 'brep-closed-v56'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v57', 'brep-closed-v57'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v58', 'brep-closed-v58'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v59', 'brep-closed-v59'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v60', 'brep-closed-v60'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v61', 'brep-closed-v61'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v62', 'brep-closed-v62'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v63', 'brep-closed-v63'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v64', 'brep-closed-v64'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v65', 'brep-closed-v65'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v66', 'brep-closed-v66'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v67', 'brep-closed-v67'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v68', 'brep-closed-v68'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v69', 'brep-closed-v69'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v70', 'brep-closed-v70'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v71', 'brep-closed-v71'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v72', 'brep-closed-v72'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v73', 'brep-closed-v73'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v74', 'brep-closed-v74'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v75', 'brep-closed-v75'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v76', 'brep-closed-v76'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v77', 'brep-closed-v77'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v78', 'brep-closed-v78'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v79', 'brep-closed-v79'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v80', 'brep-closed-v80'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v81', 'brep-closed-v81'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v82', 'brep-closed-v82'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v83', 'brep-closed-v83'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v84', 'brep-closed-v84'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v85', 'brep-closed-v85'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v86', 'brep-closed-v86'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v87', 'brep-closed-v87'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v88', 'brep-closed-v88'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v89', 'brep-closed-v89'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v90', 'brep-closed-v90'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v91', 'brep-closed-v91'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v92', 'brep-closed-v92'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v93', 'brep-closed-v93'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v94', 'brep-closed-v94'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v95', 'brep-closed-v95'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v96', 'brep-closed-v96'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v97', 'brep-closed-v97'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v98', 'brep-closed-v98'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v99', 'brep-closed-v99'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v100', 'brep-closed-v100'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v101', 'brep-closed-v101'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v102', 'brep-closed-v102'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v103', 'brep-closed-v103'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v104', 'brep-closed-v104'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v105', 'brep-closed-v105'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v106', 'brep-closed-v106'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v107', 'brep-closed-v107'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v108', 'brep-closed-v108'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v109', 'brep-closed-v109'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v110', 'brep-closed-v110'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v111', 'brep-closed-v111'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v112', 'brep-closed-v112'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v113', 'brep-closed-v113'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v114', 'brep-closed-v114'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v115', 'brep-closed-v115'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v116', 'brep-closed-v116'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v117', 'brep-closed-v117'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v118', 'brep-closed-v118'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v119', 'brep-closed-v119'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v120', 'brep-closed-v120'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v121', 'brep-closed-v121'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v122', 'brep-closed-v122'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v123', 'brep-closed-v123'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v124', 'brep-closed-v124'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v125', 'brep-closed-v125'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v126', 'brep-closed-v126'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v127', 'brep-closed-v127'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v128', 'brep-closed-v128'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v129', 'brep-closed-v129'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v130', 'brep-closed-v130'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v131', 'brep-closed-v131'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v132', 'brep-closed-v132'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v133', 'brep-closed-v133'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v134', 'brep-closed-v134'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v135', 'brep-closed-v135'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v136', 'brep-closed-v136'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v137', 'brep-closed-v137'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v138', 'brep-closed-v138'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v139', 'brep-closed-v139'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v140', 'brep-closed-v140'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v141', 'brep-closed-v141'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v142', 'brep-closed-v142'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v143', 'brep-closed-v143'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v144', 'brep-closed-v144'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v145', 'brep-closed-v145'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v146', 'brep-closed-v146'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v147', 'brep-closed-v147'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v148', 'brep-closed-v148'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v149', 'brep-closed-v149'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v150', 'brep-closed-v150'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v151', 'brep-closed-v151'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v152', 'brep-closed-v152'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v153', 'brep-closed-v153'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v154', 'brep-closed-v154'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v155', 'brep-closed-v155'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v156', 'brep-closed-v156'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v157', 'brep-closed-v157'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v158', 'brep-closed-v158'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v159', 'brep-closed-v159'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v160', 'brep-closed-v160'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v161', 'brep-closed-v161'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v162', 'brep-closed-v162'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v163', 'brep-closed-v163'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v164', 'brep-closed-v164'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v165', 'brep-closed-v165'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v166', 'brep-closed-v166'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v167', 'brep-closed-v167'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v168', 'brep-closed-v168'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v169', 'brep-closed-v169'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v170', 'brep-closed-v170'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v171', 'brep-closed-v171'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v172', 'brep-closed-v172'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v173', 'brep-closed-v173'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v174', 'brep-closed-v174'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v175', 'brep-closed-v175'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v176', 'brep-closed-v176'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v177', 'brep-closed-v177'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v178', 'brep-closed-v178'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v179', 'brep-closed-v179'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v180', 'brep-closed-v180'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v181', 'brep-closed-v181'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v182', 'brep-closed-v182'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v183', 'brep-closed-v183'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v184', 'brep-closed-v184'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v185', 'brep-closed-v185'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v186', 'brep-closed-v186'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v187', 'brep-closed-v187'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v188', 'brep-closed-v188'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v189', 'brep-closed-v189'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v190', 'brep-closed-v190'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v191', 'brep-closed-v191'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v192', 'brep-closed-v192'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v193', 'brep-closed-v193'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v194', 'brep-closed-v194'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v195', 'brep-closed-v195'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v196', 'brep-closed-v196'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v197', 'brep-closed-v197'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v198', 'brep-closed-v198'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v199', 'brep-closed-v199'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v200', 'brep-closed-v200'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v201', 'brep-closed-v201'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v202', 'brep-closed-v202'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v203', 'brep-closed-v203'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    for (const id of ['own-rust-node-v204', 'brep-closed-v204'] as const) {
      expect(refreshed[id]).toEqual(GEOMETRY_MANIFEST_ARCHIVE[id])
    }
    expect(refreshed['own-rust-node-v205'].qualification.recordId).toBe('candidate-test')
  } finally {
    vi.doUnmock('../src/core/currentRuntimeGeometryEvidence')
    vi.resetModules()
  }
})
