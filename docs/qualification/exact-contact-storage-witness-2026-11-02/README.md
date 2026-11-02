# Exact contact storage counterexample

The previous local closed-body result remains scoped to six tolerance-qualified saved G2 seams and the whole-face curvature bound. Global exact source-lift admission remains false.

Added a diagnostic original-coefficient BigRational de Boor evaluator for nonperiodic rational curves and tensor surfaces, including multispan inputs. It evaluates the original saved binary64 values as exact rationals. It never issues a whole-edge identity, a G2 certificate, or a solid admission. The printed residual approximation is only a display of a rational whose exact zero/nonzero test is performed first. Source geometry and proof thresholds are unchanged.

Run `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo run --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --example expanded_contact_runout -- --exact-contact-witness`.

For both contact proposals, original C(t) - S(P(t)) is exactly nonzero in Z at t=0.25, 0.5, 1, 1.5 and 1.75; X/Y residuals are exactly zero. At t=1 the Z residual is approximately 1.9134524951558808e-15 mm. This single exact counterexample already disproves full exact identity, irrespective of continuous tolerance bounds. The unsupported global exact-lift entries therefore must not simply be admitted by raising representation caps. These measurements concern the original proposal contact curves; the global saved-model audit remains the separate authority for its saved edges.

The diagnostic regression verifies exact equality on a plane and a detectable 2^-101 residual after a 2^-100 control-point perturbation. One test passed. The evaluator is diagnostic only, with bounded degree/control layout; it is not a replacement for the production shared-work predicate context.

Next construction work must preserve exact source contact authority through boundary storage (or use a separately specified, consistently implemented tolerance-based solid admission). This turn does not change the strict global gate, snap source geometry, declare arbitrary NURBS sewing complete, or claim WASM/UI/STEP qualification. Full seven-stage goal remains active.
