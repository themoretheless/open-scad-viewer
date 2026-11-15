# G0 v106 / G1 v123 runtime v11 reconciliation

Preserve OwnRust86, G0 v1-v105, G1 v1 through v122, their generators and all earlier records byte for byte. G0 v102 had been edited in place after it was frozen; its archived bytes (sha256 `cc38768a…`) are restored rather than re-recorded.

Why: the committed geometry kernel, the runtime evidence in `src/core/currentRuntimeGeometryEvidence.ts` and the qualified manifests had drifted apart (kernel `4b2a7987…`, evidence `744be66c…` without any record, G0 v105 binding `a27ee279…`), so no qualified manifest matched the packaged artifact. Sources were also moved from `src/services` into `src/kernel`, `src/language` and other domain directories, leaving most bound paths missing.

What: rebuild `public/wasm/geometry-kernel.wasm` reproducibly from current sources (`scripts/build-geometry-kernels.mjs`), giving `15596aa8bd7c26888ae18fd1b0bcea49c36af76ef50b1a5558a6e164c142e938`. Run the finite exact-artifact direct-API baseline (115/115 Node tests, 40/40 Chromium main and 40/40 independent-worker checks) into `runtime-direct-api-15596a-2026-11-15/` and publish it with `scripts/publish-runtime-direct-api.mjs`. Freeze the runtime v10 identity as the `a27ee279…` record bound by G0 v105 and bind runtime manifests `own-rust-node-v11` / `brep-closed-v11` to the new record. Rebind bound paths that git records as renamed since 876f0d7d5 under their existing binding ids (`scripts/qualificationPathRenamesV106.mjs`); paths retired without a rename are dropped as before. Add the v11 delivery budget profile for the new artifact.

How: `node scripts/refresh-qualification-fingerprints.mjs --write --recorded-at 2026-11-15 --reason …`, after archiving the v105 generator as `scripts/refresh-qualification-fingerprints-v105-v122.mjs` and appending the v105 outputs to the frozen archive registry.

Start semantic-manifold-g1-candidate-run-v123 with zero completed clean runs and zero completed work units. All 4740 clean work units remain mandatory, no earlier result is imported, and all matrix rows, environments, seeds, comparator mutations and budgets are preserved. This amendment executes no qualification row.

qualificationClaim remains none and qualificationApproval remains not-approved. G0 closure and production cutover are not claimed. Historical evidence remains discovery-only and byte-immutable.
