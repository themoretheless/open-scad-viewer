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
})

export function selectGeometryDeliveryBudget(wasmSha256){
  if(typeof wasmSha256!=='string'||!Object.hasOwn(profiles,wasmSha256))
    throw new Error('Geometry artifact has no reviewed delivery budget profile')
  return profiles[wasmSha256]
}
