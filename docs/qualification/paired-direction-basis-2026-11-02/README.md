# Paired-volume coordinate proposals from normalized directions

Base: `8f23db236`.

The paired cap coordinate fallback normalizes original point-jet directions
before proposing their inverse, preserving physical lengths for the subsequent
metric scaling. Literal coefficient-direction proposals keep priority, and
unpaired source paths retain their previous fallback. The rounded proposal
supplies no geometry authority: its determinant and all transformed original
coefficient jets still require full outward verification. No proof predicate,
source surface, work limit or tolerance was changed.

Native evidence (nightly-2026-09-16, locked Cargo sources, test opt-level=1,
test/dev debug info disabled, two build/test threads):

- Cargo `-p nurbs-core --lib analysis::affine_volume -- --nocapture`: **28 passed**.
- Rotating original carriers: **259 cells / 12 exact work**, previously 3928
  cells, with measured-budget replays and both one-less refusals preserved.
  This is an operation-count result for that case, not a broad performance claim.
- Same freshly built NURBS executable without filter, `--test-threads=2`:
  **2227 passed / 0 failed / 1 ignored**, 105.74 seconds.
- Cargo `-p brep-core --lib rational_hollow_cap_quarter_turn_frame -- --nocapture`:
  **0 passed / 1 failed**, 148.92 seconds. Complete actual boundary equality
  passes at 241560 work; actual filled volume still exhausts the unchanged
  99800 source cells within its 100000 aggregate budget, and no Solid is admitted.

Thus full BRep and the complete original loft/sweep/G1/G2 objective remain OPEN.
No WASM, STEP, browser, CI or TypeScript geometry changes are claimed.
Next: apply the existing exact tensor-weight cancellation owner to the original
base carrier of paired caps, retaining general independent rational end caps.
