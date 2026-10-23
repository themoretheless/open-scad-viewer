# Holed source atlases and nonlinear seam transports

Native qualification complete: **64 tests passed**, including 9 new regressions.
A separately built Rust checker accepted **12 new and archived proofs** and
rejected **44 forged certificates**. The focused run passed 21 tests.

Native source certification only; WASM, browser and dist were not rebuilt or
qualified in this milestone. No full Minkowski-sum topology claim is added.

New sufficient classes in Rust:

- Multiple holes and curved boundaries: a deterministic rational advancing-front
  generator proposes only indices. Separate exact checks prove wire isotopy, containment,
  embedded UV disks and complete boundary cancellation. The disk winding sum
  equals the original 0/1 material region, retaining the holes.
- Higher-order poles: exact substitutions x=v^r, y=u^a*v^r, including reflected
  angular coordinates. Whole homogeneous power identities and positive target
  weights are checked; singular simplices are never silently omitted.
- Arbitrary monotone rational clocks on straight seams: segment images agree
  throughout the homotopy; time-dependent edge homeomorphisms extend over disks.
- Nonprojective rational clocks on curved graph seams: an exact fractional-linear
  spatial observable recovers a parameter, the full three-coordinate composition
  identity is checked, and its rational edge clock is extended over an embedded
  triangular parameter map before the source atlas homotopy runs.

The old successful pointwise/projective recipes are attempted first. Every
transported chart still passes the original local-star, Jordan-link and global
separation checks. Certificates bind the full source expression and actual PL
endpoint. A standalone source proof does not imply full Minkowski-sum topology.

Enclosure v4 admission is frozen separately in atlas_legacy. Its production
recipe comes from the four source files pinned by the 2026-10-20 qualification;
only test modules are omitted. The new standalone source-atlas classes do not
silently add or change serialized receipts inside old enclosure certificates.

Limits remain: no universal inverse for every rational space-curve carrier;
non-monomial poles, unresolved multiple knot-cell trim composition, degenerate
or non-embedded trims and parameter extensions that fail sufficient Jacobian
predicates still refuse. Arithmetic, degree and work budgets remain finite.

Validation receipts: [manifest](manifest.json), [replay](native-replay.json),
[native tests](native-tests.log), [proof scope](proof-scope.md).
The frozen source ledger is source-files.json; all 11 owned Rust files matched
the tested snapshot at delivery.

Checker SHA256: `e28280d22d650dc3895c60fde70cb72e914d6da08af51d103b1f3418eb05495b`.
