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

The fixed WASM rebuild and scripts/check-cad-partial-preview-wasm.mts must
complete before runtime is qualified. Browser UI mode, complete intersection
and transition proofs, and document commit acceptance remain open.
