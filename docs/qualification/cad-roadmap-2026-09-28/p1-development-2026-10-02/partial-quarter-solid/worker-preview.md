# Partial annular worker preview

The worker protocol has a distinct partialAnnularPreview job returning a
display body and preview evidence, rather than a replacement document.
Qualification remains preview-only with commitAllowed=false and unresolved
boundary/transition proofs. Source body identity must match the expected
identity. Reply validation checks finite mesh coordinates and bounded indices.

Two Vitest protocol regressions pass: invalid qualification, identity and mesh
replies are refused; a late response after cancellation and replacement is
ignored. vue-tsc --noEmit passes. The display service leaves the source body
unchanged and retains the explicit qualification alongside its display mesh.

The actual WASM runtime check initially failed at segments=12: aggregate
tessellation reached 20018 triangles after face 20, over the 20000 limit.
Constant-height cylinder UV rails were represented as degree-3 curves,
preventing rectangular-face tessellation. They now use exact linear UV rails
when all radius-law controls agree. The native bridge regression now passes
at segments=12 without raising the triangle budget. All 17 circular-blend
native tests pass. Error messages include face and triangle-count context.

The edge-fillet panel now offers Partial arc preview, with explicit incomplete
qualification text and disabled command application. Its UI regression test
verifies job submission for the selected edge, unchanged document when Apply
is invoked, and a late worker response ignored after Escape. This test holds
the worker response; it does not prove actual browser display geometry.

The fixed WASM rebuild and scripts/check-cad-partial-preview-wasm.mts pass.
Eight actual WASM results cover four arcs in the original and rotated frame:
body identity, source immutability, explicit preview qualification, finite
indexed display mesh, and worker reply validation. Each display has 3857
vertices and 7714 triangles. WASM SHA-256 is recorded in partial-preview-wasm/report.json.
Production Vite build passes; verify-dist checks 140 artifacts. Asset bytes
increase by 24841 to 7248560; budgets retain the previous measured headroom.

Actual browser display and input checks,
complete intersection and transition proofs, and document commit acceptance
remain open.
