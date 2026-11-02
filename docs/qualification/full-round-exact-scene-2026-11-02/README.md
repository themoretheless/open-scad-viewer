# Exact Full Round internal scene publication

The internal BRep scene adapter selects the exact display path for a
`rational-brep` result and stores a complete verified exact snapshot. Analytic
results retain their existing path. Display triangle face indices map to the
actual exact topology face IDs; semantic entity/occurrence ownership is kept.
Display metrics remain estimates and sampling error remains uncertified.

The opt-in real-WASM suite uses the SHA-pinned module from its saved ABI
qualification. Only compilation/delivery is substituted in the test; Rust
executes source admission, graph evaluation, display, snapshots, BVH and mesh
processing. The lifecycle tests additionally inject cancellation at defined
ownership boundaries. This is not packaged Worker/production-engine delivery.

A regression exposed and fixed top-level multi-output invocation identity in
both TS and native result admission: producer and identity roots are compared
using their canonical invocation, while per-output node/owner checks remain.

Coverage includes scene payload admission, scene roundtrip with exact snapshot
restoration, stable geometry/entity/IDs across tessellation detail, stale face
selection after a real source height edit, cancellation before publication,
recovery after abort, and two mapped exact outputs with separate ownership.

This adapter is still an internal executable seam. Released kernel delivery,
actual application Worker integration, production-engine registration, general
exact edit/history/lineage transfer, downstream rational operations and the
complete original parity scope remain open.

Run the opt-in suite with `OWN_EXACT_FULL_ROUND_WASM` pointing to the SHA-named
module listed in `module-identity.json`. `OWN_EXACT_FULL_ROUND_RECEIPT` can
explicitly select that identity file; the default for this suite is this stage's
identity, not the older module that lacks multi-output root admission.
