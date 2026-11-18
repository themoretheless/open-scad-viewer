# sketch-core

A bounded native 2D constraint solver. `Sketch`, `Circle`, `Constraint` and
`Solution` are ordinary Rust types; `solve` does not use a JSON request boundary.
Runtime dependency without default features: `osv-math` only.

`solve_with_diagnostics` also reports redundant equations, degenerate constraint
indices, per-constraint residuals and inconsistent linear equations. Named host
adapters retain IDs while all residual and rank calculations remain native.
`SolverOptions` selects tolerance and an iteration limit of 1..=96; native
`solve` uses 96 iterations and named RushGraph adapters use 64.
The supported tolerance interval is `1e-8..=0.1` mm.

```rust
use sketch_core::{Sketch, Constraint, solve};
let sketch = Sketch {
    points: vec![[0.,0.], [3.,1.]], circles: vec![],
    constraints: vec![
        Constraint::Fix { point: 0, at: [0.,0.] },
        Constraint::Horizontal { a: 0, b: 1 },
        Constraint::Distance { a: 0, b: 1, value: 5. },
    ],
};
let solved = solve(&sketch, 1e-6)?;
assert!(solved.max_residual <= 1e-6);
assert_eq!(solved.degrees_of_freedom, 0);
# Ok::<(), sketch_core::Error>(())
```

The default `codec` feature provides existing value-codec implementations in a
separate module. Use `default-features = false` to link the solver alone.

Constraints include fixed/coincident points, horizontal/vertical lines, distance,
parallel/perpendicular/equal-length segments, symmetry, circle radius, point-on-circle
and line/circle or circle/circle tangency. Admission bounds input to 512 points,
256 circles and 2048 constraints, with finite coordinates bounded by 1e6. Tolerance
is 1e-8..0.1; the damped least-squares solver takes at most 96 iterations.
Status and residuals must be inspected: convergence is not globally certified.

`plan_dr` adds local first-order rigid-cluster and redundancy/underconstraint reports.
`solve_with_pipeline` exposes DR planning, bounded analytic seeding and the same global
solver as separate stages. Equations are normalized by the sketch AABB diagonal while
public residuals stay in model units. `suggest_constraints` only proposes constraints;
it never applies them. Circular patterns use the existing equal-length constraints.

`validate_profile` validates polygonal outer/hole loops before a host extrusion.
`PersistentIds` and `RebuildMap` provide sidecar stable identities through explicit
rebuild correspondence without changing `Sketch` literals or constraint serialization.
