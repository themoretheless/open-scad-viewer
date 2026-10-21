# Exact homogeneous translated source cells

Added an internal exact homogeneous translation adapter preserving source degree, weights, original parameter domain and rational offsets. Original translated-point evaluation uses this adapter rather than a separate numerator translation. This is preparation for full translated inter-wire event census; it does not yet implement that census or publish faces.

A nonrepresentable rational-offset regression checks inverse identity, unchanged weights and identical complete finite shore root primitive, isolating intervals, original parameter maps, multiplicities and one-sided signs. Invalid nonpositive weights refuse. Fresh gates:132 profile tests,13 periodic integrations and WASM compile. Earlier212-test qualification remains historical, not a fresh full run after this edit. Catalog198/200. Rust isolated; merge with shared changes remains required.

## Shared checkout preservation

The shared checkout has external changes in crates/brep-core/src/profile_certificate.rs, crates/brep-core/src/profile_certificate/span_intersections.rs, crates/brep-core/src/profile_certificate/arithmetic/algebraic_events.rs. It was not edited or reset. The previous 83-file qualified candidate was reconstructed and checked against its saved SHA256 hashes; its cumulative patch was reversed ONLY in a generated temporary directory. All 10 restored existing baseline files matched the original baseline manifest exactly. The new cumulative patch is against that verified original baseline, not the current shared checkout. A coordinated merge is required before integration; do not blindly apply it over current shared changes.
