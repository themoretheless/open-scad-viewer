# Selected original boundary admission after exact interval clipping

Fixed a real false refusal in selected original-curve winding. Previously a query on ANY portion of the supporting original cell was rejected before the actual algebraic selected interval was checked. The complete source ray census now has an internal clipping mode which retains exactly certified zero-X roots. Its independent verifier still checks complete roots, exact zero identities, ordering and germs. The original whole-cell producer and verifier remain strict.

Selected winding compares each root against both exact native-parameter cuts BEFORE deciding boundary admission. A proved point on the closed selected interval returns None; an excluded source-cell point does not. Proof-budget exhaustion remains an error, never a proved boundary or clear result. Horizontal cells use a complete X-coordinate fiber census and the same exact clipping. Existing half-open germ contributions remain intact.

Regressions cover excluded/included/end-cut points, original nonunit domain, an irrational sqrt(1/2) cut, horizontal cells, even stationary tangent roots, and malformed census witnesses. A public union regression uses intersecting rectangles and queries a point on the removed internal portion of an original bottom cell: winding is one and source replay passes. A true outer corner still refuses as boundary.

Fresh gates:171 profile tests,74 integration tests (65 original_overlap_components and9 curve_event_nodes), and WASM compilation. These are candidate-native checks, not UI/release admission or proof that all historical Boolean refusals are closed.

Catalog remains198/200. NURBS-0030 still needs complete general surface intersection discovery/branch graph/actual trims. NURBS-0173 still needs global material winding labels, operand Boolean predicates, connected region ownership and physical face publication. The next step is exact chart-local relative winding from declared base points using original source intervals and separate original-wire jump columns. Essential periodic wires must be grouped by operand before period compatibility is required. Equal winding vectors alone do not prove connected faces.

Rust source is isolated. Shared Rust changes are preserved; the cumulative patch needs coordinated integration before checkout admission.

## Shared checkout preservation

The shared checkout has external changes in crates/brep-core/src/profile_certificate.rs, crates/brep-core/src/profile_certificate/span_intersections.rs, crates/brep-core/src/profile_certificate/arithmetic/algebraic_events.rs. It was not edited or reset. The previous 83-file qualified candidate was reconstructed and checked against its saved SHA256 hashes; its cumulative patch was reversed ONLY in a generated temporary directory. All 10 restored existing baseline files and the newly included ray_winding baseline (copied only after exact original-manifest SHA256 equality) matched the original baseline manifest exactly. The new cumulative patch is against that verified original baseline, not the current shared checkout. A coordinated merge is required before integration; do not blindly apply it over current shared changes.
