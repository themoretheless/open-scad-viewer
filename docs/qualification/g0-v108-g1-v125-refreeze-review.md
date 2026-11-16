# G0 v108 / G1 v125 runtime v13 kernel refresh

Preserve OwnRust86, G0 v1-v107, G1 v1 through v124, their generators and all earlier records byte for byte.

Why: after G0 v107 bound kernel `fba78e35…`, the integration batch landed Rust changes (N2 coplanar merge, S1 monotone triangulation, S2 kernel jobs, N3 knot joints, N4 Rush full round, N1 exact analytic tangency, regression fixes, the boundary-aligned and boundary-anchored face workplane in mesh-topology, and ruled loft section correspondence by winding and nearest start vertex) together with the current main branch. A reproducible build of current sources is now `74d4272a11be3b4c190dac1fa1007b132f00c94fe23e5b44316cd248a4e3f08f`, so the packaged artifact again had no qualified manifest or delivery budget.

What: rebuild `public/wasm/geometry-kernel.wasm` with `scripts/build-geometry-kernels.mjs`. Record the finite exact-artifact direct-API baseline (115/115 Node tests, 40/40 Chromium main and 40/40 independent-worker checks) in `runtime-direct-api-74d427-2026-11-16/` and publish it unchanged with `scripts/publish-runtime-direct-api.mjs`. Freeze runtime v12 at the `fba78e35…` record and bind `own-rust-node-v13` / `brep-closed-v13` to the new record. Add the v13 delivery budget profile.

How: archive the v107 generator as `scripts/refresh-qualification-fingerprints-v107-v124.mjs`, append the v107 outputs to the frozen archive registry, then run `node scripts/refresh-qualification-fingerprints.mjs --write --recorded-at 2026-11-16 --reason …`.

Start semantic-manifold-g1-candidate-run-v125 with zero completed clean runs and zero completed work units. All 4740 clean work units remain mandatory, no earlier result is imported, and all matrix rows, environments, seeds, comparator mutations and budgets are preserved. This amendment executes no qualification row.

qualificationClaim remains none and qualificationApproval remains not-approved. G0 closure and production cutover are not claimed. Historical evidence remains discovery-only and byte-immutable.
