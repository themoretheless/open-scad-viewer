# Exact atlas branch extensions — 2026-10-28

Native Rust candidate. Qualification is recorded separately; this document does
not claim arbitrary algebraic inverses, unbounded execution, full Minkowski
sum topology, a rebuilt WASM, or browser delivery.

* Rational clocks: exact rational polynomial Euclidean GCD removes common
  factors, including interior denominator zeros. A genuine pole remains a pole.
  Range, strict interior monotonicity and full homogeneous spatial composition
  are still mandatory. Identically zero denominators refuse.
* Algebraic correspondence: propose common monomial covers t=r^p, s=r^q,
  1<=p,q<=8, full composed seam degree<=64. Both sides are verified by exact
  homogeneous identity, then extended through certified triangular UV disks.
  This is a bounded candidate family, not arbitrary implicit algebraic branch
  isolation. A valid common cover and UV disk do not guarantee closed-body
  acceptance: the composed source charts must still pass the unchanged global
  source-star regularity and separation checks, which may refuse stationary clocks.
* Stationary endpoints: nonnegative derivative Bernstein coefficients, with at
  least one positive coefficient, prove strict monotonicity on the open interval.
  UV disks may use nonnegative determinant and trace plus strict interior
  determinant and a separately verified Jordan boundary during the homotopy.
  Source surface regularity does not acquire this extra UV-only relaxation.
* Authored high degree: exact triangular degree reduction is attempted when
  composition would exceed degree 16. Every elevated control is independently
  reconstructed and compared. Effective degree and predicate budgets remain.
* Multi-span pcurves: each nonempty authored knot interval is extracted exactly,
  continuity is checked, and the complete split boundary re-enters the existing
  region isotopy, winding and triangulation proof. Holes are retained. Limits:
  64 boundary pieces per wire, 128 vertices; surface knot-cell crossing and
  global conformity across independently partitioned faces remain separate work.

Legacy v4 atlas files are unchanged. Four-million predicate work and 8192-bit
arithmetic guards remain. Exceeding a bound means unproved, never acceptance.

Reproduction (native only):

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 cargo test --manifest-path crates/Cargo.toml \
  -p brep-core --lib minkowski3d:: -- --nocapture
```

`source-files.json` binds the six changed Rust files, not an independent complete
workspace snapshot. `diagnostic-before-uv-isolation.log` retains the observed
cone regression before restricting stationary-endpoint UV handling. Its failures
are diagnostic, not the final qualification result.

Final native result: **83 passed, 0 failed** (63.49 s test wall time; not a
performance benchmark). Final log: `native-tests.log`; summary: `result.json`.
The unoptimized run was deliberately interrupted after 82 passing tests while
the torus case was still running; it is not a complete qualification run.
No standalone checker replay, WASM rebuild or browser qualification was run.
