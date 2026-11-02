# Exact rational-weight affine ring tori

`nurbs_core::primitives::exact_tori::ring(center, axes, major, minor, max_work)`
constructs the entire affine image of the standard ring torus:

`center + axis0*(R+r*cos(v))*cos(u) + axis1*(R+r*cos(v))*sin(u) + axis2*r*sin(v)`.

The three finite authored axes may be oblique or reflected but must be exactly
independent. The literal finite radii must satisfy `R>r>0`. Horn/spindle tori
and zero tubes are outside this API's domain; they are not returned with an
embedded-ring claim. The older torus constructor remains available unchanged.

Output has degree2×2, a 9×9 positive-weight tensor net, domain `[0,1]²`, clamped
nonperiodic storage and two exactly repeated seam boundaries. Both angles have
variable rational clocks, distinct from the old trigonometric parameterization.
Every affine pole sum and weight must be exactly representable in binary64.
Decimal input sums, overflow, or even a one-ULP gap between radii can require
`NotRepresentable`; no rounded or partial fallback is silently returned.

Construction freshly proves the exact unit circle (NURBS-0236). The reference
is private to the construction owner and shared with the affine ellipsoid.
`exact_circle.rs` owns this freshly proved reference; `exact_frame.rs` owns
bounded exact dot/cross operations. Relocating these helpers is not another
capability. Historical NURBS-0237 qualification retains its original source
paths and version; this package reruns ellipsoid contracts and literal-output
regressions against the shared current implementation.

Verification binds actual basis, dimensions, positive weights and every
homogeneous pole coefficient using exact Cramer projections. Weights must share
one global gauge relative to the proved circle tensor. With meridian numerator
`H=R*W_mer+r*X_mer`, the entire actual tensor is thereby bound to:

`X=X_long*H`, `Y=Y_long*H`, `Z=r*W_long*Y_mer`, `D=W_long*W_mer`.

Both complete circle identities imply the homogeneous torus quartic on all16
cells. Full oriented traversal of both circles proves full coverage. Since
`rho=R+r*cos(v)>=R-r>0`, longitude can be recovered uniquely from the radial
direction and meridian from `((rho-R)/r,z/r)`. This proves injectivity modulo
the two identified seams, and regular image geometry. An exactly invertible
authored affine frame preserves these properties. No sampled gap or epsilon
rank test substitutes for these whole-domain facts.

One ledger covers fresh reference proof, construction and verification.
The existing `1..=1_000_000` work range and 4096-bit rational guard remain.
`Report.surface` is `None` on every refusal, including late proof exhaustion.
This returns surface geometry, without B-rep shell orientation, solid incidence,
volume or numeric normal-error bounds. Nearly singular frames or tiny scales
can still make floating-point normal/derivative evaluation ill-conditioned.

Qualification: [receipt](../qualification/nurbs-exact-tori-2026-11-03/receipt.json).
Native Rust only; JSON/WASM/Rush/editor/STEP and current-source distribution
remain separate qualification work.
