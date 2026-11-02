# Exact original contact coincidences — 2026-11-03

Source base `79efd1963` plus this commit. The manifest guards 4,406 original
Rust/shader/Cargo/toolchain files and identifies the actual native binaries.
The full NURBS run used a copied executable; no geometry source changed after
its compilation. Public consumer and no-codec checks ran the same source.

## Native behavior

A new optional Rust counterexample search refutes original filled-map injection
from exact original-knot coincidences, before the expensive atlas pair search.
It covers a planar path with a same-basis constant axis-normal guide, parameter
clock, zero twist and a common clamped uniform-weight bilinear carrier. Scale,
affine axes, center and reference anchors retain their original rational laws.
The carrier may be nonplanar. Both normal signs and all three axes are covered.

The actual public material counterexample now refuses with
`ideal-volume-exact-filled-map-coincidence` at **741 total cells / 91,610 exact
work / 64 products**, including all material/trace/source work. Previously it
ended at the million-cell limit. The low-level geometric witness uses 9 cells /
7,276 exact work. Its original-volume factory totals are 677 cells / 14,238
exact work. Unequal rational law weights and an interior reference parameter
use 8,859 primitive exact work / 687 whole cells / 15,821 whole exact work.
The exact original reference value `27/1280` differs from its binary64 cache;
only the original curve participates in the witness.

The primitive preserves the existing 4096-bit guard and caps itself at 100,000
cells/exact work, debiting the caller's remaining aggregate budget. Exact
replay succeeds; one-less cells/work and zero budgets decline. A near miss,
a one-ULP non-axial carrier and identified S1 endpoints never manufacture a
negative witness or positive embedding credit. A declined search proves
nothing; positive admission still requires the complete original atlas.

## Terminal qualification

- Full default native `nurbs-core`: **2,258 passed, zero failed**, one existing
  ignored, no filtered cases; 66.44 seconds.
- Focused new native checks: **3 passed**, 2,256 filtered; seven exact witnesses,
  all axes/signs, raw reference clocks/unequal weights, budgets and negative controls.
- The same 3 checks without default features/codec: **3 passed**, 2,051 filtered.
- Public BRep contract: **2 passed**, no filtered/ignored cases; 28.07 seconds.
  All eight positive contact-material configurations retain exactly the previous
  measured geometry work records. Invalid material roles and one-ULP trace
  mismatch still refuse. The real self-intersection reaches the new exact gate.
- `cargo check --locked --manifest-path crates/Cargo.toml -p nurbs-core
  -p brep-core --no-default-features --lib`: passed with warnings.
- Architecture and staged whitespace checks pass.

`verify_native_witnesses.py` independently recomposes all seven native witnesses
using exact Fractions and unchanged original source definitions. It verifies
48 whole material-span polynomial identities, actual interior membership for
six material collisions, and the nonbinary interior-anchor case as a filled-map
witness. No Rust report flag supplies authority. The dependency oracle hash is
recorded. Decompress `focused-native.log.gz` and pass it to the script to replay.

See `docs/design/nurbs-original-product-corners-and-exact-coincidences.md` for
the source formula, negative-only ownership and the proof that a regular
product-map cap/wall corner remains transverse C0, even with curved caps.

## Remaining boundaries

This is a bounded exact negative family, not a complete intersection census.
The native reason refutes the full filled carrier; membership in a trimmed
subset remains a separate obligation. The rank-loss control still reaches
`ideal-volume-work-limit`. No positive guarantee follows from failing to find
a point witness. No full BRep suite, new WASM, STEP, browser or CI qualification
is claimed. General source-mode qualification and the full thread goal remain open.
