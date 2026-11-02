# Original material-bound rational hollow cap volumes: native qualification

Implementation: `b7a0a28304fdfaf3e0b612267898c27ebb5926a3`. All **6,597 Rust/Cargo/toolchain inputs remained byte-identical** throughout compilation and all four qualification commands. A post-commit recheck matched the same source manifest.

- New rational hollow matrix: **2 named tests, 8 distinct cases, 0 failures**.
- Complete capped-material family: **7 named tests, 18 spatial cases, 0 failures**.
- Full nurbs-core library: **2068 passed, 0 failed, 1 ignored**.
- Full brep-core library: **2075 passed, 0 failed, 5 ignored**.

`check_matrix.py` independently matches the complete 18-key Cartesian case set, rejects missing/duplicated keys and records measured work in `matrix-cases.json`. The eight new cases have a real hole and nonproportional, nonuniform positive rational cap weights. Both reference caps have different interiors and different raw UV clocks, but their complete outer/hole traces bind exactly to the same original world profiles under normalized clocks. The common material region and the globally embedded original filled map are proved together by the opaque Rust owner. Source profile/cap pointers, complete span bindings, winding, declared boundary classes and original input immutability are checked.

| Frame | Clock | Affine laws | Cells | Exact work |
|---|---|---|---:|---:|
| authored | parameter | unit | 18925 | 375910 |
| authored | parameter | varying rational | 20028 | 376084 |
| guide | parameter | unit | 51854 | 375910 |
| authored | arc | unit | 29219 | 381213 |
| authored | arc | varying rational | 30172 | 381387 |
| guide | parameter | varying rational | 54012 | 376084 |
| guide | arc | unit | 102952 | 378206 |
| guide | arc | varying rational | 106408 | 378380 |

Every case freshly admits the source, replays all measured shared limits exactly, and refuses with one less exact-work unit. Existing tests separately cover the other shared limits, mismatched material regions, incomplete bindings and closed-traversal refusal. All budgets remain unchanged. Both new matrix tests originally exhausted the one-million-cell ledger; the same authored inputs now pass after restricting the ORIGINAL Bernstein bounds. No point samples, rounded replacement surfaces or retained Solid authorize this proof.

The full suites include the new scalar/tensor restriction regressions, normalized derivative-unit checks, original cap-difference jets, and fixed-metric UV refinement with exact/one-less budget replay. The original-interior-fold regression still refuses even when the retained boundary alone forms a valid Solid.

`THEOREM.md` describes the outward algebra and proof boundaries. `result.json` contains exact commands, exit codes and complete test summaries. Full logs and both source manifests are losslessly compressed; `SHA256SUMS` binds every receipt. Compiler and test settings are recorded in `environment.json`. Ignored tests retain their existing explicit opt-in qualification status. Whitespace checks passed.

This packet closes the new native rational-weight × hole × frame × clock × affine matrix. The cap/wall join remains explicitly **C0**; smooth volume charts do not imply physical G1/G2 there. It does not qualify general miter/loft source binding, singular/polar modes, arbitrary independent image clocks, packaged WASM, browser behavior, independent STEP round-trip or remote CI. The broader thread goal remains open. Earlier qualification packets are unchanged.
