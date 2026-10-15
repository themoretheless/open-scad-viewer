# Section-authored blade in Rust

`brep_core::blade::profile` builds a symmetric design airfoil from two cubic
Bezier arcs. This is an explicit geometric profile, not a standardized section
or an aerodynamic performance claim. `blade::build` stacks those sections into
a capped B-rep with actual ruled NURBS walls and shared owned edges.

A section specifies the leading-edge XYZ position, chord, thickness-control
height and twist in radians. Section planes are parallel to XY and their Z
coordinates strictly increase. Local coordinates of the lower/upper arcs are

```
x(t) = chord * (1.95*t^2 - 0.95*t^3)
y(t) = +/- thickness_control * (1.5*t - 1.95*t^2 + 0.45*t^3)
```

The lower arc runs from leading to trailing edge; the upper arc runs back.
Twist rotates the local coordinates about the leading edge. The thickness
parameter controls the poles, not the maximum physical thickness. No fitting
or sampled mesh substitutes for these curves. Coordinates and trigonometric
placement use binary64; no zero-rounding-error certificate is claimed.

There must be 2..129 sections. Positions and generated controls are finite and
bounded by 1e6 in absolute value; chord is positive and at most 1e6;
thickness-control is positive and no greater than chord. Twist magnitude is
at most 1024 complete revolutions. A collapsed stored chord is refused. Caps,
shared-edge agreement and the returned model pass the existing rational
section-loft audits. Each cubic arc is split at its midpoint into two exact subarcs for the cap
constructor's minimum boundary-span requirement. At most 514 faces are constructed.

The construction interpolates section control points linearly on each Z
interval. It preserves authored section correspondence and endpoint ownership.
It does not promise G1/G2 continuity at intermediate stations, automatic
root/tip blends, standardized airfoils, self-intersection freedom under extreme
twist/stacking, global solid embedding, or WASM/editor/STEP integration.

Independent tests compare the profile with the polynomial equations, compare
actual side surfaces with ruled interpolation at interior parameters, check
closed incidence and two uses per edge, and exercise invalid inputs and limits.
