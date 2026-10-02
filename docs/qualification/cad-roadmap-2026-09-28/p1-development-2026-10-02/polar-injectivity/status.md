# Global polar projection for face injectivity

2026-10-02. Native Rust, packaged WASM, full worker handler and production
browser qualification.

The new sufficient condition projects the original surface to
`F=(atan2(y,x),sqrt(x²+y²)-z)` in an affine coordinate frame. A strictly
positive interval bound on `x` establishes a continuous angle branch.
The whole natural parameter rectangle contributes to one common Jacobian
hull. A constant inverse of its midpoint matrix gives the bound
`||I-Y*DF||_inf < 1`, so integrating along a straight parameter segment
proves global injectivity. Independently successful local cells do not suffice.
Disconnected internal knots are rejected by surface validation.

For homogeneous coordinates `(X,Y,Z,W)`, set `Q=X²+Y²`. The derivative
numerators are evaluated as correlated tensor Bernstein polynomials:

- angle: `(X*Y' - Y*X') / Q`;
- radial coordinate: `(W*Q' - 2*Q*W') / (2*W²*sqrt(Q))`;
- vertical coordinate: `(Z'*W - Z*W') / W²`.

All polynomial coefficients, products, divisions and square roots have
outward interval bounds. Frames are only guesses from a source quadratic
iso-curve; their quality never admits geometry. The proof uses every original
surface control and weight. Explicit frames support degrees up to eight.
Periodic charts, missing coverage, insufficient denominator separation and
failed contraction return unresolved.

The annular fixture's constant torus face (index 5) is now proven with
contraction upper bound **0.3237207671906416**, 256 polar cells and 193 prior
projection cells. Total face work is 601 cells. **25 of 27 faces** are proven;
the two collapsed transition tips remain explicitly unresolved. The combined
report still states `absenceProven=false`, including incomplete pair coverage.

Validation includes original source immutability; positive and negative arcs
at three radii/angles; folded-control rejection; partial budgets; rational
derivative bounds against an independent formula; and a rotated, translated
annular model. The bridge exposes the successful affine frame. TypeScript
validates it against source-derived coefficients and exact work counts, and
rejects forged pole proofs, changed frames, invalid coefficients and aggregates.

This establishes within-face absence for the tested regular torus patches.
It does not qualify pole quotient topology, end G1 continuity, distinct-face
contacts, wall thickness, general fillets or the complete volume body.

## Packaged runtime and browser evidence

`wasm/report.json` checks six actual WASM cases (base and rotated/translated
model, budgets 4096/600/448), plus two complete worker-handler cases. Source
models remain unchanged. The packaged module is 9,704,339 bytes, SHA256
`d09aaae3b44f8aa46a34075922269bf7ec9520dc72845891e2766585b0d81ef4`.

Chrome Canary 157 production checks pass with mouse and keyboard. The UI
shows unresolved faces **1, 11** at budget 4096, and **1, 6, 11** at 448.
Neither reports absence of intersections. Exported documents before and
after both diagnostic runs are equal. The same browser scenarios exercise
Retry, cancellation, worker restart, selection/import changes and stale
responses on the existing contact fixtures. Browser errors are empty.
The keyboard screenshot was inspected; it renders using WebGPU. This does
not establish large-scene performance.

Sixteen focused TypeScript/actual-WASM protocol tests pass. Production build
and packed/source WASM identity checks pass: 140 artifacts, 7,257,258 asset
bytes and 11,760,459 raw WASM bytes. The polar proof adds 6,444 packed kernel
bytes and the complete source/protocol change adds 8,676 asset bytes versus
the preceding production build. Budget margins remain 397 bytes for the
packed geometry chunk and 1,042 bytes for distribution assets.
