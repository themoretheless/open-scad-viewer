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
  // v13: periodic section body and current merged geometry sources. Measured
  // 18,030,969 raw WASM bytes, 6,424,363 packed geometry bytes and 12,281,514
  // asset bytes. Both browser Worker loading paths verified this exact digest.
  '15f93b60958953d09a0c72e1e581636cd148d672470bc473e83387b112bcba30':Object.freeze({
    version:13,geometryChunkBytes:6_430_000,totalAssetBytes:12_290_000,rawGeometryBytes:18_040_000,
    namedJsChunkBytes:Object.freeze({
      index:250_500,DirectModeler:494_000,geometryBuildEngine:335_200,wasmStreaming:195_600,
    }),
    exactJsChunkBytes:Object.freeze({'assets/6YzdHiG9.js':108_000}),
  }),
  // v14: exact isolated contact certificates. Local production measured
  // 18,062,286 raw WASM bytes, 6,435,493 packed geometry bytes and 12,293,327
  // asset bytes. The raw kernel grew by 31,317 bytes over v13.
  '94dc1e2fae7de0c1764723278e22892aee749fc2b9a805ebe0c5c4a3b74837d0':Object.freeze({
    version:14,geometryChunkBytes:6_440_000,totalAssetBytes:12_300_000,rawGeometryBytes:18_070_000,
    namedJsChunkBytes:Object.freeze({
      index:250_500,DirectModeler:494_000,geometryBuildEngine:335_200,wasmStreaming:195_600,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CgGnarTW.js':108_000}),
  }),
  // v15: closed SS network Boolean. Local production measured 18,339,084 raw
  // WASM bytes, 6,537,252 packed geometry bytes and 12,401,470 asset bytes.
  '842f0872b21f2add7b9ee3cad71cd37dae38db02991f2fac96a859d4808d0f85':Object.freeze({
    version:15,geometryChunkBytes:6_545_000,totalAssetBytes:12_410_000,rawGeometryBytes:18_345_000,
    namedJsChunkBytes:Object.freeze({
      index:250_500,DirectModeler:494_000,geometryBuildEngine:335_200,wasmStreaming:195_600,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BjrmXJqH.js':111_500}),
  }),
  // v16: periodic stock intersection. Measured production build:
  // 18,523,833 raw WASM bytes, 6,604,053 packed geometry bytes, up to 12,468,855 asset bytes.
  '6e11301a009b087851660ace975eeee6230c4fdc89f16baa73bfd1cd1f63989e':Object.freeze({
    version:16,geometryChunkBytes:6_612_000,totalAssetBytes:12_475_000,rawGeometryBytes:18_530_000,
    namedJsChunkBytes:Object.freeze({
      index:250_500,DirectModeler:494_000,geometryBuildEngine:335_200,wasmStreaming:195_600,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DZpgor1M.js':111_500}),
  }),
  // v17: origin/main merged with ruled loft section correspondence and boundary-anchored face
  // workplanes (runtime v13 record). Measured production build: 18,531,352 raw WASM bytes,
  // 6,611,760 packed geometry bytes and 12,501,640 asset bytes. The index, DirectModeler and
  // wasmStreaming chunks measure 256,066, 496,707 and 195,621 bytes, identical with main's
  // src/core, so their growth over v16 comes from main.
  '61b58706d1412023064cc2bbfa2bbbaa48243caea45014d5a22d2b6276609ffc':Object.freeze({
    version:17,geometryChunkBytes:6_620_000,totalAssetBytes:12_510_000,rawGeometryBytes:18_540_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:497_300,geometryBuildEngine:335_200,wasmStreaming:196_200,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // v18: Fillet operation contract for audited multi-edge fillet, nested graph column Boolean and
  // off-axis sphere tracing across a cone apex plane. Measured production build: 18,553,202 raw
  // WASM bytes, 6,616,898 packed geometry bytes and 12,530,943 asset bytes. The index,
  // DirectModeler and wasmStreaming chunks measure 256,134, 500,918 and 196,179 bytes; the
  // kernel change is Rust-only, so the DirectModeler growth over v17 (496,707) comes from main.
  '9cb8ebbc7747c61730a2bc707a9bd4e235fc900c60cfe042a3919929535b4b31':Object.freeze({
    version:18,geometryChunkBytes:6_625_000,totalAssetBytes:12_539_000,rawGeometryBytes:18_562_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:501_500,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // v19: healing edge split across both face uses, STEP /10 ordinary pcurve repair and exact planar
  // face mass properties. Measured production build: 18,558,145 raw WASM bytes, 6,617,591 packed
  // geometry bytes and 12,581,815 asset bytes. The index, DirectModeler, geometryBuildEngine and
  // wasmStreaming chunks measure 256,134, 510,648, 332,568 and 196,179 bytes; the kernel change is
  // Rust-only, so the DirectModeler growth over v18 (500,918) comes from main.
  '7467ec8b0d7e378a0510cf94790bf4da2146b7b735d4204a80975e2afa514986':Object.freeze({
    version:19,geometryChunkBytes:6_626_000,totalAssetBytes:12_590_000,rawGeometryBytes:18_567_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // v20 measured 18,881,767 raw WASM bytes, 6,732,083 packed geometry bytes,
  // and 12,833,309 total asset bytes on the Solid and Rust integration build.
  '5c5f9b414d6c8db79af8d901314e97a747689ecdaab05ef08d3fb7c31dacc94a':Object.freeze({
    version:20,geometryChunkBytes:6_741_000,totalAssetBytes:12_842_000,rawGeometryBytes:18_891_000,
    namedJsChunkBytes:Object.freeze({
      index:271_900,DirectModeler:545_500,geometryBuildEngine:338_800,wasmStreaming:220_600,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // v21 measured 18,961,944 raw WASM bytes, 6,759,159 packed geometry bytes,
  // and 12,861,112 total asset bytes for the loft integration.
  '516fa1bfeb9005d388792b6e7ba6a996c8236d5ccaec82ade454eea191050fd6':Object.freeze({
    version:21,geometryChunkBytes:6_768_100,totalAssetBytes:12_870_100,rawGeometryBytes:18_971_500,
    namedJsChunkBytes:Object.freeze({
      index: 271_900, DirectModeler: 545_700, geometryBuildEngine: 339_000, wasmStreaming: 220_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/workerHandlerRuntime-BYRGBGGQ.js':114_700}),
  }),
  // v22 measured 18,996,362 raw WASM bytes, 6,770,982 packed geometry bytes,
  // and 12,873,243 total asset bytes.
  '8a82d7afc51a7ee70d1e5bfb36623a2c76d8a86f16f9703e2f86c97884c50bf1':Object.freeze({
    version:22,geometryChunkBytes:6_780_000,totalAssetBytes:12_882_300,rawGeometryBytes:19_006_000,
    namedJsChunkBytes:Object.freeze({
      index:271_900,DirectModeler:545_700,geometryBuildEngine:339_000,wasmStreaming:220_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/workerHandlerRuntime-DjZK5xjP.js':114_700}),
  }),
  // v23 measured 19,016,254 raw WASM bytes, 6,781,272 packed geometry bytes,
  // and 12,868,831 total asset bytes after merging main and the Jordan certificates.
  '71294e3370d75ca6b9b1a162b2ef1b2ab9d1957b3059e60c1859ce2170f6b49a':Object.freeze({
    version:23,geometryChunkBytes:6_791_000,totalAssetBytes:12_878_000,rawGeometryBytes:19_026_000,
    namedJsChunkBytes:Object.freeze({
      index:271_900,DirectModeler:499_000,geometryBuildEngine:339_000,wasmStreaming:220_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/workerHandlerRuntime-B6B-VcZM.js':114_700}),
  }),
  // v24 measured 19,087,967 raw WASM bytes, 6,806,892 packed geometry bytes,
  // and 12,894,974 total asset bytes for the merged loft and affine shell runtime.
  'a0f18dd409f746f9930d0b4c966747cc8b25e813711762ff8edabfe152fd6f2a':Object.freeze({
    version:24,geometryChunkBytes:6_817_000,totalAssetBytes:12_905_000,rawGeometryBytes:19_098_000,
    namedJsChunkBytes:Object.freeze({
      index:271_900,DirectModeler:499_000,geometryBuildEngine:339_000,wasmStreaming:220_800,
    }),
  }),
  // v25 measured 19,096,303 raw geometry bytes, 6,809,720 packed bytes and 12,900,792 asset bytes.
  '7702e12173f81b2db74472762ce0f5e9824be6d65311b4d2142416c7f1cb0ae4':Object.freeze({
    version:25,geometryChunkBytes:6_817_000,totalAssetBytes:12_905_000,rawGeometryBytes:19_098_000,
    namedJsChunkBytes:Object.freeze({
      index:271_900,DirectModeler:499_000,geometryBuildEngine:339_000,wasmStreaming:220_800,
    }),
  }),
})

export function selectGeometryDeliveryBudget(wasmSha256){
  if(typeof wasmSha256!=='string'||!Object.hasOwn(profiles,wasmSha256))
    throw new Error('Geometry artifact has no reviewed delivery budget profile')
  return profiles[wasmSha256]
}
