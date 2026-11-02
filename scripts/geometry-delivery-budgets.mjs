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
  // v10: exactbaf58e bytes, sept payload measured within unchanged caps.
  'baf58e7051b0a9b39b54b61525ba014abb0e1bcf0b71c9a73df418e796cfb254':Object.freeze({
    version:10,geometryChunkBytes:5_370_000,totalAssetBytes:10_080_000,
  }),
})

export function selectGeometryDeliveryBudget(wasmSha256){
  if(typeof wasmSha256!=='string'||!Object.hasOwn(profiles,wasmSha256))
    throw new Error('Geometry artifact has no reviewed delivery budget profile')
  return profiles[wasmSha256]
}
