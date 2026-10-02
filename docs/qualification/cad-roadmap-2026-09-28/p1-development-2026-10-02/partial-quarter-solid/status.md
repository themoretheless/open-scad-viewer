# Partial quarter-rim prototype

The native constructor produces a closed 27-face annular body with smoothstep
entry and exit spans and a constant-radius middle span. Fourteen circular-blend
unit tests pass. Native STEP imports as one valid OCCT solid in each orientation.

Reproduce export and bounded boundary diagnostics:

```sh
cargo run --offline --locked --manifest-path crates/Cargo.toml -p brep-core --example partial-annular-quarter -- docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/partial-quarter-solid
```

Run `scripts/verify-cad-partial-quarter-occt.py` with the output directory using
Python with cadquery-ocp 8.0.1.0.0. The independently integrated removed-material
volume agrees with imported STEP within 6.89e-8 mm³. OCCT adaptive integration
uses Eps=1e-12: Eps=1e-10 produced a 1.63e-6 mm³ discrepancy; the acceptance
threshold remains 1e-6 mm³.

## Remaining qualification

Both orientations have identical bounded diagnostic coverage:

- 351/351 distinct face pairs visited.
- 209 pairs proven disjoint; 12 certified shared-boundary pairs.
- 130 pairs unresolved; no combined absence-of-self-intersection proof.
- 24/27 face charts proven injective; three charts remain unproven.

An unresolved result does not establish a defect. OCCT validity and volume do
not replace the missing boundary proof. The prototype is not exposed as a
source-edit command: retained source identities, a ChangeSet, whole-domain
continuity qualification, worker integration, preview/cancel and browser
acceptance remain necessary. General partial circular rims remain open.
