# Scoped push qualification, 2026-10-15

Branch: codex/bezier-certificates-20261015, based on 029e4c121.
The branch retains the Bézier/profile certificate/cusp mesh changes and their
evidence, without copying other active uncommitted feature work.
A pre-existing private basis-module reference in surface_blend.rs was fixed
through its existing public function.

The earlier README and browser evidence describe the combined working checkout.
This branch has its own rebuilt WASM and fresh qualification below; the earlier
binary hash is historical, not the published branch artifact.

## Fresh checks

- Exact predicates: 104 passed.
- Native Bézier profiles: 40 passed.
- Independent Rust certificate checks: 7 passed.
- Profile transforms: 2 passed.
- Packaged host tests: 98 passed across eight suites.
- Raw and optimized WASM smoke: all four profiles certify and produce closed
  extruded meshes with zero degenerate triangles, non-manifold edges and
  orientation conflicts. Forged/stale proofs are rejected.
- Vue and MCP type checks: passed.
- Whitespace checks: passed.

Published geometry WASM: 10948377 bytes, SHA-256

`1cbfb8d1f998eb18236605b9ca437ebb3120ba53d720496f313b8ae513c31224`

The generated and public bytes match. Native Binaryen 116 uses the standard
optimization flags. Release package size settings are retained; no distribution
size budget is raised. No additional full-workspace/sweep-matrix/browser
qualification is claimed for this scoped projection.

Scope remains represented profile topology. Construction history, numerical
area and arbitrary three-dimensional solid validity are not certified.
