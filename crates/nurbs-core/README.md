# nurbs-core

Binary64 rational B-spline curves and surfaces, editing, interval-bounded distances
and intersections. Native types and algorithms do not depend on polygon-core,
the viewer or WASM. `Curve` and `Surface` retain rational control data.

```rust
use nurbs_core::{curve::Curve, curve_distance};
let a = Curve {
    degree: 1, knots: vec![0.,0.,1.,1.],
    control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.]],
    weights: vec![1.,1.], periodic: false,
};
let mut b = a.clone();
for p in &mut b.control_points { p[1] = 2.; }
let distance = curve_distance::distance(&a, &b, 1e-6, 1000)?;
assert!(distance.distance_interval_mm[0] <= 2.);
assert!(distance.distance_interval_mm[1] >= 2.);
# Ok::<(), nurbs_core::Error>(())
```

The default `transport` feature exposes `dispatch(Value)` and `execute(&str)`
through a separate JSON boundary. Use `default-features = false` to omit that
request dispatcher. The crate retains value-codec for existing evidence reports
and domain serialization; disabling transport does not remove that dependency.
Other runtime dependencies are `osv-math`, `geometry-ops` and `cad-predicates`.

Direct APIs include `curve::Curve`, `surface::Surface`, `curve_distance::distance`,
`surface_distance::distance`, curve/surface and surface/surface intersection,
trim-domain checks and exact-edit evidence. Interval results expose convergence,
resource budgets and unresolved outcomes; callers must preserve those statuses.
B-rep topology, solid certification and display tessellation belong to consumers.
