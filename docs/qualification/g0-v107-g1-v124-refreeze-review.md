# G0 v107 / G1 v124 runtime v12 kernel refresh

Preserve OwnRust86, G0 v1-v106, G1 v1 through v123, their generators and all earlier records byte for byte.

Why: after G0 v106 bound kernel `15596aa8…`, Rust changes landed on main (C13 in cad-step and the geometry-bridge STEP scene, S13, C14). A reproducible build of current sources is now `fba78e35f862c3259769f5d7a2458b6f060a61a104c5e7b94e3b13ec2d5da837`, so the packaged artifact again had no qualified manifest or delivery budget.

What: rebuild `public/wasm/geometry-kernel.wasm` reproducibly with `scripts/build-geometry-kernels.mjs`. Record the finite exact-artifact direct-API baseline (115/115 Node tests, 40/40 Chromium main and 40/40 independent-worker checks) in `runtime-direct-api-fba78e-2026-11-15/` and publish it unchanged with `scripts/publish-runtime-direct-api.mjs`. Freeze runtime v11 at the `15596aa8…` record and bind `own-rust-node-v12` / `brep-closed-v12` to the new record. Add the v12 delivery budget profile.

How: archive the v106 generator as `scripts/refresh-qualification-fingerprints-v106-v123.mjs`, append the v106 outputs to the frozen archive registry, then run `node scripts/refresh-qualification-fingerprints.mjs --write --recorded-at 2026-11-15 --reason …`.

Start semantic-manifold-g1-candidate-run-v124 with zero completed clean runs and zero completed work units. All 4740 clean work units remain mandatory, no earlier result is imported, and all matrix rows, environments, seeds, comparator mutations and budgets are preserved. This amendment executes no qualification row.

qualificationClaim remains none and qualificationApproval remains not-approved. G0 closure and production cutover are not claimed. Historical evidence remains discovery-only and byte-immutable.
