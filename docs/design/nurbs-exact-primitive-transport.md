# Native exact primitive JSON operations

These operations run through `nurbs_core::dispatch(Value)` and
`nurbs_core::execute(&str)` when feature `transport` is enabled. They call the
typed Rust constructors directly. They do not add new catalogue capabilities
or replace the older primitive operations and their raw geometry responses.
The native `geometry_bridge::dispatch/execute` facade forwards the same five
operations and refusal semantics. Its existing outer request limit is separate
from the 2 MiB `nurbs_core::execute` limit; neither limit is changed here.

| Operation | Required fields besides `op`, `center`, `maxWork` |
|---|---|
| `curve_exact_ellipse` | `axisU`, `axisV`: three-coordinate axes |
| `surface_exact_ellipsoid` | `axes`: three three-coordinate axes |
| `surface_exact_ring_torus` | `axes` (three), `majorRadius`, `minorRadius` |
| `surface_exact_frustum_side` | `axes` (three), `radii` (two) |
| `surface_exact_planar_region` | `axes` (two), `radii` (two) |

`maxWork` is a required integer in `1..=1_000_000`. Unknown fields, including
caller-provided exactness/certificate/topology assertions, are rejected.
Coordinate arrays have their exact native dimensions. All numeric leaves must
be finite. Integer coordinate/radius tokens must be exactly representable as
binary64; the adapter rejects an additional lossy integer-to-f64 cast, including
u64/i64 endpoints that would round. Decimal/exponent inputs retain the codec's
normal parsed-binary64 semantics; this is not arbitrary-precision decimal input.

A successful transport envelope has `ok:true`. Its `value` has:

- `status`: `constructed` or `refused`;
- `geometryKind`: `curve` or `surface`;
- `geometry`: the original native curve/surface serialization, or `null`;
- `workUsed`: the native geometric work count;
- `refusal`: `null`, `resource_limit`, `not_representable`, `dependent_axes`,
  `invalid_output`, or `unproved_identity`;
- `metadata`: native construction metadata, or `null` on every refusal;
- `scope`: `literal-curve-geometry` or `literal-surface-geometry`;
- `topologyAuthority:false`.

`ok:true` alone means the request was processed, not that geometry was built.
On `refused`, geometry and metadata are always null; there is no partial or
rounded fallback. Invalid inputs produce the existing `ok:false,error` envelope.
The existing 2 MiB request cap, 4096-bit arithmetic guard, geometric coordinate
limits (including the curve-control 1e9 bound), and native work caps remain.
An exact large integer radius can be useful with small authored axes while
keeping output geometry within those original limits.

Frustum metadata distinguishes `collapsedEnd:start/end/null` and `intrinsicApex`.
Planar-region metadata gives `regionKind:filledDisk/annulus`, `collapsedCenter`
and regular intrinsic plane support. A collapsed disk center is not a sharp
cone apex. These describe only the freshly returned literal geometry; they are
not transferable proof certificates for edited nets, body incidence or solids.

Qualification: [receipt](../qualification/nurbs-exact-primitive-transport-2026-11-03/receipt.json).
This stage joins a fixed committed main snapshot into the feature branch and
checks native transport/geometry together. Installed WASM, production delivery,
Rush/editor and STEP remain separate. The main snapshot's separately qualified
21,639,548-byte quadric-carrier WASM belongs to source d2dbf1ef3 and is not a
build or delivery qualification of this combined JSON-adapter source.
