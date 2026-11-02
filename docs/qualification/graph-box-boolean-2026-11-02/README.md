# Source graph / independent box Boolean — 2026-11-02

Status: a new bounded intersecting family integrated with standard Boolean;
NOT a general intersecting NURBS/NURBS solver.

## API and dispatch

`minkowski3d::curved::{certify_graph_box_boolean, verify_graph_box_boolean}`
prove difference and intersection between an existing source Model and an
independently supplied box Model. Private `try_graph_box_boolean` is called by
standard `operations::boolean` after a specialized curved-family refusal and
before the disjoint-boundary proof. Intersection tries both operand orders;
difference keeps operand order. Existing successful specialized families keep
precedence. There is no new sampled or numerical fallback.

## Coverage, material and seams

The whole tool geometry/topology/trim data must equal the exact canonical cuboid
construction (persistent IDs may differ). No approximate plane/box recognition
is used as authority. The selected original source face has a source-exact
positive rational graph with an axis-aligned affine XY projection; this
identity is checked coefficientwise, not fitted from samples. Its box footprint
is computed with rational arithmetic in normalized source parameters.

1. The source's whole authored edge/pcurve intervals must agree continuously;
   partial/directed use metadata is refused. The existing source atlas proves
   a connected embedded outward closed shell.
2. Every source control point lies strictly below the tool roof, with positive
   weights. The convex-hull property excludes source/roof intersections.
3. The full closed tool column, including the space ABOVE the selected source
   face, must separate from EVERY other source carrier. This is stronger than
   merely excluding other carriers from the eventual pocket swept volume. It
   prevents missing additional material layers inside a nonconvex source.
4. The complete rectangular footprint is inside the source face trims. The
   existing exact pocket proof proves a strict graph-to-tool-floor gap over
   the complete rectangle. Hence each column intersects source material
   precisely from the tool floor up to the chosen source graph. There can be
   no second material interval, because its boundary would contradict (3).
5. Consequently source/tool intersection is exactly this graph pocket; source
   difference retains every original point outside it. The four side-wall
   intersections are the complete rational graph restrictions to rectangle
   edges. No tangent, hidden loop or extra branch can occur under these gates.
6. Existing exact CAD pocket construction splits the original trimmed face,
   retains prior holes, authors the independent floor and ruled walls, and
   assigns shared edge and vertex identities. Exact rational arc witnesses
   prove complete boundary coverage and orientation; inexact binary64 controls
   refuse. A fresh full-interval result boundary check and closed-edge sew
   audit run before publishing the ordinary editable Model.

The certificate binds both full source Models, operation, selected face,
column, boundary work, seam census, local CAD/pocket evidence, and final Model.
Verification reconstructs the proof and compares the entire certificate and
Model. Caller region labels, a changed sew count, or a changed output cannot
inherit authority.

## Restrictions and open work

- Difference/intersection only; this new family does not authorize union/xor.
- Canonical exact box tool, not a general curved cutter.
- One original nonperiodic rational graph knot cell with axis-aligned XY
  projection. The source need not share a quadratic deformation with the tool.
- The footprint guard is strictly inside the common source parameter triangle;
  cuts touching or crossing original face boundaries/seams refuse.
- Source atlas, CAD, exact arithmetic and control representability budgets
  remain in force. Public construction allows <=256 original source faces;
  automatic dispatch attempts <=64.
- Both input boundaries and every authored result boundary are checked within
  their declared tolerances; zero NEW geometry approximation is introduced.
- General intersecting NURBS/NURBS branch networks, arbitrary trimmed-face
  arrangements, sewing through original multi-face seams, and retained-region
  classification for such networks remain open. This change must not be
  described as finishing the general Boolean request.

## Validation

```sh
CARGO_TARGET_DIR=/private/tmp/osv-graph-box-20261102 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml \
  --config 'profile.test.package.brep-core.opt-level=0' \
  -p brep-core --test graph_box_boolean -- --nocapture
```

Native tests exercise a curved biquadratic roof cut by four tool walls,
standard difference/intersection including reversed intersection order,
continuous result checks, certificate/source/operation/result tampering,
missing floor/roof/side coverage, and additional material in the tool column.
The repeat-edit regression cuts a second hole in the saved first result.
Final results and hashes are recorded alongside this report. No WASM/browser,
STEP, full-suite or immutable full-workspace qualification is claimed.

## Final native results

96 tests passed; zero failures.

- `graph-box.log`: 5 passed, 0 failed.
- `operations.log`: 25 passed, 0 failed.
- `nurbs-ss.log`: 46 passed, 0 failed.
- `boundary.log`: 14 passed, 0 failed.
- `nonintersecting.log`: 6 passed, 0 failed.

The four unit groups use the copied native unit executable identified in
`unit-binary-sha256.txt`, with filters `operations::`, `nurbs_ss_g6::`,
`boundary_agreement::`, and `minkowski3d::curved::atlas::boolean::`.
The new integration group uses the copied executable identified in
`integration-binary-sha256.txt`. Both executables were built with core opt-level
0, dependencies opt-level 1, and test debug assertions enabled. These runs
qualify behavior, not release performance. Source checksums were rechecked
against the working tree after validation. The shared checkout is not frozen.

`initial-test-type-diagnostic.log` records a fixed test-only type annotation
error and is not successful validation evidence. `check.log` is the earlier
library check; the final test builds additionally compile the seam-audit gate.
Scoped rustfmt, diff whitespace, and the public NURBS API map checks passed.
