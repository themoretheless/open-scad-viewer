export default {
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

}
