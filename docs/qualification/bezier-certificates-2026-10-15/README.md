# Bézier representations and checkable Rust profile certificates

For the published scoped branch and its rebuilt artifact, see
[scoped push qualification](scoped-push-qualification.md).
The original measurements below concern the combined working checkout.

Scope: exact topology of a represented positive-weight planar profile.
Construction equivalence, reported numerical area and 3D solid certification
are separate claims. See [the proof contract](../../design/bezier-profile-certificates.md).

Implemented:

- Retained polynomial quadratics, general rational quadratics and rational
  cubics; affine ellipses preserve their original rational representation.
- Outward rational Green integration with a Simpson remainder.
- Fraction-free exact common homogeneous factor cancellation before the
  production implicit root finder.
- Portable source-bit-bound certificates with independent exact-rational
  checking of closure, injectivity, every curve pair, orientation and nesting.
- Algebraic checking of same-side curved cusp joints whose control hulls overlap.
- Separate Rust CLI, WASM generate/verify operations, and TypeScript transport.
- Explicit refusal for missing proofs, wrong scope, stale source, malformed
  witnesses, unknown schema and exhausted budgets.

## Evidence

The four `*-verify.json` files can be passed directly to the standalone Rust
checker. They were generated in WASM and independently verified by the native
checker; the verification result and rejection of a forged orientation are in
`standalone-rust-verifier.json`.

`raw-proof-records.json` records the source, portable certificate, verification
response, area estimate and actual extruded-mesh checks for:

1. Same-side nonlinear cusp (area 12.6).
2. Its affine coordinate change (area 25.2).
3. A nonprimitive rational cubic retaining weights 3,4,5,6 (area 50/3).
4. Retained rational ellipse (area approximately 24π).

All four extrusions have closed meshes at 32 divisions, zero degenerate
triangles, zero non-manifold edges and zero orientation conflicts; mesh volume
relative error is below 0.3%. Mesh validation is not the profile certificate.

Raw WASM SHA-256:
`97403c6148d890d6d098a259f24c407c63cefe927e93584f0e93f12bd95345ea`.

Native checks: all 104 predicate tests pass, including the independent degree-9
fixtures and the new nonprimitive rational carrier. Profile/regression and
packaged release results are recorded in the accompanying qualification logs.
`source.sha256` freezes the relevant source definitions.

Final qualification:

| Check | Result |
| --- | --- |
| Native Bézier profiles | 40/40 |
| Independent Rust certificate tests | 7/7 |
| Profile transforms | 2/2 |
| Native profile ABI | 6/6 |
| Packaged host tests, eight suites | 98/98 |
| Corrected compression bootstrap, three suites | 26/26 |
| WASM-to-standalone-Rust proof transfer | Four accepted; four forged orientations rejected |
| Vue/MCP type checks | Pass |
| Production build and distribution verification | Pass; 173 artifacts |
| Browser | Imported/saved ellipse restored and displayed; real mesh, ~120 FPS, console [] |

Optimized installed WASM: 11,077,233 bytes, SHA-256
`2e7bc66d4bf26b4f5101ca872d665db6fdd48a30ea7f84bd8e4689d0dbc7547b`.
The JavaScript geometry payload is 3,767,373 bytes (budget 3,970,000);
entry is 234,227 bytes. Size budgets were not changed.
The streaming/public/generated/distribution WASM bytes match.
Native and WASM Binaryen 116 optimizers produced identical bytes.

The first distribution check exposed a stale embedded compression bootstrap.
Publishing the matching bootstrap from the same build and rebuilding fixed it;
the final distribution verification passes. This is recorded separately from
the already-passing geometry checks.

The broader native filter happened to include unrelated spatial sweep
qualification. Eighty tests finished successfully; its one remaining closed
spatial RMF mode matrix was explicitly stopped after the focused suites passed.
No full workspace or full sweep-matrix pass is claimed.

The first browser import tab became unavailable. A fresh tab recovered and
displayed the saved ellipse successfully; import latency is not qualified.
The screenshot and `browser-qualification.json` record the inspected mesh with
smooth B-rep preview disabled.

## Explicit limits

Certificates are sufficient and conservative. A valid profile may remain
unproved when distinct components touch, an eliminant has extra roots, or a
resource bound is reached. Such a result is a refusal, never a certificate of
invalidity or an empty intersection.

Numerical Boolean/normalization history and 3D bodies remain outside this
certificate's scope. The existing profile `geometryStatus` is not relabelled
as a global certified status. General-conic/cubic offsets and degrees above
three are not claimed.
