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
  // Shared STEP export build. Measured raw kernel 18,907,780 bytes,
  // packed geometry 6,741,729 bytes and distribution assets 12,831,417 bytes.
  '087654c90fa17d70d73bb3aeab3f79179bf865393575ee55cc85a367bcbbab4f':Object.freeze({
    version:20,geometryChunkBytes:6_750_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_916_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Grouped and coloured STEP placements: 18,911,816 raw bytes,
  // 6,742,478 packed bytes and 12,835,337 distribution asset bytes, within v20 limits.
  'f56862ea191a58ddc025ae71ffd5b99aebc9fc8f3147b2d6abbe8369d98cc080':Object.freeze({
    version:21,geometryChunkBytes:6_750_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_916_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Spinner bore Boolean and compact STEP UV curves: 18,934,818 raw bytes,
  // 6,750,143 packed geometry bytes and 12,820,424 asset bytes.
  '9274758d2f6c3fee4c5c880e50ba475bb03cc17aef42dc7be2dd2f4884a7d07a':Object.freeze({
    version:22,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Engineering profiles: raw WASM 19015858, packed geometry 6775441, distribution assets 12851095.
  '9d18af4d9b9cd8f8eec0cf45cbbc595c4c51946c8208d082ad41c55d4998305a':Object.freeze({
    version:23,geometryChunkBytes:6777000,totalAssetBytes:12857000,rawGeometryBytes:19016000,
    namedJsChunkBytes:Object.freeze({index:256600,DirectModeler:511200,geometryBuildEngine:335200,wasmStreaming:196700}),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Sweep proof kernel with the same size caps as v22.
  'ae4656234ee9522506e958287cd019e8adc881947d742bc787fb72d404a20b30':Object.freeze({
    version:24,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Surface split, transpose and Bezier decomposition: 18,734,435 raw bytes, retaining the v24 limits.
  '1c2649097862e2cf909d41b3962701d76aed6d098df4c1f6666a021102eae53f':Object.freeze({
    version:25,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Verified periodic seam relocation and editable weights: 18,760,944 raw bytes, retaining the v25 limits.
  '18d8435331735d1613dccfaabbb4932e023b8667778a5ea056861c4bcd48ec37':Object.freeze({
    version:26,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Exact curve and surface edits: 18,792,232 raw bytes, retaining the v26 limits.
  '816b475eeb5eed74abe3fe32a5c2ebdf8bf032218e3c6cfa6977ebf5f006bf95':Object.freeze({
    version:27,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({}),
  }),
  // Rational factors and exact restrictions: 18,813,732 raw bytes, unchanged aggregate limits. Geometry worker dependency measured at 100,060 bytes.
  '2d6c6814ec68669abef73f2d9e73ba3906c0f5cec7ca04082e7a8e89660c9d3e':Object.freeze({
    version:28,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BG1iAHJS.js':100_100}),
  }),
  // Exact periodic surface edits: 18,845,473 raw bytes, 6,732,643 packed bytes and 12,837,450 asset bytes.
  'e352c3dd3de504df52741ab341fab44db6ca85be7713306d35c72375d4417326':Object.freeze({
    version:29,geometryChunkBytes:6_760_000,totalAssetBytes:12_840_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CzWQW3wi.js':101_100}),
  }),
  // Periodic strip and grid assembly: 18,861,337 raw bytes, 6,738,481 packed bytes and 12,850,901 asset bytes with editor measurement reports.
  '83867cf774759f75cb5ab73a12ba2edef0af998989d45fc7c0cb16d06e2a682d':Object.freeze({
    version:30,geometryChunkBytes:6_760_000,totalAssetBytes:12_860_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Bo28C-Kr.js':102_100}),
  }),
  // Axial derivative peaks and their parameter locations: 18,879,542 raw bytes, 6,743,640 packed bytes and 12,859,443 asset bytes.
  '62464fd2457d45bf5a76469b76081029d81e0b841fb02ea60444bf9b2247ab59':Object.freeze({
    version:31,geometryChunkBytes:6_760_000,totalAssetBytes:12_860_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/glh_OBZk.js':102_500}),
  }),
  // Axial motion limits, duration and slowdown: 18,894,516 raw bytes, 6,747,112 packed bytes and 12,867,338 asset bytes.
  '6e5130767022c223c9e5612d878b51315f6a0d899c72cb718d13e50c483e053c':Object.freeze({
    version:32,geometryChunkBytes:6_760_000,totalAssetBytes:12_870_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Xy2E_Dxs.js':103_000}),
  }),
  // Motion interval partitions and full-domain travel: 18,921,391 raw bytes, 6,754,287 packed bytes and 12,893,707 asset bytes with exact rational trim ranges.
  'd443112ced2ec682c03930fe6eec4e101efd86c34e71c970b1a9efb62c0af10b':Object.freeze({
    version:33,geometryChunkBytes:6_760_000,totalAssetBytes:12_900_000,rawGeometryBytes:18_940_000,
    namedJsChunkBytes:Object.freeze({
      index:256_600,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CQuDnQJd.js':108_600}),
  }),
  // Exact periodic UV path measurements: 18,953,072 raw bytes, 6,765,284 packed bytes and 12,908,847 asset bytes.
  'a7946f7571dc256d52f9ed122a15c295716be7e3a318daded61bca3dd738a797':Object.freeze({
    version:34,geometryChunkBytes:6_770_000,totalAssetBytes:12_910_000,rawGeometryBytes:18_960_000,
    namedJsChunkBytes:Object.freeze({
      index:257_200,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CLGKAjgu.js':109_000}),
  }),
  // Exact periodic region classification and moments: 18,991,115 raw bytes, 6,776,652 packed bytes and 12,924,817 asset bytes.
  '135414b46f669329bdf777a21b8b5be985cddff64227eac73f182134e6c3cf65':Object.freeze({
    version:35,geometryChunkBytes:6_780_000,totalAssetBytes:12_930_000,rawGeometryBytes:19_000_000,
    namedJsChunkBytes:Object.freeze({
      index:258_400,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BFHbW7Ps.js':109_700}),
  }),
  // Trim contour distance and projection reports: 19,014,081 raw bytes, 6,783,057 packed bytes and assets including the merged loft editor and trimmed-sheet controls.
  'c6190d70c800ea78c41fa6d94bd94e8775b773b601911167a4d8b3137dfc24f2':Object.freeze({
    version:36,geometryChunkBytes:6_790_000,totalAssetBytes:12_940_000,rawGeometryBytes:19_020_000,
    namedJsChunkBytes:Object.freeze({
      index:264_500,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/do38haOu.js':110_900}),
  }),
  // Exact input parameters and one-sided tangents on periodic trims: 19,178,023 raw bytes, 6,847,429 packed bytes and 13,000,819 asset bytes.
  '990c5e6e4506e1f44ebef044ea2fec379b9575bbddb7c3cc78b413617ca3ad5c':Object.freeze({
    version:37,geometryChunkBytes:6_850_000,totalAssetBytes:13_010_000,rawGeometryBytes:19_180_000,
    namedJsChunkBytes:Object.freeze({
      index:265_500,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/COMyN-JL.js':111_100}),
  }),
  // Trim-edge bindings, mesh and STEP delivery: 19,199,922 raw bytes, 6,855,332 packed bytes and 13,014,188 asset bytes.
  '030e4a378b6b4cec1f36732d0bf8dabc3b440aa4cac134b04c505b4d23bf2484':Object.freeze({
    version:38,geometryChunkBytes:6_860_000,totalAssetBytes:13_020_000,rawGeometryBytes:19_200_000,
    namedJsChunkBytes:Object.freeze({
      index:266_100,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CXEZE1xc.js':112_300,'assets/mainSolid.worker-CKQd3Niq.js':177_700}),
  }),
  // Trim component geometry and bounded trimmed surface integrals: 19,213,978 raw bytes, 6,858,734 packed bytes and 13,022,945 asset bytes.
  '5e225b0293e21e0cb2f95adef30f4a00050403a7fa46f456ec9e5a7a8c80ba77':Object.freeze({
    version:39,geometryChunkBytes:6_860_000,totalAssetBytes:13_030_000,rawGeometryBytes:19_220_000,
    namedJsChunkBytes:Object.freeze({
      index:267_400,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/mainSolid.worker-DIwXa-UR.js':178_300,'assets/DlBCRQE0.js':113_000}),
  }),
  // Verified gear profile and nominal wall bounds: 19,229,741 raw bytes, 6,865,524 packed bytes and 13,031,988 asset bytes.
  '13d92c870844e7c7f5d64b0a870c9a2ce8321d3c52b472f3a80aa21506cabe5e':Object.freeze({
    version:40,geometryChunkBytes:6_870_000,totalAssetBytes:13_040_000,rawGeometryBytes:19_230_000,
    namedJsChunkBytes:Object.freeze({
      index:267_800,DirectModeler:511_200,geometryBuildEngine:335_200,wasmStreaming:196_700,
    }),
    exactJsChunkBytes:Object.freeze({'assets/m51LYVn7.js':113_200,'assets/mainSolid.worker-dP66mhXS.js':178_900}),
  }),
  // Unified SKADIS + Voronoi lattice kernel & generator UI + named/grouped 3MF export: 19,240,497 raw bytes and 6,870,039 packed bytes.
  '2f0e1b7156582268666f0813d1bb689941b4591367d73f5cdd2022e1f521b70a':Object.freeze({
    version:48,geometryChunkBytes:6_875_000,totalAssetBytes:13_800_000,rawGeometryBytes:19_250_000,
    namedJsChunkBytes:Object.freeze({
      index:269_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DSjI2jWL.js':204_000,'assets/DwdnO0M6.js':175_500,'assets/mainSolid.worker-CnQLZS06.js':180_500}),
  }),
  // Rack, bevel gear and radial cam: 19,311,891 raw bytes, 6,889,996 packed bytes and 13,816,855 total asset bytes.
  'b12aeb458afec60a271f415fcb4330201d8c0997d08de242aa937e2b839429c4':Object.freeze({
    version:49,geometryChunkBytes:6_890_000,totalAssetBytes:13_817_000,rawGeometryBytes:19_312_000,
    namedJsChunkBytes:Object.freeze({
      index:270_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CHRq6vwt.js':203_600,'assets/mainSolid.worker-CLIcZhRT.js':180_200,'assets/nhGMEiGw.js':175_700}),
  }),
  // Finite external and internal threads: 19,430,452 raw bytes, 6,934,516 packed bytes and 13,864,347 total asset bytes.
  'c2665d0bb6ad1b89700dd18a028cc14b1c5f63373bdc1ab8f66b4c08231ca111':Object.freeze({
    version:50,geometryChunkBytes:6_935_000,totalAssetBytes:13_865_000,rawGeometryBytes:19_431_000,
    namedJsChunkBytes:Object.freeze({
      index:270_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BQGN7RBM.js':175_900,'assets/BfqnPdKM.js':203_600,'assets/mainSolid.worker-BwX6FH7i.js':180_800}),
  }),
  // Finite ZA worm and wheel, incremental source validation and lossless Brotli snapshots: 20,504,318 raw bytes, 7,380,787 packed bytes and 14,318,471 total asset bytes.
  '4a39c0cbccf4da30610b807f6a0256e1c42cb37bcbfc9b8d431a2aea21476b28':Object.freeze({
    version:51,geometryChunkBytes:7_381_000,totalAssetBytes:14_319_000,rawGeometryBytes:20_505_000,
    namedJsChunkBytes:Object.freeze({
      index:271_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BBEIF7pP.js':176_600,'assets/DHjc9Gcs.js':203_600,'assets/mainSolid.worker-UcPV213b.js':181_400}),
  }),
  // Base tangent gear span and thinning inspection: 20,509,473 raw bytes, 7,381,823 packed bytes and 14,322,821 total asset bytes.
  'ef7a8795a09165d78de98a03dbade07bb9ff7f7e20a903ff1c05e6070c607d75':Object.freeze({
    version:52,geometryChunkBytes:7_382_000,totalAssetBytes:14_323_000,rawGeometryBytes:20_510_000,
    namedJsChunkBytes:Object.freeze({
      index:271_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BWb8HeFi.js':203_600,'assets/CSmmaEwk.js':176_900,'assets/mainSolid.worker-DYpC4sxB.js':182_100}),
  }),
  // Cubic waves and exact optical cap surfaces: 20,517,381 raw bytes, 7,384,126 packed bytes and 14,328,848 total asset bytes.
  '90264575ed4c93bffcad7a6cddbfbf50d747dc545720963329c44beeb522ae18':Object.freeze({
    version:53,geometryChunkBytes:7_385_000,totalAssetBytes:14_329_000,rawGeometryBytes:20_518_000,
    namedJsChunkBytes:Object.freeze({
      index:271_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/7BuhttlD.js':177_400,'assets/DEZSYXAD.js':203_600,'assets/mainSolid.worker-CpYWBzZ2.js':182_700}),
  }),
  // Corrugated bellows inner and outer skins and annular rims: 20,523,458 raw bytes, 7,386,205 packed bytes and 14,334,287 total asset bytes.
  '7dc9354d07964db84e3b6e251ead0f4b71d1e9c4c9604dd0b98c8dc37588b502':Object.freeze({
    version:54,geometryChunkBytes:7_387_000,totalAssetBytes:14_335_000,rawGeometryBytes:20_524_000,
    namedJsChunkBytes:Object.freeze({
      index:271_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/X38o5o91.js':177_700,'assets/dYq9hK0k.js':203_600,'assets/mainSolid.worker-Bv9WsNA6.js':183_300}),
  }),
  // Signed-power superellipsoid patches and interpolation estimate: 20,530,034 raw bytes, 7,387,759 packed bytes and 14,338,205 total asset bytes.
  '48a5edb3db99a38df2538b741e0dd111e22cddac6e96057f3d4826ce0acc6d7d':Object.freeze({
    version:55,geometryChunkBytes:7_388_000,totalAssetBytes:14_339_000,rawGeometryBytes:20_531_000,
    namedJsChunkBytes:Object.freeze({
      index:271_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DIMHgtrh.js':178_000,'assets/DfGBwY9g.js':203_600,'assets/mainSolid.worker-3NvgMff_.js':184_000}),
  }),
  // Original curve and surface metrology: 20,537,566 raw bytes, 7,390,454 packed bytes and 14,345,160 total asset bytes.
  '70c2b175031c3d2a622783f9af27c7f5a339d6733fffcafc35b68564f01248ad':Object.freeze({
    version:56,geometryChunkBytes:7_391_000,totalAssetBytes:14_346_000,rawGeometryBytes:20_538_000,
    namedJsChunkBytes:Object.freeze({
      index:272_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BYrEraiI.js':178_400,'assets/PZ05rwhm.js':203_600,'assets/mainSolid.worker-N38d199J.js':184_600}),
  }),
  // Nearest points and distance intervals for original curves and surfaces: 20,549,079 raw bytes, 7,396,075 packed bytes and 14,355,539 total asset bytes.
  '5acc90be5969ff15a7abe6bb512060b15bf42a3ef558e9df321a10a8d37dc09b':Object.freeze({
    version:57,geometryChunkBytes:7_397_000,totalAssetBytes:14_356_000,rawGeometryBytes:20_550_000,
    namedJsChunkBytes:Object.freeze({
      index:272_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DKQXKyQu.js':179_100,'assets/Dnjqrwu-.js':203_600,'assets/mainSolid.worker-B35oCQR-.js':185_200}),
  }),
  // Curve curvature, torsion and principal surface curvatures: 20,560,577 raw bytes, 7,397,223 packed bytes and 14,359,940 total asset bytes.
  '6ea78540a060f465b9f85a7184737428bfbd87a1ba889801f898f313a1e195b1':Object.freeze({
    version:58,geometryChunkBytes:7_398_000,totalAssetBytes:14_360_000,rawGeometryBytes:20_561_000,
    namedJsChunkBytes:Object.freeze({
      index:273_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BgsTt7bA.js':203_600,'assets/DWIaTQej.js':179_500,'assets/mainSolid.worker-Dibhe8ef.js':185_800}),
  }),
  // Original curve and ray intersection queries: 20,631,697 raw bytes, 7,423,424 packed bytes and 14,390,833 total asset bytes.
  '38f9d6c17208966015e9cf035c7badf1d7dae1c13271f8795b006aef2b2a8691':Object.freeze({
    version:59,geometryChunkBytes:7_424_000,totalAssetBytes:14_391_000,rawGeometryBytes:20_632_000,
    namedJsChunkBytes:Object.freeze({
      index:273_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BhdmR_Kl.js':203_600,'assets/FlBkefjU.js':180_100,'assets/mainSolid.worker-Bhg6fRlb.js':186_500}),
  }),
  // Original self-contact and surface regularity audits: 20,646,442 raw bytes, 7,428,457 packed bytes and 14,399,987 total asset bytes.
  '6ade5583d10d6c34a51784fccb60349bb4346645664c0dfedd09db229ec85044':Object.freeze({
    version:60,geometryChunkBytes:7_429_000,totalAssetBytes:14_400_000,rawGeometryBytes:20_647_000,
    namedJsChunkBytes:Object.freeze({
      index:273_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/JhG2wTjp.js':203_600,'assets/hPOs8ny8.js':180_600,'assets/mainSolid.worker-hkv2VJeK.js':187_100}),
  }),
  // Endpoint position, tangent and curvature continuity audits: 20,654,905 raw bytes, 7,432,790 packed bytes and 14,406,705 total asset bytes.
  '68eb72c25943d35279ff3eca4e2ab0ae40cf23c1fad02368e2f29d5fee6b42dc':Object.freeze({
    version:61,geometryChunkBytes:7_433_000,totalAssetBytes:14_407_000,rawGeometryBytes:20_655_000,
    namedJsChunkBytes:Object.freeze({
      index:274_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BI8sv_y5.js':203_600,'assets/CJGCgMkc.js':180_800,'assets/mainSolid.worker-BeT-sMdL.js':187_700}),
  }),
  // Surface seam position, tangent plane and signed curvature audits: 20,669,677 raw bytes, 7,437,942 packed bytes and 14,415,099 total asset bytes.
  'ff126d7f3ea8611f95c4ee373adfc00691ff31af43ba11d6e1f78defb6d33fe0':Object.freeze({
    version:62,geometryChunkBytes:7_438_000,totalAssetBytes:14_416_000,rawGeometryBytes:20_670_000,
    namedJsChunkBytes:Object.freeze({
      index:274_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CUSJ7sJr.js':181_000,'assets/cb08XRKx.js':203_600,'assets/mainSolid.worker-CVxs0nRv.js':188_400}),
  }),
  // Whole-domain corresponding-parameter edit deviations: 20,671,039 raw bytes, 7,438,642 packed bytes and 14,418,059 total asset bytes.
  'fe2b8e900e68dae7445a43e0264e43fab4a2dca2a05ef716b1dbd3f6ceaad9b5':Object.freeze({
    version:63,geometryChunkBytes:7_439_000,totalAssetBytes:14_419_000,rawGeometryBytes:20_672_000,
    namedJsChunkBytes:Object.freeze({
      index:274_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/D3irrdBD.js':181_100,'assets/kJdrB_ZS.js':203_600,'assets/mainSolid.worker-o01A-V6t.js':189_000}),
  }),
  // Exact UV joins, simplicity and trim-domain classification: 20,674,308 raw bytes, 7,436,157 packed bytes and 14,419,025 total asset bytes.
  '0b04ef1cd52aa5ee88f416b4483d80eaed5a308630883712b45c63558cdb6548':Object.freeze({
    version:64,geometryChunkBytes:7_439_000,totalAssetBytes:14_420_000,rawGeometryBytes:20_675_000,
    namedJsChunkBytes:Object.freeze({
      index:274_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/kLoaIiT9.js':203_600,'assets/mainSolid.worker-DUA8J7_F.js':189_600,'assets/xWIRwJnH.js':181_300}),
  }),
  // Common section bases and orientation selection: 20,676,984 raw bytes, 7,438,901 packed bytes and 14,424,697 total asset bytes.
  'f7ff284f23fd4ca00861c4ed176a5d6c7bfbc50691b2d2dee3d137eaf45341f6':Object.freeze({
    version:65,geometryChunkBytes:7_439_000,totalAssetBytes:14_425_000,rawGeometryBytes:20_677_000,
    namedJsChunkBytes:Object.freeze({
      index:275_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B6KMcSQD.js':181_500,'assets/BzIyG4MO.js':203_600,'assets/mainSolid.worker-DFkod1WV.js':190_200}),
  }),
  // Closed-section landmark alignment and continuous shape error: 20,683,871 raw bytes, 7,440,630 packed bytes and 14,429,395 total asset bytes.
  '097e751e8db7cbbcdf617b2cbab6156619916feeab278fc4453b6e877b165977':Object.freeze({
    version:66,geometryChunkBytes:7_441_000,totalAssetBytes:14_430_000,rawGeometryBytes:20_684_000,
    namedJsChunkBytes:Object.freeze({
      index:275_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DuYYNNzv.js':181_800,'assets/EkORlYFN.js':203_600,'assets/mainSolid.worker-cRk-2lXm.js':190_900}),
  }),
  // Planar offset and original-knot miter construction: 20,688,980 raw bytes, 7,440,203 packed bytes and 14,433,104 total asset bytes.
  'c35045123a10a1c0f5ee9b86f698cec8b7ce52a6c18c4be19db3f5c28ab5cf1f':Object.freeze({
    version:67,geometryChunkBytes:7_441_000,totalAssetBytes:14_434_000,rawGeometryBytes:20_689_000,
    namedJsChunkBytes:Object.freeze({
      index:275_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CqzxF87S.js':182_400,'assets/DvQ2zAlz.js':203_600,'assets/mainSolid.worker-DllmwPMw.js':191_500}),
  }),
  // Orthogonal offset cleanup and clearance-qualified toolpaths: 20,715,080 raw bytes, 7,465,424 packed bytes and 14,461,266 total asset bytes.
  '802ae98e65e1b6a842bf4b042ff53dd798a1a2fd7dad7e0c34602f522f3ab587':Object.freeze({
    version:68,geometryChunkBytes:7_466_000,totalAssetBytes:14_462_000,rawGeometryBytes:20_716_000,
    namedJsChunkBytes:Object.freeze({
      index:275_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B6_CYUpO.js':203_600,'assets/lHwxfll1.js':182_600,'assets/mainSolid.worker-DX6sTLpR.js':192_100}),
  }),
  // Parallel section planes and selected bounded curves: 20,730,634 raw bytes, 7,469,106 packed bytes and 14,468,097 total asset bytes.
  '721641f1c5e8bcfcba1217da709ab51034d77f8bfa96605b8445b8ee82929e13':Object.freeze({
    version:69,geometryChunkBytes:7_470_000,totalAssetBytes:14_469_000,rawGeometryBytes:20_731_000,
    namedJsChunkBytes:Object.freeze({
      index:275_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BCyQZxJ9.js':203_600,'assets/HOgluJqC.js':182_900,'assets/mainSolid.worker-B4es6skn.js':192_700}),
  }),
  // Bounded original-curve chords and conforming mesh construction: 20,741,874 raw bytes, 7,472,368 packed bytes and 14,474,974 total asset bytes.
  'bffc6b58c4335e86454fe17e9e90b0524edac054e079fedc5cda7248bb2d7705':Object.freeze({
    version:70,geometryChunkBytes:7_473_000,totalAssetBytes:14_475_000,rawGeometryBytes:20_742_000,
    namedJsChunkBytes:Object.freeze({
      index:276_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CHID7inX.js':183_300,'assets/CgjgoDCg.js':203_600,'assets/mainSolid.worker-0WZ_d1kq.js':193_400}),
  }),
  // Shared indexed seam construction across NURBS patches: 20,756,211 raw bytes, 7,479,074 packed bytes and 14,489,231 total asset bytes.
  '95cf7a27518fbdcef0a358630595aa55235fce74704212000fbf451591f38371':Object.freeze({
    version:71,geometryChunkBytes:7_480_000,totalAssetBytes:14_490_000,rawGeometryBytes:20_757_000,
    namedJsChunkBytes:Object.freeze({
      index:276_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DTM9-KVT.js':185_800,'assets/V2cS-l_c.js':203_600,'assets/mainSolid.worker-C8jMER8p.js':194_500}),
  }),
  // Mixed bounds, conditioning and coordinate transforms: 20,765,277 raw bytes, 7,481,846 packed bytes and 14,496,460 total asset bytes.
  '7860ac57a8ac9b1ca2416f54702e1e8a4d77934ccf566fb75f413ded2f7a4571':Object.freeze({
    version:72,geometryChunkBytes:7_482_000,totalAssetBytes:14_497_000,rawGeometryBytes:20_766_000,
    namedJsChunkBytes:Object.freeze({
      index:276_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Dj4mghv9.js':186_600,'assets/fNjAlwlO.js':203_600,'assets/mainSolid.worker-aYjV-4iS.js':195_100}),
  }),
  // Sampled highlights, reflection lines, isophotes and dent zones: 20,789,601 raw bytes, 7,511,925 packed bytes and 14,529,961 total asset bytes.
  'a69c95b927ef121c760991238329c520d12cf7783fbba5d9232581d806bce624':Object.freeze({
    version:73,geometryChunkBytes:7_512_000,totalAssetBytes:14_530_000,rawGeometryBytes:20_790_000,
    namedJsChunkBytes:Object.freeze({
      index:276_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/6kPjrzxR.js':186_800,'assets/B740Sfah.js':203_600,'assets/mainSolid.worker-v8ZmYhof.js':195_700}),
  }),
  // Cloud fits for spheres, cylinders, cones, tori and planes: 20,828,726 raw bytes, 7,503,280 packed bytes and 14,524,836 total asset bytes.
  '78ca45b71ee4b8a188324b4c478165104f825ad55a7ab2f9e3d3e2cbcf201093':Object.freeze({
    version:74,geometryChunkBytes:7_512_000,totalAssetBytes:14_530_000,rawGeometryBytes:20_829_000,
    namedJsChunkBytes:Object.freeze({
      index:277_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/-XAU6Tq1.js':187_100,'assets/mainSolid.worker-CQ55bVYS.js':196_400,'assets/t10Eu7J2.js':203_600}),
  }),
  // Exact affine support and spatial curve correspondence: 20,837,198 raw bytes, 7,498,429 packed bytes and 14,523,647 total asset bytes.
  '8114795176d6a4d32e80a70211c52a1d4d897e8de7f365f1d3f05ef3286e45b2':Object.freeze({
    version:75,geometryChunkBytes:7_512_000,totalAssetBytes:14_530_000,rawGeometryBytes:20_838_000,
    namedJsChunkBytes:Object.freeze({
      index:277_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C6BXuwzn.js':203_600,'assets/DDImy5cQ.js':187_500,'assets/mainSolid.worker-hbFlDuZk.js':197_000}),
  }),
  // Exact linear simplification and terminal span extension: 20,855,130 raw bytes, 7,510,343 packed bytes and 14,539,067 total asset bytes.
  'ee07007dbc630cd634fd56a3f300abf32844d7ee7ea9a8db03fa446c35f59950':Object.freeze({
    version:76,geometryChunkBytes:7_512_000,totalAssetBytes:14_540_000,rawGeometryBytes:20_856_000,
    namedJsChunkBytes:Object.freeze({
      index:277_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B_tSDbLI.js':187_900,'assets/MdUbPQD1.js':203_600,'assets/mainSolid.worker-DPGzbpf4.js':197_600}),
  }),
  // NACA and CST section profiles and algebraic section moments: 20,881,895 raw bytes, 7,520,017 packed bytes and 14,551,930 total asset bytes.
  'd4eb215b2ad65efea3b481adf3c9da01da8f4dde81cf6baae60fefb06782722d':Object.freeze({
    version:77,geometryChunkBytes:7_521_000,totalAssetBytes:14_552_000,rawGeometryBytes:20_882_000,
    namedJsChunkBytes:Object.freeze({
      index:278_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BRk-mrAr.js':203_600,'assets/CR7mOv7w.js':188_100,'assets/mainSolid.worker-CooNpjEo.js':198_200}),
  }),
  // Signed curvature reconstruction and biarc fitting: 20,922,946 raw bytes, 7,520,297 packed bytes and 14,555,257 total asset bytes.
  '10e39fc869d216239a2d98f1e40c176b77f5961a0dfad890fca3263fc9279dae':Object.freeze({
    version:78,geometryChunkBytes:7_521_000,totalAssetBytes:14_556_000,rawGeometryBytes:20_923_000,
    namedJsChunkBytes:Object.freeze({
      index:278_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CrzFIbwB.js':203_600,'assets/DF1wllES.js':188_400,'assets/mainSolid.worker-Di4J4Ygk.js':198_900}),
  }),
  // CST fit to measured section sides: 20,934,815 raw bytes, 7,523,447 packed bytes and 14,560,925 total asset bytes.
  'ec492e21edc6492ca646cce40cfb8e8472ab4a046ae1dea7f0f4a590d0cad1fa':Object.freeze({
    version:79,geometryChunkBytes:7_524_000,totalAssetBytes:14_561_000,rawGeometryBytes:20_935_000,
    namedJsChunkBytes:Object.freeze({
      index:278_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CYMEBJs4.js':188_400,'assets/EuHkC_Yf.js':203_600,'assets/mainSolid.worker-Bk97WAXh.js':199_500}),
  }),
  // Exact structural curve and tensor surface joins: 20,938,484 raw bytes, 7,522,383 packed bytes and 14,563,067 total asset bytes.
  '621306c598cfc6e982fd42c5088cbd1f2c413b07db319ce966598b9e7d205e85':Object.freeze({
    version:80,geometryChunkBytes:7_524_000,totalAssetBytes:14_564_000,rawGeometryBytes:20_939_000,
    namedJsChunkBytes:Object.freeze({
      index:278_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Bgqs_wHz.js':188_800,'assets/C30FN_Qm.js':203_600,'assets/mainSolid.worker-xCbY7AOW.js':200_100}),
  }),
  // Exact implicit quadric surface support: 20,939,492 raw bytes, 7,525,799 packed bytes and 14,569,359 total asset bytes.
  'de104a3b9580c787d3f3ba04238d93ca13881f4c869e9fe033802b1a9e386c14':Object.freeze({
    version:81,geometryChunkBytes:7_526_000,totalAssetBytes:14_570_000,rawGeometryBytes:20_940_000,
    namedJsChunkBytes:Object.freeze({
      index:279_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CSF59WNi.js':189_000,'assets/ClJleRdY.js':203_600,'assets/mainSolid.worker-DP5Zh9uQ.js':200_800}),
  }),
  // Exact projective curve and surface transforms: 20,949,928 raw bytes, 7,526,982 packed bytes and 14,574,092 total asset bytes.
  'aa02ac7b44de1358fb36f86ea77fd20e0eb7ac0f95f0fbdff3e2e6f8f59615d1':Object.freeze({
    version:82,geometryChunkBytes:7_527_000,totalAssetBytes:14_575_000,rawGeometryBytes:20_950_000,
    namedJsChunkBytes:Object.freeze({
      index:279_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/03UbNDoJ.js':203_600,'assets/D06WvPGp.js':189_400,'assets/mainSolid.worker-ZvcpDlNM.js':201_400}),
  }),
  // Exact original-span curve fields: 20,963,422 raw bytes, 7,532,771 packed bytes and 14,583,225 total asset bytes.
  '2f27771952ef6111de7807d6879ed8a366fec214dfb450b112ba7be06ab7a2a8':Object.freeze({
    version:83,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:279_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/AnxUr0UA.js':203_600,'assets/BFzCMUOe.js':189_700,'assets/mainSolid.worker-COohGjAW.js':202_000}),
  }),
  // Exact tensor surface differential fields: 20,802,121 raw bytes, 7,441,827 packed bytes and 14,495,717 total asset bytes.
  'c3379c50569e39eb004398c23cbf130515fd7691a20d663107cb8f9d180e8cf4':Object.freeze({
    version:84,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:280_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DYEurc6w.js':203_600,'assets/RHlLccXr.js':190_000,'assets/mainSolid.worker-DFgvKvop.js':202_600}),
  }),
  // Bounded differential fields: 20,823,189 raw bytes, 7,447,371 packed bytes and 14,505,465 total asset bytes.
  '2578c8d3a50b20c38b9eb03c177381742fb35a18dd97752cf4e98063a872fc3d':Object.freeze({
    version:85,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:281_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BonCN-mY.js':203_600,'assets/C7kQLciN.js':190_300,'assets/mainSolid.worker-BUeU-GFQ.js':203_300}),
  }),
  // Torsion and surface curvature fields: 20,826,826 raw bytes, 7,447,280 packed bytes and 14,509,902 total asset bytes.
  '6e315e84e3ccefff900ac47dec038a90df9bd10f11da275e6158f34943259bc4':Object.freeze({
    version:86,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:282_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CZyayFs4.js':190_600,'assets/WeoQoQsQ.js':203_600,'assets/mainSolid.worker-ZJLX1nJ0.js':203_900}),
  }),
  // Stored surface interpolation error bounds: 20,836,556 raw bytes, 7,452,012 packed bytes and 14,518,002 total asset bytes.
  'dcaa30bd26295512adfa188df5f64c55f7e155906b3decd085ca37a925ab1477':Object.freeze({
    version:87,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:283_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C71XaJjl.js':203_600,'assets/XP2Bx2Su.js':190_900,'assets/mainSolid.worker-CqwWIXCg.js':204_500}),
  }),
  // Oriented NURBS shell signed volume: 20,848,188 raw bytes, 7,456,142 packed bytes and 14,524,801 total asset bytes.
  '3805c9ef7cb1d61be0c3cd25ed7b5cc726e7c66434e178d9a0dc7a4a3b0f0d3a':Object.freeze({
    version:88,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:283_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/6T2CwHAE.js':203_600,'assets/D3vh5Sny.js':191_000,'assets/mainSolid.worker-J4HD4yD0.js':205_200}),
  }),
  // Umbilic and principal curvature gap regions: 20,864,988 raw bytes, 7,461,196 packed bytes and 14,532,744 total asset bytes.
  '5e79815bb15ddcba169ecfb7649c18b46a1148d47ce3a15221ddeb85cd6aee76':Object.freeze({
    version:89,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:283_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CsjS3uuZ.js':203_600,'assets/DrCq49Ym.js':191_200,'assets/mainSolid.worker-B6HQ0Xa9.js':205_800}),
  }),
  // Curvature threshold contour roots and graphs: 20,884,398 raw bytes, 7,468,910 packed bytes and 14,543,597 total asset bytes.
  '794b4de8b9e34cfa5cfe67a49988df36e93f14ab26d0fb07401b8771fb842d4f':Object.freeze({
    version:90,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:284_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DJVwJb1j.js':191_300,'assets/D_keCb9x.js':203_600,'assets/mainSolid.worker-DOkEDyt1.js':206_400}),
  }),
  // Curvature contour UV polyline bounds: 20,898,980 raw bytes, 7,472,564 packed bytes and 14,550,494 total asset bytes.
  'df7846ba3658251bc1db2a7cc027235a6090255c6d5d86feadf7fb42705f497d':Object.freeze({
    version:91,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:285_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BUZwAhs0.js':191_500,'assets/C0nYg56a.js':203_600,'assets/mainSolid.worker-OahbVh1-.js':207_000}),
  }),
  // World contour measurements and chords: 20,918,495 raw bytes, 7,478,696 packed bytes and 14,560,546 total asset bytes.
  '089ee965ef68578452d7f5a115f8733a0dcdefa768c289789b65c25256e0787f':Object.freeze({
    version:92,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:285_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CW5CXCb2.js':203_600,'assets/Cy3_h8eo.js':191_900,'assets/mainSolid.worker-DUkKRVjA.js':207_700}),
  }),
  // Certified plane section events: 20,923,089 raw bytes, 7,480,558 packed bytes and 14,564,327 total asset bytes.
  '0f8f4dd90b5f9b3e545e1894c1d19cb8027d5e7add5a59a19d8966a0d36b9a60':Object.freeze({
    version:93,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:285_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B9OzjWAY.js':191_900,'assets/Bcpnwjru.js':203_600,'assets/mainSolid.worker-CtwAveLQ.js':208_300}),
  }),
  // Certified contour pair clearance: 20,934,768 raw bytes, 7,481,391 packed bytes and 14,567,990 total asset bytes.
  '98c5e3643430319c13d9de49d1ae6acdf18a7f3eed60d53d0b34039e3a7ab8ff':Object.freeze({
    version:94,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:285_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CaR2OLLW.js':203_600,'assets/Dyfq_PcY.js':192_100,'assets/mainSolid.worker-BJZxrLrd.js':208_900}),
  }),
  // Exact tensor patch compression: 20,946,118 raw bytes, 7,487,684 packed bytes and 14,577,789 total asset bytes.
  '6bbce3f10b5bd597ac6da1c49723146c2652f395c60f1d826eeb67f4c5bcdd5c':Object.freeze({
    version:95,geometryChunkBytes:7_533_000,totalAssetBytes:14_584_000,rawGeometryBytes:20_964_000,
    namedJsChunkBytes:Object.freeze({
      index:286_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Bjua-wx8.js':192_400,'assets/CCBOeZ4y.js':203_600,'assets/mainSolid.worker-BUXpErgd.js':209_500}),
  }),
  // Exact patch grid assembly: 20,967,079 raw bytes, 7,494,229 packed bytes and 14,587,431 total asset bytes.
  '889d85951c026def08d76632f27dadfbeddb779139c3e9346e96a36bf240c89d':Object.freeze({
    version:96,geometryChunkBytes:7_533_000,totalAssetBytes:14_588_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:286_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CIcVCw-R.js':203_600,'assets/Y-LGtLy9.js':192_700,'assets/mainSolid.worker-oe0saS3_.js':210_200}),
  }),
  // Exact tensor split and tiling with lossless table packing (see runtime-direct-api-b5a8a2-2026-11-26/lossless-data.json): 20,601,620 raw bytes, 7,641,383 packed bytes and 14,737,542 total asset bytes.
  'b5a8a209e718d469bb9ad71a3db54371641426d3428807d9f3b29b17ad5c9011':Object.freeze({
    version:97,geometryChunkBytes:7_642_000,totalAssetBytes:14_738_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:286_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CgArQZZs.js':203_600,'assets/D1hkBHSO.js':192_900,'assets/mainSolid.worker-RmEBSJ2M.js':210_800}),
  }),
  // Exact Mobius parameter clocks: 20,623,220 raw bytes, 7,648,215 packed bytes and 14,748,580 total asset bytes.
  'c19e11f920a65f5d7bf7cc72175a7c4c033659c7ebb15ae160dec574843b850c':Object.freeze({
    version:98,geometryChunkBytes:7_649_000,totalAssetBytes:14_749_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:287_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CIJXa2qY.js':193_300,'assets/mainSolid.worker-Dtws1DMI.js':211_400,'assets/qEiwmHHg.js':203_600}),
  }),
  // Exact piecewise span clocks: 20,630,878 raw bytes, 7,651,169 packed bytes and 14,754,244 total asset bytes.
  '214ddc89ceb8895f8ac5dbe08aded410a25a118a38184b91df9f187b09c28b5d':Object.freeze({
    version:99,geometryChunkBytes:7_652_000,totalAssetBytes:14_755_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:287_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B5wvCsdT.js':193_400,'assets/DB9w6tLr.js':203_600,'assets/mainSolid.worker-BcqANkU5.js':212_100}),
  }),
  // Exact UV contour transfer: 20,636,575 raw bytes, 7,652,464 packed bytes and 14,757,741 total asset bytes.
  '3253e8aad3f03bed8ce1a3307b9735cc25b71a0c1dc4881762a965ec56575fa9':Object.freeze({
    version:100,geometryChunkBytes:7_653_000,totalAssetBytes:14_758_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:288_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DysJ77R4.js':203_600,'assets/mainSolid.worker-C1AGqr2m.js':212_700,'assets/tg9djheB.js':193_400}),
  }),
  // Trimmed surface clocks and UV containment: 20,642,258 raw bytes, 7,654,291 packed bytes and 14,763,662 total asset bytes.
  'cb42920b4c002f040aa11ed5b7ef96553a9d248f070a6e0b5232a12513d6d4cd':Object.freeze({
    version:101,geometryChunkBytes:7_655_000,totalAssetBytes:14_764_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:288_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BtqmRRZB.js':193_800,'assets/Dd5dHnMN.js':203_600,'assets/mainSolid.worker-BDF9YI9A.js':213_300}),
  }),
  // Exact UV seam contact and splitting: 20,652,422 raw bytes, 7,658,316 packed bytes and 14,769,952 total asset bytes.
  '1edefb05b6cfc456417891b10ff39bdf9c5804bd167ce0e4907f8ea8f650d289':Object.freeze({
    version:102,geometryChunkBytes:7_659_000,totalAssetBytes:14_770_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:288_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CdSX8q9i.js':193_900,'assets/mainSolid.worker-D5x2PhEH.js':213_900,'assets/wXyTdWoU.js':203_600}),
  }),
  // Exact UV curve lift: 20,653,813 raw bytes, 7,660,619 packed bytes and 14,775,101 total asset bytes.
  '220217d64d89f97bbc4ec125f5d0e81911dc19f1251563635f32fc889714124c':Object.freeze({
    version:103,geometryChunkBytes:7_661_000,totalAssetBytes:14_776_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:289_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CblNNBs9.js':203_600,'assets/DjNc9y41.js':194_100,'assets/mainSolid.worker-UeLyalSJ.js':214_600}),
  }),
  // Piecewise UV trim transfer: 20,660,800 raw bytes, 7,661,347 packed bytes and 14,778,470 total asset bytes.
  '406d9bb164e0b2a9b79695ad1f6baa7f8e845ace1a7de83ff67ad5ebcd33d4cd':Object.freeze({
    version:104,geometryChunkBytes:7_662_000,totalAssetBytes:14_779_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:289_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C9I4BwRM.js':194_200,'assets/DzuPNIYg.js':203_600,'assets/mainSolid.worker-t6iYdAcY.js':215_200}),
  }),
  // UV edge and tangent agreement: 20,682,785 raw bytes, 7,665,372 packed bytes and 14,786,216 total asset bytes.
  '66dfc64d9f7bc9f3e9545ede35a1bf5244b0da51af55fee0fd8d4b9863b5860a':Object.freeze({
    version:105,geometryChunkBytes:7_666_000,totalAssetBytes:14_787_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:289_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CI35i86Z.js':203_600,'assets/CLqdAQlw.js':194_600,'assets/mainSolid.worker-bf5ApaUa.js':215_800}),
  }),
  // UV seam tolerance certification: 20,711,478 raw bytes, 7,670,923 packed bytes and 14,795,353 total asset bytes.
  '60834a0b85a3b328fd5ae26c8743cf327d152421179b0d02cecbd4e6bfd01d77':Object.freeze({
    version:106,geometryChunkBytes:7_671_000,totalAssetBytes:14_796_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:290_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/2XBx0lbx.js':203_600,'assets/CQ4KaLEF.js':194_800,'assets/mainSolid.worker-CNT2A3Bi.js':216_500}),
  }),
  // Affine curve and surface clocks: 20,712,613 raw bytes, 7,670,083 packed bytes and 14,796,989 total asset bytes.
  'f5056df61cdf9be5f1a13fba6c8a779878eb9d8b14d6fc2d251c224b8b2e3f7e':Object.freeze({
    version:107,geometryChunkBytes:7_671_000,totalAssetBytes:14_797_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:290_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/ClT-O_yK.js':203_600,'assets/DKHOpeX2.js':194_900,'assets/mainSolid.worker-Csp_6764.js':217_100}),
  }),
  // Seam orientation comparison: 20,715,132 raw bytes, 7,671,840 packed bytes and 14,800,423 total asset bytes.
  '4e4160aed582e5b98dc5d815b55f872e4d92b1f76645e8a88db42c1332ab4af9':Object.freeze({
    version:108,geometryChunkBytes:7_672_000,totalAssetBytes:14_801_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:290_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BuIIcSRP.js':194_900,'assets/CO2AR5dH.js':203_600,'assets/mainSolid.worker-DPSWPM13.js':217_700}),
  }),
  // Trimmed surface axis maps: 20,721,181 raw bytes, 7,675,494 packed bytes and 14,809,433 total asset bytes.
  '0f06ac16e825bd67465845eba7f4bf0ed312dc733fea74867bbc3f3d68d80428':Object.freeze({
    version:109,geometryChunkBytes:7_676_000,totalAssetBytes:14_810_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:291_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DNjhEBLk.js':203_600,'assets/kEiDFNTZ.js':195_600,'assets/mainSolid.worker-BinYSxPD.js':218_300}),
  }),
  // Active hulls and planar charts: 20,739,169 raw bytes, 7,680,366 packed bytes and 14,818,015 total asset bytes.
  '44f681389b01401fc37ad9a78820e4dbbdd5465fe63db614b78472f4fdd96916':Object.freeze({
    version:110,geometryChunkBytes:7_681_000,totalAssetBytes:14_819_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:291_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/5MkIGmkg.js':196_000,'assets/DvsbbpsE.js':203_600,'assets/mainSolid.worker-DmCR20_8.js':219_000}),
  }),
  // Planar contour projection and lift: 20,756,779 raw bytes, 7,685,126 packed bytes and 14,826,140 total asset bytes.
  'db83dbdb5b0fedc81413ef70457f6aa1b578c1a2b595fbe6d8ff93e7fc3c35cf':Object.freeze({
    version:111,geometryChunkBytes:7_686_000,totalAssetBytes:14_827_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:292_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BaerISMZ.js':203_600,'assets/DC_nte9z.js':196_300,'assets/mainSolid.worker-Bbttn9KJ.js':219_600}),
  }),
  // Signed planar face area: 20,760,386 raw bytes, 7,686,918 packed bytes and 14,830,619 total asset bytes.
  'a7b0240ef365887f5e25f3376cffaee98e3ea88848b1eaedb7a34a749e31ea82':Object.freeze({
    version:112,geometryChunkBytes:7_687_000,totalAssetBytes:14_831_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:292_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CPf_ogci.js':196_600,'assets/Dn2Ofq05.js':203_600,'assets/mainSolid.worker-BqJkA_Sw.js':220_200}),
  }),
  // Exact planar wire moments: 20,788,633 raw bytes, 7,696,830 packed bytes and 14,843,801 total asset bytes.
  '37ef935613069ba4022171842a52d94b88c9bb5ef1e013be36682e7bdb0c3ae7':Object.freeze({
    version:113,geometryChunkBytes:7_697_000,totalAssetBytes:14_844_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:292_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BR74BSFl.js':203_600,'assets/mainSolid.worker-BBRRnPXZ.js':220_900,'assets/x9fyohJu.js':196_900}),
  }),
  // World-space face moments: 20,810,075 raw bytes, 7,703,340 packed bytes and 14,852,916 total asset bytes.
  '43b439ae3f432e700d7d97164308c5c2ca4ab5aaf174e497918deab2c769bf76':Object.freeze({
    version:114,geometryChunkBytes:7_704_000,totalAssetBytes:14_853_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:292_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C9pbFN13.js':197_000,'assets/CJJ-rmsd.js':203_600,'assets/mainSolid.worker-BsukTNAd.js':221_500}),
  }),
  // Rational wire moment bounds: 20,819,120 raw bytes, 7,706,805 packed bytes and 14,859,352 total asset bytes.
  '99b96effb83cbcf17ed0404ea3ffb789b06c8bbb17295314c8df2238d8e7b956':Object.freeze({
    version:115,geometryChunkBytes:7_707_000,totalAssetBytes:14_860_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:293_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BscAHTpe.js':197_100,'assets/jbyymqEm.js':203_600,'assets/mainSolid.worker-BeLGpm2B.js':222_100}),
  }),
  // World face moment and principal axis bounds: 20,831,264 raw bytes, 7,708,625 packed bytes and 14,864,016 total asset bytes.
  'c9dafc90093cc83e06bc9970721352b4cc693daf1ba2d54df042dcfc58acff19':Object.freeze({
    version:116,geometryChunkBytes:7_709_000,totalAssetBytes:14_865_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:293_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Bo822T5I.js':203_600,'assets/C-yT7Qjx.js':197_400,'assets/mainSolid.worker-IR55ES-S.js':222_700}),
  }),
  // Planar face perimeters and distances: 20,851,828 raw bytes, 7,713,861 packed bytes and 14,872,446 total asset bytes.
  'fbbaa1784ebe4346f67dad3f5b749801db50ba857fc7aa22dca596f2b6c4868a':Object.freeze({
    version:117,geometryChunkBytes:7_714_000,totalAssetBytes:14_873_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:293_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/9pEfcFnH.js':203_600,'assets/Y1gicBFx.js':197_700,'assets/mainSolid.worker-BEVDb3y8.js':223_400}),
  }),
  // Exact trimmed face ray casting: 20,860,748 raw bytes, 7,718,103 packed bytes and 14,879,262 total asset bytes.
  '14aa880d6c0e3b8cdfe2c907a241005e91519c2cdd02ab889182fec465e248ca':Object.freeze({
    version:118,geometryChunkBytes:7_719_000,totalAssetBytes:14_880_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:293_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DLD5mSm-.js':203_600,'assets/mainSolid.worker-7SeEicbv.js':224_000,'assets/v28zsBN4.js':197_800}),
  }),
  // Coplanar ray contacts and material intervals: 20,879,850 raw bytes, 7,724,760 packed bytes and 14,888,515 total asset bytes.
  'e9e9000535395be8b8cc961c306d54e496cc9b10b5a5ee12152e3ba87565afa8':Object.freeze({
    version:119,geometryChunkBytes:7_725_000,totalAssetBytes:14_889_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:294_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BN6rSSAO.js':197_900,'assets/Bj19zhZE.js':203_600,'assets/mainSolid.worker-lKTvXIWC.js':224_600}),
  }),
  // Oblique UV line contact isolation: 20,881,669 raw bytes, 7,724,620 packed bytes and 14,890,476 total asset bytes.
  'a70cb8004a08ad944688fbcd944d13d7c19a78a886e1b5f43a4896a9c2cb6eb9':Object.freeze({
    version:120,geometryChunkBytes:7_725_000,totalAssetBytes:14_891_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:294_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C1NUUDz7.js':197_900,'assets/DmRDR6Zc.js':203_600,'assets/mainSolid.worker-DJ91MG6i.js':225_300}),
  }),
  // Exact support plane intersection: 20,888,165 raw bytes, 7,727,133 packed bytes and 14,895,560 total asset bytes.
  'da33088cb91e7a84a45cf16e68ebd0b0d5cae616209f3260d0bb6370ac7be9a0':Object.freeze({
    version:121,geometryChunkBytes:7_728_000,totalAssetBytes:14_896_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:294_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Dz2C2709.js':203_600,'assets/mainSolid.worker-BR8YnPam.js':225_900,'assets/whM7aK8L.js':198_100}),
  }),
  // Trimmed plane sections and paired segments: 20,901,486 raw bytes, 7,732,005 packed bytes and 14,903,994 total asset bytes.
  '9b29242f17aba8dfbbd9cbdf9c3e34ef2bd6df8b2ba8e14175a0136478767a63':Object.freeze({
    version:122,geometryChunkBytes:7_733_000,totalAssetBytes:14_904_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:294_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BmFKhf_T.js':198_500,'assets/CZI30JKo.js':203_600,'assets/mainSolid.worker-CqxKBOy5.js':226_500}),
  }),
  // Stored section edge rounding: 20,922,305 raw bytes, 7,741,553 packed bytes and 14,917,460 total asset bytes.
  '55ef4a80f4f00dc28e11e7b618c25cba7e9a718df9c21a87dda8d49ebfb340f6':Object.freeze({
    version:123,geometryChunkBytes:7_742_000,totalAssetBytes:14_918_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:295_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BX--Erz_.js':203_600,'assets/f7TrzCip.js':198_900,'assets/mainSolid.worker-DVmT7WIV.js':227_200}),
  }),
  // Transverse face boundary contacts: 20,947,181 raw bytes, 7,748,091 packed bytes and 14,927,898 total asset bytes.
  '81a7d49302d26557e09721f9d214d693e9bdb12374d85942b368f5b1b47f0e36':Object.freeze({
    version:124,geometryChunkBytes:7_749_000,totalAssetBytes:14_928_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:295_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Bb7TOlFe.js':203_600,'assets/Cm_WdoDk.js':199_400,'assets/mainSolid.worker-smGMWGkm.js':227_800}),
  }),
  // Face contact classification: 20,950,872 raw bytes, 7,746,964 packed bytes and 14,928,519 total asset bytes.
  'd4db9ff71ce8e55d3273a648e9c63714b0fd5e08b15ce6aeed9314efce311b0e':Object.freeze({
    version:125,geometryChunkBytes:7_749_000,totalAssetBytes:14_929_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:295_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CYD8VPXv.js':203_600,'assets/DJNMUejI.js':199_400,'assets/mainSolid.worker-BU0-tamP.js':228_500}),
  }),
  // Adaptive spatial contact resolution: 20,956,144 raw bytes, 7,748,854 packed bytes and 14,933,294 total asset bytes.
  '1c6712c632c77ac93fced0e26c4f1c6ff6c09f9b8fe210f377f27d2a7420c1a7':Object.freeze({
    version:126,geometryChunkBytes:7_749_000,totalAssetBytes:14_934_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:295_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B-Lx7UZv.js':199_500,'assets/BeXQK62M.js':203_600,'assets/mainSolid.worker-DlYqWadZ.js':229_100}),
  }),
  // Exact quadratic face contacts: 20,893,893 raw bytes, 7,730,360 packed bytes and 14,916,994 total asset bytes.
  'df7525738a44014fcb0848db6ed65b2eb7e7eccc621dfcc0eb6b5a14c465340f':Object.freeze({
    version:127,geometryChunkBytes:7_749_000,totalAssetBytes:14_934_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:295_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BYwaGJC2.js':203_600,'assets/DURUhWo9.js':199_500,'assets/mainSolid.worker-FI7KXKHa.js':229_700}),
  }),
  // Ordered and enclosed face section events: 20,921,226 raw bytes, 7,740,307 packed bytes and 14,929,754 total asset bytes.
  'e38c1425b604fc991426548572b15917bb67d69bfa0aad0085575cdf134dbcaa':Object.freeze({
    version:128,geometryChunkBytes:7_749_000,totalAssetBytes:14_934_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:296_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/ByvvtuZr.js':203_600,'assets/DZYT3s0b.js':199_600,'assets/mainSolid.worker-C_4T6d2h.js':230_400}),
  }),
  // Section material classification and measurement: 20,933,008 raw bytes, 7,745,697 packed bytes and 14,938,113 total asset bytes.
  'a2c451c48df55634ec2a0205514a41d28d5b9e291eb96243512374a420129584':Object.freeze({
    version:129,geometryChunkBytes:7_749_000,totalAssetBytes:14_939_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:296_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BWrtzMRZ.js':203_600,'assets/Cti6ThLS.js':199_900,'assets/mainSolid.worker-BwXvUugr.js':231_000}),
  }),
  // Exact section material edges: 20,940,410 raw bytes, 7,746,698 packed bytes and 14,941,531 total asset bytes.
  'f844fcb947fbf3eba3965f7654e53862c910bf8b8843cc3dae54daeef9d0a975':Object.freeze({
    version:130,geometryChunkBytes:7_749_000,totalAssetBytes:14_942_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:296_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BwSvzsxt.js':203_600,'assets/DzyKNYdc.js':200_000,'assets/mainSolid.worker-yQ_Uy1aX.js':231_600}),
  }),
  // Plane side and graph certification: 20,956,930 raw bytes, 7,751,430 packed bytes and 14,949,350 total asset bytes.
  '8b7e8d1f8b766192059ac1b394ab079c22d25d183eb0856d14c1a79abee559f7':Object.freeze({
    version:131,geometryChunkBytes:7_752_000,totalAssetBytes:14_950_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:297_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Banq7LmC.js':203_600,'assets/dxwB4eKA.js':200_200,'assets/mainSolid.worker-H-BBSPGl.js':232_300}),
  }),
  // Plane graph sampling and polyline approximation: 20,944,775 raw bytes, 7,766,760 packed bytes and 14,967,223 total asset bytes.
  '96a93d02c9c8a928ad94d7ffe7428fe1dcb243952e50c8471fb4294cbdc53138':Object.freeze({
    version:132,geometryChunkBytes:7_767_000,totalAssetBytes:14_968_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:297_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/68pM2n8P.js':203_600,'assets/DXkVZIbh.js':200_300,'assets/mainSolid.worker-C_XJuxdi.js':232_900}),
  }),
  // Stored plane section curves: 20,958,990 raw bytes, 7,769,021 packed bytes and 14,972,726 total asset bytes.
  '450fd9af458357744980721264004954b8abda0580e1c297dbaa893e787ec521':Object.freeze({
    version:133,geometryChunkBytes:7_770_000,totalAssetBytes:14_973_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:297_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DED5VaFV.js':203_600,'assets/IJZauybj.js':200_500,'assets/mainSolid.worker-DpnKh1I4.js':233_500}),
  }),
  // Complementary surface cell trims: 20,962,553 raw bytes, 7,774,747 packed bytes and 14,981,073 total asset bytes.
  'e9ad95efca8c9bbad977ca835681963c099c88963a65e68603daad6f0ea0a828':Object.freeze({
    version:134,geometryChunkBytes:7_775_000,totalAssetBytes:14_982_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:298_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BtvTh92_.js':200_800,'assets/DMkb9l4z.js':203_600,'assets/mainSolid.worker-DF_ctZOW.js':234_200}),
  }),
  // Certified multispan plane branches: 20,960,914 raw bytes, 7,769,651 packed bytes and 14,979,044 total asset bytes.
  'e57a9199eeef7b6770149ac8d691ea2506f9694731302806a6fd3fa2818bb3ae':Object.freeze({
    version:135,geometryChunkBytes:7_775_000,totalAssetBytes:14_982_000,rawGeometryBytes:20_968_000,
    namedJsChunkBytes:Object.freeze({
      index:298_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BRs-3Gph.js':200_800,'assets/BkpKQebs.js':203_600,'assets/mainSolid.worker-BZSdTn2J.js':234_800}),
  }),
  // Certified periodic section cycle: 20,970,233 raw bytes, 7,775,405 packed bytes and 14,986,799 total asset bytes.
  'd6a39cafff03aa97a22944e7d6864e1379a98b705475c5e4a762a57aee4209b7':Object.freeze({
    version:136,geometryChunkBytes:7_776_000,totalAssetBytes:14_987_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:298_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DbRo_-2H.js':200_900,'assets/DpKIqmLo.js':203_600,'assets/mainSolid.worker-K96HbQ80.js':235_400}),
  }),
  // Full periodic section trims: 20,959,615 raw bytes, 7,774,257 packed bytes and 14,987,937 total asset bytes.
  'c33cf77041cf33b83330b9014229e9bfa91b173d6e427eb4f60227d475378122':Object.freeze({
    version:137,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:298_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B0K866q3.js':203_600,'assets/BjVo8uGq.js':200_900,'assets/mainSolid.worker-Diy1NyUc.js':236_100}),
  }),
  // Plane section caps and islands: 20,898,888 raw bytes, 7,729,093 packed bytes and 14,946,346 total asset bytes.
  'c806c1fa5711c40d09f2939ee1a7216f9956ee480927350bd6c4ab2732d4bf2f':Object.freeze({
    version:138,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:298_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CgbGeSl2.js':201_200,'assets/mainSolid.worker-DApJDC8E.js':236_700,'assets/zOLxCb1Y.js':203_600}),
  }),
  // B-rep planar section sheets: 20,909,434 raw bytes, 7,732,733 packed bytes and 14,953,059 total asset bytes.
  '88139dadca6984df60fba1c930b44da66333707417655c352229d2ddadcc2027':Object.freeze({
    version:139,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:299_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B527MzAr.js':201_400,'assets/CwRQsgKH.js':203_600,'assets/mainSolid.worker-1ad5OS-F.js':237_300}),
  }),
  // B-rep section rim caps: 20,929,421 raw bytes, 7,740,076 packed bytes and 14,964,072 total asset bytes.
  'ddfc5fc8f6216ebfb813fbe25a57b1888ed0bac515a7c78399eb55537a0da7d0':Object.freeze({
    version:140,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:299_600,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/B0AYVNNO.js':201_700,'assets/CiZdQRKR.js':203_600,'assets/mainSolid.worker-CXQ8Q_6T.js':238_000}),
  }),
  // multi-plane B-rep closure: 20,934,559 raw bytes, 7,741,616 packed bytes and 14,967,366 total asset bytes.
  '7aa414fb2e2c37f66fc0e06c2aa469f946f995d6f209c988f85191e3bea4ffcb':Object.freeze({
    version:141,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:299_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C6H9OTAQ.js':203_600,'assets/ky75b6ts.js':201_700,'assets/mainSolid.worker-Bfrh-yw3.js':238_600}),
  }),
  // certified capped solid volume: 20,949,478 raw bytes, 7,748,497 packed bytes and 14,976,937 total asset bytes.
  '21a46b392b08ecb534a641b5b96de8c3804bd7bb77ec913c6d06a239a1afadf5':Object.freeze({
    version:142,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:300_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C9BqCB8S.js':201_800,'assets/DkpJwH1Z.js':203_600,'assets/mainSolid.worker-dPNTjhkm.js':239_200}),
  }),
  // inferred planar shell rims: 20,964,459 raw bytes, 7,753,320 packed bytes and 14,984,690 total asset bytes.
  'c866f6b255b9d2f3bcb93377c23f8b9c7c0eb6caa547c6b91082457ddec51a6b':Object.freeze({
    version:143,geometryChunkBytes:7_776_000,totalAssetBytes:14_988_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:300_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CUiRUBwG.js':201_900,'assets/DHPDyTaY.js':203_600,'assets/mainSolid.worker-HvLaS4DH.js':239_900}),
  }),
  // expanded planar cap supports: 20,969,261 raw bytes, 7,756,925 packed bytes and 14,991,284 total asset bytes.
  'b8f9d6d9adcc93184ea2d247ecd6d3261e404592fb23fafee9bfdea0b6c071c3':Object.freeze({
    version:144,geometryChunkBytes:7_776_000,totalAssetBytes:14_992_000,rawGeometryBytes:20_971_000,
    namedJsChunkBytes:Object.freeze({
      index:301_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C-d1L7CC.js':202_000,'assets/DNnWTTLy.js':203_600,'assets/mainSolid.worker-CMdNuvSq.js':240_500}),
  }),
  // expanded shell closures: 20,971,096 raw bytes, 7,756,379 packed bytes and 14,993,783 total asset bytes.
  'bebcf8ec37382ac844dccbcfb71a3b820d190963a006506006e472a94c7f8a8a':Object.freeze({
    version:145,geometryChunkBytes:7_776_000,totalAssetBytes:14_994_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:302_000,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CqN4XFCn.js':203_600,'assets/TNvXge7R.js':202_200,'assets/mainSolid.worker-B00bOdiJ.js':241_100}),
  }),
  // polyhedral mass properties: 20,938,995 raw bytes, 7,762,028 packed bytes and 15,003,404 total asset bytes.
  '32d9ed8786ddd10832e7cdaacb81eb9d4d05d957b5695180c47b74b0c1703399':Object.freeze({
    version:146,geometryChunkBytes:7_776_000,totalAssetBytes:15_004_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:302_800,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DJ0bAXWM.js':203_600,'assets/DXp2B85H.js':202_400,'assets/mainSolid.worker-Cp68LSFm.js':241_800}),
  }),
  // polyhedral spatial measures: 20,957,164 raw bytes, 7,771,653 packed bytes and 15,016,500 total asset bytes.
  'a96cb8ac506bc3ff9de66e505acedf9688dff96e9d71b1f9dd7a27542c22b694':Object.freeze({
    version:147,geometryChunkBytes:7_776_000,totalAssetBytes:15_017_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:303_500,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/IYeLw5YL.js':203_600,'assets/U7DkS4-L.js':202_600,'assets/mainSolid.worker-qgpTykZQ.js':242_400}),
  }),
  // transverse polyhedral sections: 20,965,913 raw bytes, 7,780,977 packed bytes and 15,028,944 total asset bytes.
  'eae5e79c4120be510ae51470de26589c241b6456abdcd4c7ea98436ddb2b0935':Object.freeze({
    version:148,geometryChunkBytes:7_781_000,totalAssetBytes:15_029_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:304_200,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CJeZ2gw-.js':202_800,'assets/Dz_fJm1p.js':203_600,'assets/mainSolid.worker-DdH1QV9O.js':243_000}),
  }),
  // rational section queries: 20,949,770 raw bytes, 7,769,749 packed bytes and 15,022,727 total asset bytes.
  'd2f24601688cd1bef22a557deb07e848cbf959f99edbd5d2c599211ffe19c41a':Object.freeze({
    version:149,geometryChunkBytes:7_781_000,totalAssetBytes:15_029_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:304_900,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/DBI58UtN.js':203_600,'assets/_8v_NiS0.js':203_000,'assets/mainSolid.worker-BUinjsrj.js':243_700}),
  }),
  // polyhedral solid queries: 20,959,359 raw bytes, 7,778,065 packed bytes and 15,034,464 total asset bytes.
  '8631c8da2f5a6cbe0bf67da8f2d7ae1cee3070ab934849714a352acccbb3fb42':Object.freeze({
    version:150,geometryChunkBytes:7_781_000,totalAssetBytes:15_035_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:305_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BBqKHSZb.js':203_600,'assets/CCmJboga.js':203_200,'assets/mainSolid.worker-90xp6beX.js':244_300}),
  }),
  // swept polyhedral clearance: 20,963,859 raw bytes, 7,775,580 packed bytes and 15,034,976 total asset bytes.
  '01bef35e3da4646c71c2c08b66489cc211a6ced2bd73163dfbb252d0b1c0ec6a':Object.freeze({
    version:151,geometryChunkBytes:7_781_000,totalAssetBytes:15_035_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:306_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/C9Lw7NtT.js':203_600,'assets/CF_NiiaL.js':203_400,'assets/DSHZm6Hd.js':100_500,'assets/mainSolid.worker-BVH5FoK_.js':244_900}),
  }),
  // exact polyhedral path contacts: 20,834,723 raw bytes, 7,710,893 packed bytes and 14,972,853 total asset bytes.
  '2c75a12169bc392a84c18d93d32c76f13898c2c07222a7025cec6de932f70be5':Object.freeze({
    version:152,geometryChunkBytes:7_781_000,totalAssetBytes:15_035_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:306_700,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/BvznK7SD.js':101_100,'assets/Bx3KUUhc.js':203_600,'assets/D_HRlRN8.js':203_500,'assets/mainSolid.worker-B8QJPZ_3.js':245_600}),
  }),
  // polyhedral path partition and length: 20,847,237 raw bytes, 7,713,364 packed bytes and 14,978,825 total asset bytes.
  '5825b74aab6827d8c1c9c788db5279519050c60c6ba46f8016873e5fb7027eac':Object.freeze({
    version:153,geometryChunkBytes:7_781_000,totalAssetBytes:15_035_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:307_300,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CLDeGQ75.js':203_600,'assets/DjJaX281.js':203_600,'assets/XuZAXGrw.js':101_800,'assets/mainSolid.worker-CegizeSI.js':246_200}),
  }),
  // polyhedral path recovery: 20,850,491 raw bytes, 7,714,022 packed bytes and 14,981,278 total asset bytes.
  'b146d46bce34cf159fdee1c6e5ceabebdbcab7ea697520986e5cdff72bcd1819':Object.freeze({
    version:154,geometryChunkBytes:7_781_000,totalAssetBytes:15_035_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:307_400,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/Be-KW1hZ.js':102_400,'assets/CdroHspr.js':203_600,'assets/DEOpc2Gl.js':203_600,'assets/mainSolid.worker-DVKJNPKp.js':246_800}),
  }),
  // assembly path coverage: 20,878,653 raw bytes, 7,723,136 packed bytes and 14,994,276 total asset bytes.
  '3964a918917087b2652c46bd21261f4b92bb36dc10e804ca05b745234b501afe':Object.freeze({
    version:155,geometryChunkBytes:7_781_000,totalAssetBytes:15_035_000,rawGeometryBytes:20_972_000,
    namedJsChunkBytes:Object.freeze({
      index:308_100,DirectModeler:498_800,geometryBuildEngine:345_500,wasmStreaming:185_800,
    }),
    exactJsChunkBytes:Object.freeze({'assets/CWK-8AGG.js':103_000,'assets/DtF_wiPe.js':204_000,'assets/mainSolid.worker-Bwta0hvJ.js':247_500,'assets/n61l1S_s.js':203_600}),
  }),
})

export function selectGeometryDeliveryBudget(wasmSha256){
  if(typeof wasmSha256!=='string'||!Object.hasOwn(profiles,wasmSha256))
    throw new Error('Geometry artifact has no reviewed delivery budget profile')
  return profiles[wasmSha256]
}
