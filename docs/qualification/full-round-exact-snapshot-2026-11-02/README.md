# Lossless exact Full Round snapshots

NativeGeometryArtifact gains a distinct `rational-brep` kind. The exact snapshot
payload stores the complete frozen artifact under `exact-full-round-snapshot/1`,
not a legacy f64 model. Creation and restoration both run the real native recipe,
material, tangency, radius and geometry comparison before admitting a body.
Restored geometry/recipe data are deeply frozen. Existing snapshot hashes and
limits still bind storage bytes; even a forged payload with valid new hashes
must pass the independent native proof.

The opt-in Vitest suite substitutes only module delivery to load the previously
qualified SHA-pinned WASM. All geometry operations run through the production
host kernel and real Rust ABI. It does not qualify production module delivery,
application worker bootstrap or the released package. Run with
`OWN_EXACT_FULL_ROUND_WASM=/absolute/path/to/qualified.wasm` and select
`tests/exactFullRoundSnapshot.test.ts`. Without that external artifact the suite
is explicitly skipped, rather than running against an older packaged kernel.

Coverage: storage JSON roundtrip preserves rational strings, IDs and frozen
recipe; display density and document revisions do not change geometry revision;
valid-hash geometric forgery refuses; an actual height edit invalidates prior
face references; damaged transport, getter metadata, wrong kind and unknown
payload fields refuse.

The separately saved document is metadata, not evidence that a parametric
history regenerates this body. General edit history, lineage transfer and
branch/occurrence stability remain open. The application scene still refuses
exact output until its publication path is integrated with these snapshots.
