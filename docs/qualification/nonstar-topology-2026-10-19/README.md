# Nonstar and holed source topology — verified private candidate

Certificate version 3 adds exact, source-bound Rust proofs for:

- Signed prismatic atlases: nonstar profiles, multiple through-holes, nested
  negative shells, sealed cavities and disconnected material. One ambient plane
  map preserves the full labeled Jordan containment forest and every signed
  shell membership at all height levels. No star center is required.
- Polynomial ambient atlases: an exact global inverse reduces the original
  curved charts to polyhedral material. Exact filled-set equality and a complete
  moving-simplex checker connect the reference to the actual approximation.
  The checked freeform fixtures include nonstar material, a through-hole and a
  sealed cavity. This proves preservation of material and its complement.
- A separate full-sum proof for convex-hole prisms with a convex kernel:
  independent erosion bounds prove that each hole survives or is filled, and
  the actual nominal sum must have the corresponding planar product topology.

The interface and worker protocol distinguish source approximation topology
from full Minkowski-sum approximation topology. Two source proofs alone never
create a full-sum claim. Existing radial proofs and exact-source identities
remain available. Legacy certificate versions 1 and 2 keep their old recipes.

See [proof-scope.md](proof-scope.md) for the mathematical obligations and explicit
limitations. This is a checkable Rust certificate implementation, not a proof
assistant development or a universal arbitrary-NURBS topology algorithm.

## Exact candidate and evidence

- WASM SHA-256: `df2f30f7211783d8d1e1e0839f660b0d9cfff1f5ac7c57e48c7b554a24b5789a`
- WASM bytes: `12202661`
- Frozen source and packaged runtime:
  `/private/tmp/osv-nonstar-topology-source-20261019`
- Independent native checker:
  `/private/tmp/osv-nonstar-topology-checker-v3-20261019`
- Authority: original oriented NURBS face images and positive-winding filled
  material. Unrelated stored edge metadata is not certified.

[manifest.json](manifest.json) binds the exact artifact, source ledgers, checker
and checks. [source-receipt.json](source-receipt.json) records both owned changes
and unrelated native deltas already present in the shared checkout.

| Check | Evidence |
| --- | --- |
| 35 native geometry tests | `native-final-tests.log` |
| 6 existing profile regressions and 2 new forest tests | `profile-regression.log`, `planar-tests.log` |
| 7 complete native source certificate scenarios | `native-full-results.json` |
| 8 WASM certificates, 19 rejected forgeries, 1 expected refusal | `wasm-proofs.json`, `wasm-runtime.log` |
| The same 8 certificates and 19 forgeries checked independently in Rust | `standalone-results.json` |
| 5 application transactions, JSON export/reimport, originals preserved | `host-results.json` |
| 16 candidate host/protocol tests | `host-candidate-tests.log` |
| 282 existing application/UI tests on the installed kernel | `host-ui-current-tests.log` |
| 5 focused UI tests on installed and candidate kernels, including source-only status | `ui-source-only-test.log`, `ui-candidate-tests.log` |
| Vue/MCP types and whitespace checks | `types.log`, `mcp-types.log`, `diff-check.log` |

The five application certificates exactly match their raw-WASM counterparts.
The candidate runtime was exercised with Node `v26.12.0`; this record does not
claim a new Node 20 or real-browser release gate.

## Declared limitations

Prism recognition currently uses world-coordinate height axes, exact cap-curve
keys, and profile transforms exactly representable in the existing curve DTO.
Different clocks on otherwise identical cap curves can still refuse. Polynomial
atlases require constant tensor weights, affine transverse coordinates and
straight inverse UV trims. General rational deformations outside these classes
remain unproved.

For proved nonradial source atlases, v3 uses the exact original-control box as
outer source enclosure and the empty inner set. This prevents metric-cover
complexity from discarding a valid topology proof. No filled Hausdorff bound is
claimed in that case; the interface reports that it is absent.

The critical rounded display case is an explicit refusal, not a positive
certificate. The internal exact counterexample proves a curved hole survives
when its coarse sum fills it (`critical-hole-diagnostic.log`). The f32 output
embedding cannot be established, so both WASM and independent Rust refuse the
operation. No rounding check was bypassed.

## Delivery boundary and history

This task changed no common `public/wasm`, generated package, `dist` or runtime
manifest binding. The installed artifact remains `b4cd5d582ad1…`. The separate
Sweep fast path is included in the frozen source; its own qualification does
not qualify this different kernel. Preserve historical runtime v3/v4 and the
independent Sweep v5 work. Later promotion of this candidate needs its own
version and exact-artifact provider/release qualification.

`44f8bf8148c6…` and `c67d17705ab9…` were rejected intermediate candidates. They
must not be promoted. Their logs/source deltas are retained as `attempt-1-*` and
`attempt-2-*`. The former exceeded the metric-cover facet-plane budget; the
latter still exhausted tight metric-cover work on a sealed curved cavity.
Both limits were resolved by the conservative enclosure recipe above.

Older `legacy-standalone*` logs preserve successful v1/v2 compatibility checks
from intermediate native builds; they are not presented as complete release
qualification of the final artifact. Final native unit tests separately cover
both legacy certificate recipes.

## Reproduce

Run the following inside the frozen source checkout; the build destination is
private and must not be changed to the shared checkout during concurrent work:

```sh
OSV_NONSTAR_FIXTURE_DIR=/tmp/osv-nonstar-fixtures-20261019 cargo test --locked --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d::
node scripts/build-geometry-kernels.mjs --staging-root=/private/tmp/osv-nonstar-topology-source-20261019
node --import tsx scripts/check-nonstar-topology.mts public/wasm/geometry-kernel.wasm /tmp/osv-nonstar-fixtures-20261019 /tmp/nonstar-wasm-proofs.json
node --import tsx --loader ./tests/fixtures/raw-import-loader.mjs scripts/check-nonstar-host.mts /tmp/osv-nonstar-fixtures-20261019 /tmp/nonstar-host-results.json
python3 scripts/check-certified-followup.py /tmp/nonstar-wasm-proofs.json /tmp/nonstar-native-results.json --checker /private/tmp/osv-nonstar-topology-checker-v3-20261019
```
