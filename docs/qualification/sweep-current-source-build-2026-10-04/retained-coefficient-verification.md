# Native retained coefficient integration

Qualified source graph: `/private/tmp/open-scad-viewer-sweep-current-source-2026-10-04-1791102401`, derived from the frozen qualified 9a3 graph with the native retained-wall coefficient operation added. This is not a claim of whole dirty-primary source equivalence.

New optimized WASM: `b5b1293aa06d0301bb69bdc8896ba15360379030dcbe7291ad59963f052a8cf1`, 10,731,712 bytes. Fresh compilation/optimization/packaging passed; generated/public binaries and packed verification agree. Compiler inputs and raw hash are recorded in `retained-coefficient-build-proof.json`.

- Three native/JSON tests passed in both publication and derived source graphs.
- 17 integration tests passed in the derived graph; 18 passed in primary (its additional test is separate).
- 30 additional integration regressions passed in the derived graph.
- Type checking and Vite build passed.
- The new adapter and four-file artifact tuple were installed in primary after guards confirmed the previous 9a3 artifact, unchanged adapter/test files and matching native module.
- Ten reconstructed STEP cases passed independent OpenCascade topology and analytic-volume verification, including the three conic weights and closed/frame/guide modes.
- 33 baseline STEP cases passed independent OpenCascade verification; all 43 STEP cases now passed for b5.
- The 74-case wide/narrow browser matrix passed all 852 assertions on b5; exact artifact/source hashes are bound in `retained-coefficient-ui-matrix.json`. Prior 9a3 results remain historical.

The exact coefficient predicate has moved from TypeScript into Rust. Section segmentation and the remaining complete retained-family orchestration are still pending native migration. Universal embedding and arbitrary rational join completeness remain unproved; the native operation reports no embedding certificate. PR #30 head 85318706 CI remains pending. A runnable native example was locally verified in the publication worktree and awaits publication.

## Native family migration checkpoint

Local publication commit `b72ab215` adds segmented authored Bezier control extraction, complete retained coefficient-family verification and a runnable native example. Six focused native/JSON regressions, 332 NURBS unit tests, two integration tests and one doctest passed. New adapter type checking passed. The derived frozen source `/private/tmp/open-scad-viewer-sweep-current-source-2026-10-04-1791103438` is building its WASM (`/tmp/sweep-native-family-wasm-build.log`); the adapter remains isolated pending real-WASM tests. The previous published head 85318706 has passed Rust, official OpenSCAD and STEP CI; Node and native smoke jobs still run. The family commit is local pending the next publication checkpoint.

## CI failure and append-only refresh in progress

Published head 85318706: Rust, official OpenSCAD and STEP jobs passed. Node 22 completed with six failures in `g0ToolchainFingerprints.test.ts`; 438 other test files passed. The failed premises are the stale transport.rs frozen digest and an unrecorded rebuilt WASM. A fresh publication-graph WASM is building, separate from the larger frozen integration graph. Own-Rust v31 and G0 v39 / G1 v56 are being prepared append-only, with old archives preserved and clean-work counters remaining zero. CI will upload exact geometry kernel bytes and identity for observed Ubuntu-host variant verification. No gate was removed.

## Family integration and source-record repair

Native family WASM `3b5a90bca57e82cff21fde1334872d722efafb3385f37256dba38a0059fa21b1` (10,737,139 bytes) passed 47 integration tests on derived source graph 1791103438. Primary installation was stopped before mutation: another task had installed artifact `34a4e5be7bd2b9fb9b4156adb42da4c30400e444d50331103594b12d8b27e0e6`; it does not expose the segmented-control operation. Its artifact was preserved, and the family adapter remains isolated.

The publication graph independently built artifact `9ec9bf63a27f431673c847eb7b1bbe0bf59781cea24a413520a48b1e14428c33` (9,902,585 bytes). Direct native JSON calls through that WASM confirmed segmented controls and typed family refusal. Append-only own-Rust v31 / G0 v39 / G1 v56 records were generated. G0/G1 tests: 23 passed; read-only drift audit matched; the drift regression passed. All earlier archives were preserved and clean-work counters stayed zero. Node 20 CI failed with the same six old-record errors as Node 22; repair is being pushed with exact kernel/identity CI artifact upload enabled for subsequent observed-host verification.

The native family and append-only registry repair were pushed as PR #30 head `45d7f6a55e2f66e07b287de5c11ab157311bd022`; fresh CI is running. A stable current-primary source graph (3082 verified inputs) was frozen at `/private/tmp/open-scad-viewer-sweep-current-source-2026-10-04-1791104606` and is rebuilding with the tested family adapter while retaining concurrent new geometry features. Build log: `/tmp/sweep-family-current-primary-build.log`. No previous integration artifact was installed over the concurrently updated primary kernel.

## Current source and observed-host checkpoint

Current-primary frozen native graph 1791104606 passed 905 NURBS, 693 BRep and 23 predicate unit tests (1621 total; two BRep measurements ignored). Its freshly built WASM 9098cf836a40257babab160a920a87c4fe7806be14af2fe03bbb8c61bf00de5f (10,737,116 bytes) passed 48 integration tests, including unchanged coordinates with altered ring partition. Its installation was stopped before mutation when primary acquired production changes to exact_curve_segments and contour, plus additional test changes. Those active changes and artifact 34a were preserved.

Both downloaded Ubuntu CI binaries for publication head 45d7f6a5 match 3ca78b534b4ba38a171cde120dd2ec0c676e36c0b32f36e31d89b1b0c6ccfd26 (9,902,781 bytes), with exact matching identity files. This differs from the observed local 9ec artifact. Own-Rust v32 / G0 v40 / G1 v57 append-only records now include that observed variant; they do not infer any other host or carry clean results forward. Registry tests passed locally. This metadata-only repair is awaiting publication.

Observed-host repair was pushed as `7adb7b34` in PR #30. Exact downloaded Ubuntu binary was admitted in an isolated registry preparation with source and archived digest checks intact; clean-work counters stayed zero (`/tmp/sweep-observed-ci-admission-test.log`). New-head CI is running. Installation of 9098 remains withheld because active primary production code gained unclamped exact blossom decomposition and contour support after the snapshot; no old artifact or adapter was copied over those changes.
