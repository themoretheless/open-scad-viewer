# Diagnostic runs

These development runs are retained for traceability, not final timing claims.
Compilers and GPU checks were active during some measurements.

- `before.json` and `early-classification.json`: original sequential RNG corpus;
  exposed three open boundaries at translated coordinates and an invalid fill
  triangle in a retraced contour. Exact failing geometries are covered by
  `crates/planar-geometry/tests/stroke_robustness.rs` (two capsules and the
  retraced triangle regression), with the expanded corpus covering more cases.
- `fixed-initial.json`, `adaptive-axis-initial.json`: 240 independently seeded
  cases with the first numerical fixes; no oracle failures.
- `stress-1024-initial.json`: stricter scales exposed a defect in the test oracle:
  strict triangle interiors counted shared triangle edges as uncovered when
  binary64 translation rounded sample points onto those edges. Final oracle
  uses standard top-left ownership, assigning an internal edge exactly once.
  Production geometry was not changed to hide these test failures.

The final 1024-case corpus is `../stress-1024.json`. All random seeds, width,
coordinates and the distance oracle are reproducible from the saved harness.

`baseline-1024.json` uses the final top-left oracle and the pre-fix kernel: 192 cases return errors. The same final corpus passes all 1024 cases after the numerical fixes. `gpu-no-captures.json` is an excluded GPU attempt with zero captures; final GPU evidence uses the successful retry.
