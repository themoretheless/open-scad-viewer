# Expanded checkable Rust profile certificates — 2026-10-15

Delivered geometry WASM SHA-256:
`da90493b88d40a1c052523e42b44edb033c3d009a8c1855375e7e723dd7db3cc`.
The generated module, public module and rebuilt `dist` module match. The packed
module was also independently decoded and checked by the distribution verifier.

## Implemented

- Topology verification now accepts stored planar curves of degrees 1..8,
  multi-span curves and periodic active domains. Exact homogeneous de Boor
  restriction and rational Bernstein interpolation preserve the original bits;
  the checker does not replace source definitions with fitted curves.
- A separate `represented-profile-topology-and-area` certificate independently
  verifies a rational Green-area enclosure. Decimal integer ratios preserve
  exact bounds through JSON/JavaScript. The enclosure uses an exact polynomial
  reciprocal expansion with a proved whole-interval remainder, not samples.
- A separate `represented-curve-affine-parameter-agreement` certificate proves
  coincidence on specified partial domains, including reversed traversal,
  different degrees, common homogeneous factors and multiple knot spans.
  Every common interval must satisfy both exact homogeneous polynomial identities.
- Exact agreement is now attempted before numerical duplicate agreement in the
  production Boolean and even-odd normalization paths.
- New capabilities are exposed through the geometry ABI and TypeScript
  transport. The standalone Rust checker supports topology, measure and curve
  agreement modes and rechecks WASM-produced certificates without the bridge.

## Verification

- 17 Rust certificate tests passed, including source-bit substitution, malformed
  ratios, forged bounds/scopes, omitted spans, reversed partial agreement,
  non-dyadic knot restriction, periodic closure, degree-eight separators,
  crossings, open seams and work-limit refusal.
- 40 native Bézier tests and 7 native profile bridge tests passed.
- Vue and MCP TypeScript checks passed.
- 102 host tests across 9 files passed, including four new certificate transport
  tests and the existing profile, drawing and Solid-worker regression suites.
- Direct release-WASM tests verified three measured profiles (periodic cubic,
  degree-eight boundary and rational ellipse) plus partial cubic/quadratic
  agreement. Requested area width was at most 1e-6 mm². Forged/stale proofs failed.
- Four existing release-WASM cusp/rational-profile fixtures passed certificate
  checks, extrusion and closed-mesh checks with no degenerate triangles,
  nonmanifold edges or orientation conflicts; volume error remained below 0.3%.
- The separately compiled Rust checker verified all three WASM measure records
  and the agreement record and rejected forged area bounds.
- Production `dist` passed the existing budgets: 173 artifacts, 8,072,282 asset
  bytes + 14,170,593 raw WASM bytes = 22,242,875 bytes. No budgets were increased.
- A fresh in-app browser tab imported `ellipse-solid.json` from this WASM,
  displayed its retained sketch/body, and displayed the ordinary mesh with
  smooth B-rep preview disabled. No warnings/errors were captured. See
  `browser-mesh.png` and `browser-console.json`. Import latency is not qualified.

## Explicit remaining limits

This extends the certificate domains, not every modeling operation. Production
Boolean/extrusion carrier admission still has its own degree limits. General
conic/cubic Euclidean offsets are not added here. The agreement certificate
checks the supplied affine parameter correspondence; arbitrary nonlinear or
projective reparameterizations are not covered by that certificate.

The topology checker remains sufficient and conservative: external component
contacts, unavoidable extra eliminant roots and exhausted work budgets can
remain unproved. It never treats that refusal as a proof of invalidity or absence
of intersections. General Boolean construction history, tessellation accuracy
and arbitrary 3D solid validity remain outside these certificates' scopes.
The existing `geometryStatus` is not relabelled as globally certified.

No Git commit or push was performed for this change. Parallel work was preserved.
`source.sha256` records the inspected source files. The design and API contracts
are documented in `docs/design/bezier-profile-certificates.md`.
