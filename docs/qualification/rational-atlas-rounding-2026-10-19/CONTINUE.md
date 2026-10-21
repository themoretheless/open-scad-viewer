# Current work, 2026-10-19

User asks to finish arbitrary-NURBS topology proof and remaining mesh-rounding
cases. Do not claim universal completion: generic rectangular rational atlases
are implemented; general curved trims, poles and nonlinear curved-seam clocks
still refuse. No agents authorized. Preserve all unrelated shared changes.

New Rust code: curved/atlas.rs + atlas/bernstein.rs in minkowski3d. Vertex-disk
Jacobian positivity (diagonal dominance OR determinant+trace), Jordan star links,
pointwise rational seams, global nonincident swept hull separation. Standalone
source certificate, enclosure version 4 only when the rational atlas is needed.
TS transport/protocol updated. Native tests include a nonanalytic rational torus
with genus 1, 128 triangles, 64 vertices, 7290 disjoint pairs; rational box;
forgery/schema/endpoint binding checks; Bernstein restriction/derivative tests.

Rounding: coplanar in-plane SAT, exact positive-Bernstein subdivision, continuous
time slabs, absolute hulls FIRST and common-origin hulls second, squared area
for rotating triangles, no double charge per pair. Last regression uses an exact
2^-80 gap: moving-origin bounds must not discard an existing absolute witness.

IMPORTANT compatibility audit found source polynomial atlas used the same
rounder to choose canonical n=1/2/4/8 segments. Its legacy v3 source recipe is
now frozen in rounding_legacy.rs copied from the verified df2 frozen source;
curved/shear.rs uses that backend. Display uses enhanced rounding. This preserves
v3 source canonical counts. Native test distinguishes old/new admission.

Candidate 030c344748d38880e829a10e7949199bf47f359359f91a774737549a12482d45
(12,284,160 bytes) passed 46 native tests, 4 WASM certificates +16 forgeries,
4 native replays +16 forgeries, 2 app calls/JSON proof roundtrips,17 candidate
host/protocol tests,5 current UI component tests,types. BUT superseded after the
compatibility audit. Preserve it as test input, not final delivery evidence.
- wasm /private/tmp/osv-rational-atlas-kernel-030c3447.wasm
- checker /private/tmp/osv-rational-atlas-checker-precompat-8713149b
- old records diagnostics/030c-runtime/
- current top-level wasm-results.json / wasm-binding-results.json still contain
  those producer certificates, intentionally reused as untrusted replay inputs.

Final source snapshot: /private/tmp/osv-rational-atlas-source-20261019
- crates/target symlink to shared main crates/target for WASM cache.
- Separate optimized native target /private/tmp/osv-rational-atlas-native-target-20261019
- Frozen source ledger currently updated for compatibility and last rounding fix.
- Never run builder from main cwd! All new binary builds must use frozen cwd.

Active executions (functions.exec exec_command sessions):
- 45022 final WASM build -> final-compatible-wasm-build.log
- 55940 final native 47 tests -> final-compatible-native-tests.log
- 78278 native checker build queued/possibly pre-last-edit; must rerun build
  after final test build and copy executable to final standalone checker path.
Earlier sessions 66953/19952 were pre-last-absolute-gap changes; not final evidence.
Last rounding change adds absolute-first bounds and removes double per-pair charge; total final tests = 47.
Final native command: cargo test --locked --manifest-path crates/Cargo.toml -p
brep-core --lib minkowski3d --target-dir=/private/tmp/osv-rational-atlas-native-target-20261019
--config profile.dev.opt-level=1 --config profile.dev.debug=0 -- --nocapture
Final builder: node scripts/build-geometry-kernels.mjs
--staging-root=/private/tmp/osv-rational-atlas-source-20261019 (frozen cwd).

Next:
1. Finish native47 and final WASM, source ledger drift check. Build final native
   example verify_profile_certificate with same target/profile, copy to
   /private/tmp/osv-rational-atlas-checker-20261019, record new sha.
