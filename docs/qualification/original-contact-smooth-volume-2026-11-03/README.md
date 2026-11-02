# Original contact-fit C1/C2 qualification — 2026-11-03

Rust source base: `cfc7d77d59f3ebdd689d6d7ec0d847289562f92c` plus the
contact smoothness implementation and regressions in this commit. Both source
manifests guard 4,401 Rust/shader/Cargo/toolchain files. The only change after
the full NURBS snapshot was correction of expected BRep test geometry; all
NURBS proof sources and the copied full-suite executable stayed identical.

## Terminal evidence

- Full native `nurbs-core`: **2,255 passed, zero failed, one existing ignored**,
  zero filtered, 70.97 seconds, copied executable.
- Focused contact tests: **3 passed**. Includes open C1/C2; rational scale,
  affine axes/center, independent domains, curved guide and nonplanar caps;
  exact budget replay/one-less refusal; C1-only guide refusal at C2.
- Closed original atlas: periodic nonconstant scale/fit, full C2 motion and
  source-profile joins. Parameter case: 2,780 cells/13,545 exact work. Rational
  clock-change arc case: 155,978 cells/81,725 exact work. Cells/exact/products
  replay and one-less refusal pass. The public boundary API rejects zero and
  negative fit denominators despite a certified frame. Declared C0 diamond
  corners remain C0; requesting G1 refuses.
- Focused native `brep-core`: **2 passed**, 2,316 filtered, 0.38 seconds. The
  new open material body has a proven volume, C2 original filled map and
  boundary-error upper bound `7.739605515799558e-5` under budget `0.05`.
  Its cap/wall classification remains C0. Exact work replay passes; one-less
  cells/exact work and a broken original trim refuse. The earlier C1 cap with
  an actual curvature jump still refuses C2.
- `cargo check --locked --manifest-path crates/Cargo.toml -p nurbs-core
  -p brep-core --lib --no-default-features`: passed, existing warnings retained.
- Architecture audit: passed, 54 pinned oversized files, zero layer violations.
  `git diff --check`: passed.

See `result.json`, compressed terminal logs and source manifests. The proof
composition is documented in `docs/design/nurbs-original-contact-smooth-volume.md`.

## Limits

The new contact gate proves smoothness of the original normal-axis fit, not
continuous rail coincidence. Profile/cap trim binding, retained geometry,
global embedding and cap/wall joins keep their independent owners. The closed
atlas tests are not a complete closed-material BRep qualification. No full
BRep-suite, new WASM, STEP round trip, browser, CI, arbitrary full-boundary G1/G2
or completion of the broader geometry goal is claimed by this packet.
