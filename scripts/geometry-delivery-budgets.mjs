// Packaging policy, not geometry tolerances, proof work, memory or WASM limits.
// A new artifact cannot inherit a larger budget without a reviewed exact pin.
const profiles=Object.freeze({
  '2ff06f662379e92193ff80849ce03bac9ab28c82d00223ff9a48e4d0307ca058':Object.freeze({
    version:6,geometryChunkBytes:4_680_000,totalAssetBytes:9_090_000,
  }),
  // v7: 5,360,574 geometry bytes / 10,066,845 total assets, measured locally.
  // The controlled pre-refactor input already exceeded the previous v6 cap.
  '91521ffbc32570cddc99b4555dcdd3f55fdaa902a806fccc215bf7afefab129f':Object.freeze({
    version:7,geometryChunkBytes:5_370_000,totalAssetBytes:10_080_000,
  }),
  // v8: 5,360,256 geometry bytes / 10,068,403 total assets, measured locally.
  // Retain the v7 limits; only this independently measured artifact is added.
  '442d2703d243fe29ed8434dcbce04e7cf06c2be36be149af07b51b00e45160f6':Object.freeze({
    version:8,geometryChunkBytes:5_370_000,totalAssetBytes:10_080_000,
  }),
  // v9 packaging only: exact measured390fb9 artifact; same v7/v8 caps.
  // Review: docs/qualification/geometry-delivery-390fb9-2026-11-02/review.json
  '390fb9f1e731a8de56228e7bcb6454eae96f28f06df621a9d1ca8f489b2a0201':Object.freeze({
    version:9,geometryChunkBytes:5_370_000,totalAssetBytes:10_080_000,
  }),
  // v10: exactbaf58e bytes, sept payload + full CSS emission measured within packaging cap.
  'baf58e7051b0a9b39b54b61525ba014abb0e1bcf0b71c9a73df418e796cfb254':Object.freeze({
    version:10,geometryChunkBytes:5_370_000,totalAssetBytes:10_160_000,
  }),
  // v11: exact 15596a bytes (G0 v106 / runtime v11 re-freeze). Measured locally:
  // 6,272,449 packed geometry bytes and 12,034,839 total asset bytes; the kernel grew
  // from 15.9 MB to 17.7 MB raw since baf58e. Caps keep a small reviewed margin.
  '15596aa8bd7c26888ae18fd1b0bcea49c36af76ef50b1a5558a6e164c142e938':Object.freeze({
    version:11,geometryChunkBytes:6_280_000,totalAssetBytes:12_050_000,
  }),
  // v12: exact fba78e bytes (G0 v107 / runtime v12 re-freeze). Measured locally:
  // 6,272,442 packed geometry bytes and 12,057,424 total asset bytes, including the
  // CodeMirror editor and other main-branch UI growth. Caps keep a small reviewed margin.
  'fba78e35f862c3259769f5d7a2458b6f060a61a104c5e7b94e3b13ec2d5da837':Object.freeze({
    version:12,geometryChunkBytes:6_280_000,totalAssetBytes:12_070_000,
  }),
})

export function selectGeometryDeliveryBudget(wasmSha256){
  if(typeof wasmSha256!=='string'||!Object.hasOwn(profiles,wasmSha256))
    throw new Error('Geometry artifact has no reviewed delivery budget profile')
  return profiles[wasmSha256]
}
