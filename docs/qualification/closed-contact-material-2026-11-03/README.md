# Closed contact material qualification — 2026-11-03

Production source base: `3df7cdca1` (original contact-fit C1/C2 composition).
The new public native contract and exact fixture qualify the original closed
material owner on the full matrix below. The source manifest guards 4,403
Rust/shader/Cargo/toolchain files; source hashes were rechecked after terminal
runs. Binary hashes identify the copied default executable, no-codec test and
the existing material regression executable.

## Positive source guarantees

Each case issues `OriginalClassifiedClosedMaterial` only after a fresh valid
outer/hole UV region, exact complete normalized profile/trim identities,
whole filled-map regularity, global embedding, preserved parameter topology,
C2 periodic motion and G2 of every curved profile join. Assertions check all
binding records, original source addresses, valid UV audit, winding `+1/-1`,
opaque-owner source references and original input preservation. There are
48 bound spans across the eight base cases. The carrier is a reference
filled section of the closed source, not a physical end cap.

| Clock | Carrier | Holes | Cells | Exact work | Products |
|---|---|---:|---:|---:|---:|
| parameter | planar | 0 | 3498 | 45166 | 52 |
| parameter | planar | 1 | 3582 | 89756 | 104 |
| parameter | nonplanar | 0 | 3466 | 53349 | 52 |
| parameter | nonplanar | 1 | 3518 | 106773 | 104 |
| arc | planar | 0 | 169105 | 113409 | 52 |
| arc | planar | 1 | 169189 | 157999 | 104 |
| arc | nonplanar | 0 | 169076 | 121592 | 52 |
| arc | nonplanar | 1 | 169128 | 175016 | 104 |

Scale and normal-center laws vary periodically. Affine axes are nonuniform.
The arc cases use an exact rational change of source clock with unequal raw
endpoint velocities. The nonplanar section is the exact graph
`X=oldY*oldZ/16, Y=-oldZ, Z=oldY`. Rational degree-four XYZ profiles are
algebraic lifts of quadratic UV curves; no sampled fit supplies geometry.
Original XYZ/UV clocks retain their distinct domains. Exact cell/exact-work/
product/step replay passes; one-less allowances refuse.

## Terminal tests and independent controls

- Default native public contract: 2 passed, no failures, no skips/filters,
  36.63 seconds. Includes all eight positive configurations and four invalid
  source packets, work limits and a one-ULP UV binding perturbation.
- Native without default features/codec: the same 2 passed, no failures,
  36.47 seconds; all eight geometry work records equal the default run.
- Existing closed-material native regressions: 10 passed, 2,308 filtered,
  35.29 seconds. Covers authored/guided nonplanar sources, material holes,
  profile classes and alternative exact boundary clocks.
- Architecture audit and staged whitespace check pass.

`exact_witness.py` independently consumes original native JSON definitions.
Fraction polynomial cross-products prove each whole profile/UV/carrier
identity for both exported packets (16 spans). Two one-ULP coefficient
controls fail. The derived rank control also retains all eight exact source
bindings but has an identically zero carrier v derivative along u=1.

For the actual figure-eight contact source, exact de Boor evaluation and the
original laws prove that `(u,v,t)=(2,4,0)` and `(256/133,4,1/2)` both map to
`(0,0,3/256)`. Both UV points lie strictly inside the outer rational ring and
outside its hole. This is a material-volume collision, not merely a repeated
centerline position. The witness is exact rational arithmetic, not a sampled
positive embedding argument. Reproduce it by decompressing `native-default.log.gz`
and passing that log path to `python3 exact_witness.py`.

## Refusal and qualification boundaries

Wrong hole orientation and a hole outside the outer loop (while still inside
the carrier rectangle) refuse at the independent UV region gate. The rank-loss
and genuinely self-intersecting sources retain valid UV/trace proofs but their
native volume search ends with `ideal-volume-work-limit` at the shared million-
cell ceiling. They issue no volume/material token. Native diagnostics therefore
remain *unproved due to work*, not a claimed exact intersection classifier;
the separate Fraction witness proves the counterexamples geometrically.

These are original source guarantees. This packet does not qualify a new
retained Solid, mesh, STEP round trip, WASM/browser/CI state or general physical
cap/wall G1/G2. Existing explicit C0 source-join behavior remains covered by the
native regressions. The broader geometry goal remains open.
