# QualificationPlan — analytic-fillet/1

Status: **frozen seeds only** — capability remains `Unavailable`.

## Scope (immutable until new version)

- Cylindrical / toroidal / rolling-ball subset on convex planar edge chains
- Explicit refuse: variable-radius, setback corners, non-manifold rails

## Positive matrix (to be hashed before first implementation run)

| ID | Input | Expected |
|----|-------|----------|
| AF-01 | cuboid single edge | Complete cylindrical fillet certificate |
| AF-02 | convex chain 2–8 edges | Complete toroidal corners where admitted |

## Negative matrix

| ID | Input | Expected |
|----|-------|----------|
| AF-N1 | curved cylinder edge | typed Unavailable/Unsupported |
| AF-N2 | faceted-only claim labeled analytic | kill |

## Work units

- 3 clean CI runs after implementation lands
- False Complete resets counter and freezes Unavailable

## Non-goals

Faceted blends already in `operations` do not satisfy this plan.
