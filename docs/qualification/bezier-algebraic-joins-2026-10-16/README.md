# Algebraic Bézier correspondence, contact census and offset joins — 2026-10-16

This qualification expands three bounded geometric domains. Construction,
source binding and independently checkable certificate verification remain in
Rust. TypeScript transports requests and proof data only.

## Implemented behavior

- Bevel, miter and round joins for signed one-sided normal offsets of connected
  retained Bézier/NURBS curves and internal source knots. Exact source cells
  retain the previous whole-interval pointwise bound. Joins add exact rational
  normal/tangent boxes, a miter extension bound, or checked minor-arc coverage
  and a chord/sagitta enclosure. The complete target joined wire has a checked
  Hausdorff error bound, including closed seams and output rounding. No source
  poles are fitted or replaced to manufacture a certificate.
- Nonlinear coincidence without a supplied parameter formula. A polynomial
  relation H(s,t) divides both homogeneous coordinate equations; exact boundary
  signs and monotone projections prove complete one-to-one image coverage.
  The tested t=s^(2/3) correspondence is algebraic and non-rational, beyond
  affine, Möbius and explicitly supplied rational maps. Original curves are
  retained in Boolean duplicate admission and even-odd cancellation.
- Complete isolated contact/self-intersection certificates, including multiple
  roots. An exact resultant and every projected fiber are independently checked
  by exact division and Sturm root counts. Self diagonal t-s is removed by a
  checked identity; stationary diagonal events remain distinct from crossings.
  Common positive homogeneous factors are canceled only through exact quotients.
- Production normalization applies the exact eighth-degree self-event census to
  the original parent before stationary trimming. Existing native searches run
  first where already adequate; the complete exact census supplies a fallback.
  High-degree endpoint-only certificates add sentinel 65534 and can be used by
  profile topology checks without relaxing other-wire manifold restrictions.

## Scope and explicit remaining limits

This is not a universal proof of arbitrary CAD geometry. Nonlinear image proofs
currently require one retained active span per supplied interval and a common
monotone projection; unpartitioned multi-span/non-monotone images may refuse.
The isolated-event producer currently requires dyadic parameter witnesses within
40 isolation levels and exact binary64 transport. Non-dyadic/algebraic event
parameters, nonisolated fibers, vanishing eliminants and exhausted exact-arithmetic
budgets can remain unproved. Existing intersection algorithms remain available.
An injective self proof does not claim a complete stationary-jet census.

Joined-offset certificates prove the specified bevel/miter/minor-arc target
wire, not filled-region Minkowski erosion/dilation, clipping, offset injectivity
or validity of touching compounds. The existing line/circle filled-region API
is preserved. Singular normals and excessive/parallel miters explicitly refuse.
Output is labelled `bounded_approximation`; neither mesh tessellation nor 3D
solid validity is silently relabelled as formally certified.

These are independently checkable exact-arithmetic Rust certificates under the
trusted compiler/runtime/arithmetic libraries, not proof-assistant verification
or third-party certification of the whole CAD kernel.

## Evidence

The fixtures cover a fourth-order tangency, a cubic/eighth-degree loop with
parameters 1/4 and 3/4, an exact rotated cusp with a stationary diagonal at 1/2,
a nonprimitive rational loop, rational/reversed/partial nonlinear coincidence,
false correspondence branches, signed joins, internal weighted corners, closed
seams and forged/omitted proof claims. Runtime records and standalone checker
results accompany this report. Test counts and delivered hashes are recorded
in `manifest.json` after final packaging.

The direct WASM runtime also models three closed outward-offset styles and both
eighth-degree loop components, checking mesh closure, zero degenerate triangles,
zero nonmanifold edges, zero orientation conflicts and volume discrepancy below
0.3%. This is a fixture qualification, not a mesh self-intersection proof or a
formal tessellation-error certificate.

## Delivered checks and package

- 91 selected native Rust tests passed: 39 certificate tests, 40 existing Bézier
  profile regressions, two new algebraic arrangement tests, three higher-degree
  modeling tests and seven bridge tests. Filtered-out tests are not counted.
- 114 host tests passed across 11 files; Vue and MCP type checks passed.
- The final release-WASM smoke verified the new nonlinear relation, fourth-order
  contact, eighth-degree loop, nonprimitive rational loop, cusp diagonal, general
  quartic endpoint proof and all three join styles. Zero-offset singleton output
  retains its represented point. Five actual extrusion meshes passed the stated
  closure/defect/volume fixture checks. Previous measure, topology and degree-eight
  follow-up release-WASM fixtures passed again on this same module.
- A separately compiled Rust checker accepted nine WASM-produced certificates
  and rejected nine forged/omitted claims. See `standalone-checker.json`.
- A fresh in-app browser tab imported `round-offset.solid.json`, displayed its
  sketch and body with smooth B-rep preview disabled, and captured no page
  warnings/errors. See `browser-round-offset.png` and `browser-console.json`.
  This verifies this imported mesh fixture; no universal browser, UI-authoring,
  latency or tessellation-error claim is made.

`public`, generated module and `dist` contain identical 11,327,907-byte geometry
WASM, SHA-256 `7ca38df94ebfa12ef5bcc9a1f29bec3aa1ccffb426d55f3d9bc392eb58018b47`.
The production build and packed/raw identity checks passed: 173 artifacts,
8,132,398 asset bytes + 14,356,188 raw WASM bytes = 22,488,586 total bytes.
Assets grew 37,590 bytes relative to the preceding qualified delivery. The
aggregate budget was explicitly increased from 8,100,000 to 8,140,000 bytes,
leaving 7,602 bytes; individual module limits remain unchanged.

No commit or push was performed. Parallel checkout work was preserved.
`source-sha256.json` records the inspected task-related source snapshot; the
shared checkout also contains other ongoing work. API and mathematical contracts
are documented in `docs/design/bezier-profile-certificates.md`.
