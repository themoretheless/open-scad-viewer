# Curvilinear charts, poles and projective seams (2026-10-20)

Native qualification passed. Shared runtime artifacts are not promoted.

## Exact Rust changes

- A rectangular chart with a collapsed boundary can be replaced by a triangular
  chart only after an exact homogeneous polynomial divisibility identity.
  Rotations cover all four sides. The singular rectangle is not silently omitted.
- Positive projective barycentric changes synchronize nonlinear seam clocks.
  Exact endpoint jets propose a fractional-linear correspondence; the complete
  rational identity is then verified. A simultaneous multiplicative system
  checks cycle consistency across all triangles, including exact rational roots.
  Existing pointwise seams retain their original recipe.
- A three-edge rational UV boundary without holes can be filled by a triangular
  rational chart. Its projected Jacobian and Jordan boundary homotopy are
  checked before composing it exactly with the original tensor NURBS surface.
  All UV controls must remain in one tensor knot cell. The resulting source
  atlas still passes the usual local-star and global-separation checks.

## Scope

These are sufficient conditions; no universal NURBS admission is claimed.
Arbitrary non-projective clocks, irrational projective scales, general holed or
multi-edge trim triangulations, unresolved internal knot crossings, and poles
that fail the polynomial blowdown identity are still refused.

The standalone source-atlas operation uses these charts. The Minkowski source
report additionally requires equality with its actual nominal approximation
chain; a standalone proof must never bypass that binding or imply that the
full Minkowski sum preserves topology. The existing enclosure preflight may
also refuse non-pointwise source seams.

Shared WASM, generated assets and dist have not been updated by this task.

The synthetic sphere uses exact rational circular trim clocks, rather than a
rounded sqrt(1/2) weight. It is a distinct authored input, bound into its own
certificate; the ordinary constructor's approximate equator is not silently
accepted. Both source poles are included. See [proof-scope.md](proof-scope.md).

A pre-existing event-node optimization also prevented compilation and rejected
supported intersection certificate versions2/3. The local correction extracts
its verified node tuple and retains the public verifier's version/count checks.
Other edits in that shared file are preserved.

## Final validation

- 55 Minkowski/source-atlas/rounding Rust tests and1 event-node regression passed.
- 4 source certificates produced and replayed in a separate Rust process;
 28 mutated bindings/schemas/counts/connectivities rejected.
- Exact sphere:32 chart triangles,18 vertex stars,328 disjoint pairs,216 link pairs.
- Exact collapsed tetrahedron, nonlinear-clock freeform box and original rational
 freeform box were also replayed. JSON zero normalization occurs before fresh
 certification, matching the browser transport convention.
- The four requests and certificates are identical before/after the event-node
 compatibility correction; the final executable actually reverified all four.
- Frozen source ledger:3641 files; snapshot drift0; owned live Rust drift0.

Checker SHA256:
`e804ed7479c5942b7d46c5e0f13649b1a36119cc034d7916da25e0ab1a1261db`.
Receipts: [manifest.json](manifest.json), [native-replay.json](native-replay.json),
[native-tests.log](native-tests.log), [event-node-regression.log](event-node-regression.log).

This is native source-certifier qualification. No new WASM, browser/host scenario
or dist/package qualification was performed in this turn.
