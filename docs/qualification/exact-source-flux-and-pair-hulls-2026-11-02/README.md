# Exact oriented flux and complete conservative pair-hull audit

The full seven-stage CAD goal remains active. This builds on the exact closed source shell; it does not certify global embedding or publish a solid. Previous receipts remain unchanged.

## Actual source-surface integral

`PolynomialFlux` computes z*(x_u*y_v-x_v*y_u) in exact rational tensor Bernstein arithmetic. Original surface definitions must be nonperiodic, clamped polynomial Bezier supports with constant positive weights. New transitions use immutable exact Patch coefficients, never rounded preview poles. Full rectangles integrate by the Bernstein coefficient mean. Trimmed original domains use a polynomial V-antiderivative and Green's theorem on complete normalized directed UV curves. Polynomial composition, degree elevation, products, integration and face accumulation share the exact coefficient/bit budget. BigInt binomial factors avoid machine-integer overflow at higher degrees. Source knot domains are normalized exactly; face-use orientation determines the sign.

`ClosedShell.signed_flux` first rechecks the saved native topology and complete UV/world lifts, then uses the actual transition, retained and imported surfaces and authored trim curves. The result is algebraic oriented flux, NOT proof of a simple embedded boundary or positive physical volume. No absolute-value orientation correction or body creation is performed.

Independent tests use S(u,v)=(u,v,1+u^2+2uv+3v): the rectangle integral is 10/3 and the curved domain 0<v<u^2 has integral 1. Tests also check reversed loops, nontrivial projected Jacobian (x,y)=(2u+v,u+3v), reversed orientation, invalid/out-of-domain loops, exhausted work, rational support refusal, and degree-80 products whose binomial factors exceed machine-integer arithmetic.

The complete constant-width sewn-shell test compares to a separate graph-height derivation. The original volume is 32+2*integral(q)-integral(h). The quartic transverse blend lowers the volume by integral_0^2(h)/80 at width 1/4. For the independently integrated h=280*x^4*(2-x)^4/2^18, integral_0^2(h)=1/1152, hence the exact correction is -1/92160 mm^3. The generated whole-shell integral agrees exactly, while the original document remains unchanged.

## All unordered face pairs, with explicit uncertainty

`ClosedShell.inspect_pair_hulls` visits all unordered face pairs or refuses on insufficient pair budget. Transition hulls come from exact coefficient nets; retained/imported hulls use complete original support poles, so trimmed domains are conservatively enclosed. A strict exact axis separation proves that the pair cannot intersect. Touching or overlapping boxes remain UNRESOLVED. Shared edges are recorded as incidence metadata and never treated as an intersection certificate. Within-face self-intersections are outside this broad phase.

The expanded 27-face shell yields 351 checked pairs: 277 strictly separated and 74 unresolved, zero untested. Every unresolved pair has a shared edge; this does NOT prove that contact is limited to that edge. `unresolved-pairs.json` contains the exact face/edge indices for subsequent narrow-phase work. Tests keep the adjacent transition pair unresolved, require separated pairs to have no shared edge, and reject insufficient pair/geometry budgets.

## Terminal evidence and reproduction

17 focused NURBS tests and 13 B-rep integration tests pass. The expanded model remains 27 faces, 82 edges, 57 vertices and zero open edges. Nineteen internal G2 joins and the 10/mm field cap remain unchanged; maximum whole-support curvature bound is 9.867331764661289/mm. Signed flux is approximately 34.01287162678737 mm^3; the exact fraction is in result.json. Shared charged coefficient work is 1697116 after flux and 1801986 after pair hulls. Bodies remains empty.

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core original_graph_transition --lib`. For B-rep, add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and use `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining scope

Prove that the 74 unresolved neighboring face pairs meet only on their admitted shared boundaries, and qualify within-face embedding, radius and thickness before volume/solid admission. General rational/multi-span NURBS trimming, arbitrary contacts and complex unequal-radius nodes remain open. Native exact geometry is not yet exported through a qualified WASM/worker/UI, STEP or application history path. Three-part acceptance, P0 and P2-P3 remain open. Listed hashes are from the shared dirty checkout, not an isolated delivery revision.