2. Native replay old new-source requests using check-rational-atlas-native.py;
   input wasm-results.json -> final-standalone-results.json; binding file ->
   final-standalone-binding-results.json; legacy-v3-inputs.json -> legacy-v3-results.json
   (8 old v3 positive certificates +19 forgeries; ignore old refusal because it
   is intentionally fixed by new display rounder).
3. Run final WASM replay (in frozen cwd, Node26):
   node --import tsx scripts/check-rational-atlas-replay.mts
   public/wasm/geometry-kernel.wasm
   MAIN/docs/qualification/rational-atlas-rounding-2026-10-19/wasm-results.json
   MAIN/docs/qualification/rational-atlas-rounding-2026-10-19/wasm-binding-results.json
   MAIN/docs/qualification/rational-atlas-rounding-2026-10-19/legacy-v3-inputs.json
   MAIN/docs/qualification/rational-atlas-rounding-2026-10-19/final-wasm-results.json
   This verifies/recomputes 4 current +8 archived certificates and35 total
   forgeries, reproduces source-box and2 sum producer certificates. Certificate
   producer artifact030c remains explicitly recorded; final artifact actually
   verifies every request. Verify itself recomputes the complete proof.
4. Final host: node --import tsx --loader ./tests/fixtures/raw-import-loader.mjs
   scripts/check-rational-atlas-host.mts MAIN/.../fixtures MAIN/.../final-host-results.json
   (frozen cwd). Rerun the3 tests files (17 tests) there, output final-host-candidate-tests.log.
5. Update README/build receipt/manifest/delivery status to FINAL artifact,47
   native tests, final replay counts. README currently still describes030c.
   Match final host single-cap certificate to replay sum case.
6. Verify owned main files match frozen; no other native drift. Record shared
   kernel observation. Do NOT promote common public/generated/dist/runtime
   evidence (peer preparing v5=6744 separately). No push requested this turn.

Math primary citation in proof-scope.md: Edwards–Kirby Cor1.4 verified from paper;
joint local-flat track argument is written explicitly. Do not claim proof-assistant
formalization. Native geometry all Rust; TS transports/tests only.

Memory used MEMORY.md:1515 (Rust geometry ownership), rollout
01a10c8f-377c-7f42-84ca-305bdfaedc89. Final must append exact oai-mem-citation block.

## Latest state after compatibility changes
- Final native 47 tests PASSED: final-compatible-native-tests.log.
- Final checker build3632 FINISHED, copied to /private/tmp/osv-rational-atlas-checker-20261019.
  SHA de11083ee3b44717e7cdb229788223c93fe99e941c2f18e84d025c7ff3953181.
- Active final WASM build45022 -> final-compatible-wasm-build.log (geometry-wasm compiling).
- Active native replay83003 -> final-standalone-results.json (4 current proofs+12 forgeries).
- Native bindings76667 DONE: final-standalone-binding-results.json (4 forgeries rejected).
- Active native legacy76274 -> legacy-v3-results.json (8 old proofs+19 forgeries).
- Legacy code comparisons BOTH true in legacy-recipe-check.json: shear unchanged
  except backend alias; minkowski3d.rs exact primitives unchanged except module declaration.
- Final replay helper now exists and is copied into frozen scripts:
  check-rational-atlas-replay.mts; use it after final WASM packaging, see next steps above.
- Final source has absolute-first bounds, then common moving origin, single work
  charge per slab; this preserves old acceptance/work accounting too.
- No further Rust edits are planned. Final README/manifests still need updating.

## Scoped milestone finished
Final WASM70d2945dbde77b1fc223eda4a1e1e9c84558a0cc5c5c4e60f95bb410a65d9e48,12294925bytes.
Final tests47native;12WASM+native certificates and35forgeries;2app transactions;17candidatehost/protocol;5currentUI;typespassed.
Final checker de11083ee3b44717e7cdb229788223c93fe99e941c2f18e84d025c7ff3953181.
Source drift0. Shared runtime/dist/evidence untouched by this task.
Generic curved trims/poles/nonlinear curved-seam clocks remain explicitly unproved.
README/manifest/build receipt/delivery status finalized. No further Rust edits or builds needed for this scoped milestone.
