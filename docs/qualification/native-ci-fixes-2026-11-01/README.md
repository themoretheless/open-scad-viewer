# Native CI fixes: partial exact-artifact checkpoint

The new geometry WASM is `442d2703d243fe29ed8434dcbce04e7cf06c2be36be149af07b51b00e45160f6`, 15900166 bytes. Generated and public copies match. Previous v7 artifacts and its qualification record are preserved.

[receipt.json](receipt.json) records 385 passing geometry-bridge tests, zero failures and one existing ignored test. Direct calls on the new WASM passed the whole-wall fixture and a box STEP round-trip preserving all 34 identity sets, vertex-ID/world-point bindings and edge-ID/vertex-ID bindings. Vertex ordering changes legitimately during STEP import; array-position equality is not the identity contract.

Two scalar sweeps passed native STEP round-trips and independent OCCT import/re-export checks for valid solids, six faces, volume and sampled surfaces. The local independent oracle was OCP 7.7.2.0; this does not claim execution of the CI oracle version. Complete STEP files, manifest and OCCT results are in `scalar-step-roundtrips.tar.gz`; compressed probe and build/test logs are adjacent.

At the original checkpoint, the actual packaged candidate failed runtime admission because no qualified manifest matched its new fingerprint. That refusal result is preserved. It never inherits historical v7 qualification. The original checkpoint remains explicitly partial and unqualified; subsequent finite API evidence and its separate runtime baseline are recorded below. No G0/G1 or universal CAD claim is made.

## Direct API matrix

[direct-api-matrix.json](direct-api-matrix.json) adds 112 passing direct WASM API checks across ten files: NURBS primitives and surface distance, mesh topology and diagnostics, B-rep construction/intersections, curved-graph Boolean and STEP. The full Vitest report is preserved as `direct-api-vitest.json.gz`.

The surface-distance file combined API and worker imports, so runtime admission blocked even its standalone mathematical tests. It is now split into API and worker files. All five original test bodies are preserved exactly; the three direct API cases passed, while the two worker cases remained mandatory for the subsequent admission stage. No test, admission rule or numerical tolerance was disabled to obtain this direct-API result.

## Chromium API probes

[chromium-direct-api.json](chromium-direct-api.json) records 28 passing checks on the main thread and 28 in a qualification worker, with no page errors, in Chromium 151.0.7922.34. Both use the exact candidate fingerprint above. The main thread exercises asynchronous packaged-kernel initialization; the worker receives a verified compiled module through the existing host bootstrap.

Run `node tools/browser-qualification/candidate-direct-api.mjs NEW_OUTPUT_DIRECTORY` to repeat these finite checks. Ellipse equations, three Boolean volumes, STEP vertex bindings, manifold/component counts and a surface-distance enclosure are checked. This development-module probe does not modify provider admission and does not qualify production chunk packaging, the application worker protocol or the whole UI.

## Runtime v8 API baseline

`runtime-api-baseline.json` binds the completed direct API, Chromium, native-bridge and STEP evidence to this exact fingerprint. The separate `../runtime-provider-442d2703d243-2026-11-01.json` qualifies that finite API baseline for runtime v8. It explicitly does not qualify the full frontend/UI, application worker protocol, production packaging, Linux or hosted CI. Those checks continue independently; historical v7 manifests remain bound to their original artifact and dependency lock.

[post-admission-checks.json](post-admission-checks.json) records twelve passing follow-up checks, including both preserved surface-distance handler tests, actual-byte admission and rejection of mismatching artifacts. Archive-isolation checks retain the original v6/v7 records while v8 evidence changes. These finite checks do not substitute for the full frontend and worker matrix.

## Local production build

[distribution.json](distribution.json) records all 202 artifacts and their hashes after the local production build and complete distribution verifier passed. Geometry bytes measure 5360256 and total assets measure 10068403, within the unchanged v7 limits of 5370000 and 10080000. The v8 profile is pinned to the new artifact SHA; unknown artifacts remain refused. The three streaming WASMs total 19180880 bytes. This result does not claim hosted CI or full application qualification.

## Source and packaging checks

[metadata-checks.json](metadata-checks.json) retains 40 passing fingerprint/manifest checks and 50 passing Node checks with no skips, including native Binaryen comparison. The full-tree metadata fixtures use a scoped 120-second harness deadline after three measured 42-55-second runs exceeded the inherited 30 seconds. Source, archive, race and exclusive-publication assertions are retained; native work limits and application worker deadlines are unchanged.

OwnRust82 / G0v96 / G1v113 append current identities while preserving predecessors. They import no clean qualification result. All 4740 clean G1 work units remain incomplete. The complete frontend and native-library runs continue separately.
