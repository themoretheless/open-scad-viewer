# Variable annular end transitions

Native prototype built from `bd2ffba81`, 2026-10-07.

## Change

`partial_annular_variable_arc` authors a closed annular topology with three
independently sized angular spans: zero → first radius, first → second radius,
second radius → zero. Radii vary by cubic smoothstep in rational angular
parameter. Equal radii retain the existing constant middle patch.
The existing `partial_annular_arc` uses the same closing assembly and keeps its
original parameters. No tolerance welding is introduced: complete shared
boundary definitions must match before assembly succeeds.

Supported construction: positive fitting radii, a strictly retained inner wall,
three consistently directed nonzero sweeps each at most pi/2. This is a native
construction prototype, not a general source-body fillet edit.

## Evidence

- Full `circular_blend::tests::` regression: 32 passed, 0 failed, 148.68 s
  (native test execution; compilation excluded). Other integration binaries
  selected zero tests under this module filter.
- Closure regression covers 0.5 → 1.25, 1.25 → 0.5 and 1 → 1 mm in both
  directions. Each result has one body, no boundary edges and two collapsed
  pole edges. Each non-pole edge has exactly two oppositely oriented uses.
- Invalid/nonfinite radii, mixed directions, zero/oversized spans and contact
  overlap with the inner wall are refused.
- Four unequal-radius specimens were exported through native STEP v9. Each
  has 31 faces, one body and no boundary edges.
- All twelve original support patches received whole-patch section-radius
  certificates. Entry/exit bound is at most 8.24417943427623e-7 mm against
  1e-6 mm tolerance. Middle bound is at most 1.584700718875821e-12 mm against
  1e-9 mm tolerance. These bounds include original coefficient rounding.

`fixtures/manifest.json` records parameters, bounds and uncompressed artifact
hashes. Model definitions and STEP specimens are reproducibly gzip-compressed
with mtime zero. Logs record focused closure and fixture generation.

## Reproduction

```sh
cargo +nightly-2026-09-16 test --locked --manifest-path crates/Cargo.toml \
  -p brep-core --lib circular_blend::tests:: -- --nocapture
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-transitions -- /tmp/variable-annular-fixtures
```

## Open qualification

Combinatorial closure does not certify geometric solid validity. The radius
law is a section-radius law; a varying section does not automatically satisfy
the rolling-ball envelope relation. Pole G1, full seam tangency, global
thickness and absence of self-intersections remain separate requirements.

No independent OCCT STEP measurement is claimed: the earlier temporary OCP
environment is absent; inspected available Python environments do not contain
OCP. No new WASM, worker or UI package was built for this native prototype.
The previous 255 unconfirmed annular wall pairs are not reduced by this work.
Named-part acceptance, P0 and P2–P3 remain open.
