# Nominal axial graph workbench

The spatial lightening controls now expose an **Axial graph model** panel.
This integrates the checked native solver and scenario API into an editable
workflow without importing PR7's strength rankings or heuristic material factors.

## Model boundary

`cad_spatial_graph` uses the source mesh's bounding box, not the clipped members
of the finished lightened solid. The UI and JSON report identify the model as
`nominal-bounding-box-axial`. It excludes surface clipping, shell/core stiffness,
beam bending and buckling; it is not a part-strength assessment.

A real-worker regression demonstrates this distinction: a cube and a triangular
prism with the same bounding box produce the same 14-node/36-member octet graph,
even with different shell/wall/core options. The graph is not promoted to a
finished-solid model by attaching analysis results to it.

The new `latticeGraph` request runs placement/welding and native graph generation
inside the existing CAD worker. Only geometry arrays and scalar options cross
the request boundary; scene buffers are cloned, not transferred. Admission is
bounded at 100,000 source triangles/16 MiB, 125 distinct graph nodes/400 unique
members and finite three-dimensional bounds. Planar/duplicate-node graphs fail
with a recoverable operation error. Result validation binds the nominal model
kind, coordinates, counts and edge indices before the UI receives the graph.
The byte limit counts distinct whole backing buffers, including the transform,
not just visible typed-array slices. Shared-memory buffers are refused so input
cloning really snapshots the geometry; aliased views are counted only once.

## Workflow

- Generate the nominal graph for one selected, currently built body. Uniform E
  and member area require explicit positive values; no material is assumed.
- Inspect node coordinates and explicit XYZ restraint/load checkboxes. Initial
  nodes are unrestrained and unloaded. Bulk bounding-plane selections use exact
  graph coordinate extrema and remain visible in the table; no nearest-node
  support, hidden anchor or implicit ground constraint is introduced.
- Specify total XYZ force in N, moment in N mm about the displayed origin, and
  the selected loaded nodes. Choosing their centroid updates that explicit origin.
- Duplicate/remove named cases, edit their masks and wrenches independently, and
  solve a single case or a signed linear combination. The UI supports up to 32
  cases with one wrench each; the scenario API still handles multiple wrenches.
  Combinations with different restraints refuse instead of unioning supports.
- Inspect displacement, residual, free DOFs, signed member forces/stresses, and
  node displacements/reactions. Export the exact solved model, coefficients,
  response and nominal graph settings in `nominal-truss.json`.

Input edits immediately discard results and revoke the previous report URL.
Changing source, readiness, selection, selected mesh/geometry buffers, transform
or lattice options also discards the graph. Cancellation terminates active
noncooperative work through the shared client. Revision checks prevent late
responses from restoring invalidated results. Material/case changes and graph
generation are not auto-solved on every keystroke.

The panel is a separate async Vue component, loaded for spatial lightening.
It follows the existing workbench layout with bounded scrollable node/member
tables; no second solver, geometry realm or numerical dependency is introduced.

## Verification

`scripts/check-nominal-truss-browser.mjs` mounts the actual CAD panel with real
geometry and a real worker on desktop 1280x900 and mobile 390x844. Chrome
156.0.8063.3 passed:

- 14-node/36-member octet generation; no automatically restrained nodes.
- Native singular refusal before supports are explicitly set.
- A -100 N load balanced by +100 N support reactions.
- A `1.2 * (-100) - 0.5 * (-50)` combination balanced by +95 N reactions.
- Doubling E halves the displacement; JSON reports retain the actual inputs.
- Incompatible supports refuse; editing invalidates the report immediately.
- Holding a real worker reply, editing, then delivering its captured stale
  callback does not restore a result. Retry creates a new worker and succeeds.
- Mesh replacement within the same scene array and source changes discard the
  graph. Controls stay inside the workbench at both viewport sizes; screenshots
  were inspected and neither page reported JavaScript errors.

Reports and images: `/private/tmp/osv-nominal-truss-browser-final.json`,
`/private/tmp/osv-nominal-truss-{controls-,}{desktop,mobile}.png`.
This is UI/worker behavior evidence, not physical validation or a latency benchmark.

The separate production-worker harness also passes the new graph request with
translated scene geometry, expected world-space bounds, unchanged caller buffers,
worker reuse, scenario refusal and abort/restart. It loads the emitted worker
from `dist/assets`, rather than the development worker used by the UI probe.
Report: `/private/tmp/osv-nominal-truss-production/report.json`.

38 focused protocol/native-WASM/worker/scenario/CAD tests pass. Vue, MCP and
standalone test/harness typechecks pass. Full Vitest: 3,429 passed, nine existing
qualification-binding failures in four files, in 175.93 seconds. No archive was
rewritten; the suite is not globally green. After the final mesh-watch change,
the focused tests and both browser viewport workflows were rerun successfully.
After the backing-buffer guard was tightened, its protocol tests and the real
worker/CAD UI regression subset passed again (10 tests).
Log: `/private/tmp/osv-nominal-truss-full.log`.

Vite and dist verification pass with 95 artifacts, 5,991,807 asset bytes plus
9,751,374 raw WASM bytes: 15,743,181 total, +22,735 bytes. Existing budgets pass
without increases. Geometry WASM is unchanged at SHA-256
`909b94a4b895db447a184bcc6cfb544a226b51589b94b124424da6670eb0b8ca`.

Finished-solid structural derivation, experimentally grounded FDM properties,
bending/buckling models and physical validation remain separate work. This UI
does not complete or justify wholesale merging of the legacy strength branch.
