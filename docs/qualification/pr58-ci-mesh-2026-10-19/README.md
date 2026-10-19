# PR 58 CI and tessellation corrections

Base: `f7d85f4e20987ffac702d7b2654e3eec0e133228`.

The rectangular Rust mesher now assigns a collapsed edge to its authored grid
row or column. Previously, a vertical pole edge collapsed interior rows and
created sixteen non-manifold mesh edges at four subdivisions. The regression
covers three implicit spheres and six fibered bodies at 2, 4 and 8 subdivisions,
including oblique and descending frames, multiple motion pieces and a cavity.
Chromium repeats the native construction and mesh checks and retessellates
each model after STEP import. All boundary-edge, non-manifold-edge, winding
conflict and degenerate-triangle counts must be zero.

Current face-injectivity reports can use an earlier projective proposal than
the historical client ledger. Rust now replays such reports on the original
surface under the remaining shared budget; the client only transports this
request and its boolean result. Only the complete fresh replay establishes a
current witness; tampering tests cover counts, enclosures, projections and
interior source coefficients. Historical fixture validation remains supported. Integral binary64 coefficients and their JSON integer
representations compare numerically, including signed zero.

The existing guided arc-chord correction is backported independently: an
outward scalar curvature enclosure can finish above its refinement proposal
target while retaining its actual conservative error bound. No tolerance,
proof-work allowance or delivery size limit is increased.

The contact preview regression expects the first accepted native refinement
level. The RMF regression distinguishes raw C1 jets from exact forward tangent
directions under arc-length reparameterization. A speed-only weight change can
have a certified geometric join; reversed directions and exhausted work still
refuse. Source geometry and admission remain Rust-owned.

`verification.json` records the final check results and raw/packed artifact
identities. `source-inputs.json` binds the Rust inputs, toolchain, build scripts
and affected protocol/tests. The first source snapshot and failed signed-zero
fixture probes are retained as compressed attempts, not successful evidence.

This receipt does not qualify the full semantic G1 matrix, arbitrary global
geometry, analytic mesh-deviation bounds or the complete UI lifecycle matrix.

Reproduce the browser mesh checks after building the kernels:

```sh
node scripts/qualify-source-strata-tessellation.mjs \
  docs/qualification/pr58-ci-mesh-2026-10-19/cases.json \
  docs/qualification/pr58-ci-mesh-2026-10-19/browser.json
```
