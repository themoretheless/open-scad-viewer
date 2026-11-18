# ADR 0012: Module layer ratchet inside the kernel crates

- Status: accepted
- Decision: accepted for `nurbs-core` (repository owner, 2026-11-10); enforced by `scripts/moduleLayers.mjs`.
- Date: 2026-11-10
- Evidence: `docs/refactor/ARCHITECTURE.md` sections 4 and 5,
  `tools/refactor-audit/nurbs_layers.py`, `tools/refactor-audit/brep_families.py`

## Context

`scripts/audit-architecture.mjs` enforces layering between crates and that
ratchet holds at zero violations. Inside the two largest crates there is no
such rule. Measured on production code:

- `nurbs-core`: 44 of 79 top-level modules, holding 90% of the code, form one
  dependency cycle.
- `brep-core`: 107 of 244 top-level modules, holding 67% of the code, sit in
  four cycles.

Cargo forbids cycles between crates, so neither crate can be split while the
cycles exist. In `nurbs-core` the cycle rests on 17 upward module pairs with
109 references in about 50 files.

## Decision (proposed)

1. Declare an ordered list of module layers for `nurbs-core` and, once the
   module families are confirmed, for `brep-core`.
2. Check it in `audit-architecture.mjs` with the existing ratchet rules:
   current upward references are pinned in `architecture-baseline.json`, a pin
   may only shrink, and a new upward reference fails the audit.
3. Split a crate along its layers only after its pinned list is empty.

Initial order for `nurbs-core`, bottom to top:

```text
numerics < core < foundation, distance, certificates < analysis, trim
         < intersection, offset, join, editing < construct, sweeps, tessellation
         < transport < domains
```

## Alternatives

- Split into crates first and let the compiler find the cycles. Rejected: the
  first step does not compile, so it cannot be done incrementally.
- Rely on review. Rejected: the cycles formed under review.

## Consequences

- The audit needs the module reference graph. `tools/refactor-audit/audit-rs`
  produces it in about ten seconds.
- `numerics` has to be divided into a scalar part and a part that works on
  curves and surfaces, because 13 of its files reference `core`.
