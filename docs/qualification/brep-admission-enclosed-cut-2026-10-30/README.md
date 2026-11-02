# B-rep admission and certified enclosed subtraction — 2026-10-30

Two bounded native Rust extensions. This is not universal admission of every
B-rep and not an arbitrary intersecting Boolean/cell-deletion implementation.

## Previously unproved source B-reps

The existing successful source-atlas recipe runs first and keeps its receipts.
After a failed proof, a second recipe may replace whole source charts only after
proving complete image equality by the existing planar/rational graph checks.
It retains original chart vertices, orientation, authoring Model and source
binding; the normal seam, vertex-star and global separation proofs then run
again. Every changed chart has a complete geometric image proof. No regularity
predicate is relaxed and an old error alone never authorizes acceptance. Each
proof attempt retains its existing finite work and geometry bounds.

The new regression authors a cube using symmetric cubic stationary clocks
R(t)=3t²-2t³ on every face and shared world edge. The unchanged old recipe must
refuse that ACTUAL model. The new recipe proves it and its certificate verifies.
A source face changed to cross the opposite boundary remains refused. This is
geometric source admission; no original parametric endpoint regularity or G2
claim is made.

## A new B-rep after removal of an enclosed curved region

Public APIs under `minkowski3d::curved`:

* `certify_enclosed_difference(container, tool, depth)`
* `verify_enclosed_difference(container, tool, result_model, certificate)`
* `CertifiedEnclosedDifference::model/certificate/into_model`
* `EnclosedDifferenceCertificate`

Both original inputs are immutable and independently pass the full source-atlas
proof. This implementation requires one connected outward outer shell in each
input, with no existing cavities. Exact nominal signed volume, together with
the proved embedding homotopy, checks outward orientation; the reported nominal
volumes are NOT exact curved-solid mass properties.

Every container chart must lie exactly in its outward corner plane. Every
positive homogeneous source control must lie on the inner side of every such
plane. Together with the connected embedded closed source proof and positive
volume, these conditions prove the complete convex polyhedral container. The
tool's complete rational control hull and proved source corner limits must lie
STRICTLY inside every support. This proves containment of its whole boundary
and bounded material, not merely a sampled centroid or a mesh bounding box.

The result copies both sets of original curves/surfaces without coordinate
rounding, offsets topology references, reverses the complete tool shell and
adds it as an inner shell of the material body. New deterministic topology IDs
are authored. A FRESH source-atlas proof validates the entire result, including
all cavity interactions. Verification reconstructs containment, output topology
and the result proof from the original inputs; caller-provided plane/volume
claims are never trusted. Source and result hashes bind the exact represented
B-reps. The test removes a closed curved cone region from a box and verifies the
new one-body/two-shell model; a changed support certificate, source tool and
cavity orientation are rejected.

## Remaining scope

Touching/intersecting tools refuse: the required closing intersection/cap
patches have not been proved. Nonconvex containers, multiple/overlapping tools,
existing cavity inputs and arbitrary deletion of signed atlas cells remain
unsupported by this operation. A caller must supply a complete closed tool
B-rep; there is no conversion from an arbitrary selected algebraic domain to a
new tool shell yet. Other source B-reps beyond the proved image-normalization
class can still remain unproved. No partial/open result is published on failure.
WASM/dist, UI/MCP transport and browser qualification are outside this run.
Legacy v4 is unchanged.

## Validation

**110 native Minkowski tests passed, 0 failed**, including all new admission,
subtraction, touching/crossing and inward-orientation cases and prior atlas,
implicit-region, cone, sphere and torus checks. The signed-support forgery was
included in the final full run. Test wall time 66.02 s is observational, not a
performance benchmark. Formatting and scoped whitespace checks pass.

The retained diagnostic log has an invalid negative-test box fixture (equal
min/max bounds); the fixture was corrected before the passing full run. That
log is not final qualification evidence.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d:: -- --nocapture
```

`source-files.json` binds seven scoped files, not a complete frozen workspace.
`native-tests.log` and `result.json` are final evidence. Concurrent unrelated
work was preserved; this milestone did not commit, push or deploy changes.
