# manifold-ops

Boolean operations with a manifoldness guarantee — the operation layer of
the manifold library.

`boolean_manifold(a_pos, a_idx, b_pos, b_idx, op)`:

1. **Pre-check** — each operand must be a strict 2-manifold; otherwise it is
   repaired first (`manifold-core` Full repair). Unrepairable operands are
   rejected (`BooleanError::UnrepairableInput`).
2. **BSP boolean** — `polygon-core` kernel with default budgets.
3. **Post-check** — the output must be strictly manifold; one conservative
   weld pass handles BSP seam duplicates, anything worse is
   `BooleanError::NonManifoldOutput` (a kernel defect, never a silent
   non-manifold result).

Separate from `manifold-core` because `polygon-core` depends on
`manifold-core` — the facade cannot live in the core without a cycle.
