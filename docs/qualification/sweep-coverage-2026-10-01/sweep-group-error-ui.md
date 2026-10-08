# Group source refusal qualification — 2026-10-04

Observed localhost:5202 through the actual browser UI. The original Solid
scene contained group `model`, one B-rep body, no selection.

The group editor previously had no error region although `buildSolidGroup`
recorded failures in `error`. Added an assertive alert to that editor.

Using the affine + authored frame + orientation guide hollow quadratic
profile example (`examples/rush/miter-unsegmented-frame-guide-affine-hollow.r`,
compact equivalent source with tessellation 4):

1. Set scalar scale values to [0,0]. Build refused with visible
   `Miter scalar laws require [value,0,0]; scale must be positive`.
   Original scene still contained one body. Screenshot: `sweep-group-refusal-wide.jpg`.
2. Restore scalar scale [1,1]. Build succeeded: the named qualification
   group and its B-rep body appeared, scene count 2.
   Screenshot: `sweep-group-solid-success-wide.jpg`.
3. Reopen the new group's stored source, set viewport 720 x 900, replace
   scalar scale with [0,0]. Rebuild refused with the same visible alert.
   Screenshot: `sweep-group-refusal-narrow.jpg`.
4. Cancel editing, hide source, reset temporary viewport, Undo the successful
   insertion. Scene count returned to 1, only original `model` remained,
   selection cleared. Screenshot: `sweep-group-restored-wide.jpg`.
5. Closed the temporary browser tab.

This verifies successful group Solid construction, visible scalar-law refusal,
retry after refusal, stored group-source reopening, edit cancellation and Undo
restoration. It does not qualify cancellation during active worker computation,
source replacement races, narrow-window success, all geometric refusal classes,
or the complete UI matrix. Geometry remains evaluated by Rust/WASM.

Validation: direct Vue typecheck and scoped diff check. `npm run typecheck`
also invokes `pretypecheck` rebuilding every kernel; that unrelated rebuild was
stopped during wasm-opt and is not reported as a passing check.
