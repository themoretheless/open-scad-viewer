# Loft generalization: continuing goal

The complete goal remains: preserve active CAD work and stash, clean confirmed obsolete Git state, support arbitrary section reparameterization, incompatible rational weights, nonplanar caps and global self-intersection checking, and remove the former 11-section / 9-guide / 32-control restrictions. This continuation is not complete until runtime delivery, validation and publication cover those requirements.

## Implemented in the development branch

- `loft_reparameterization::prepare` accepts a complete strictly monotone rational piecewise map per normalized section. Null entries retain their parameterization. Cropping and nonmonotone maps refuse. Natural and automatic guided loft JSON constructors accept `section_mappings`; TypeScript uses the existing `RationalReparameterization` type. Automatic guided output includes the mapped sections and their map certificates so downstream matching can use the same definitions.
- Affine map pieces preserve source curves with multiple knot spans. Piece joins align degrees and homogeneous scales. Materialized nested maps use the same first-to-last authored evaluation order as the direct map evaluator; certification checks whole factor extents and bounds recursion. Nonlinear composition uses the actual degree-product budget (25), rather than an artificial degree-8 input restriction.
- `foundation::bound_reparameterization_preimages` now encloses knot inverses for rational piecewise and nested maps with outward de Casteljau arithmetic. It separately proves positive derivative coefficients using outward Bernstein products, shares an explicit leaf-evaluation budget across roots/factors, and refuses when either work or binary64 precision cannot establish the requested interval width. Interior inverse brackets are not promoted to exact knots. This is a native prerequisite; the nonlinear materializer has not yet consumed these brackets.
- BRep section lofts retain different rational weights between corresponding section spans. The positive-weight interpolation gate and cap/topology audits remain in force. Tests change conic shape rather than merely multiply every weight by one scalar.
- `LoftCap` contains an authored NURBS carrier plus outer/hole UV trims. `capped_loft_with_caps` supports nonplanar carriers and noncoplanar boundary curves, and requires exact whole-edge/lift agreement. `capped_loft_with_caps_checked` additionally requires whole-boundary embedding: valid trims, chart injectivity and classification of every face pair. Incomplete work refuses. The new JSON/TypeScript cap constructor uses this checked path.
- Surface nets now admit 256 controls per axis. Cubic section/grid/Gordon constructors admit 86 sites (3*(86-1)+1 = 256 controls); automatic guided loft conservatively admits 84 guides to reserve two boundary guides. Degree, control growth and stage-specific work budgets still apply. Not every maximal combination fits every downstream operation.

These are source/native changes. The packaged WASM and language/runtime delivery have not yet been updated for this continuation.

## Evidence

- Different-weight hollow BRep loft: retained rational sections at authored stations, closed shared-edge topology and STEP export.
- Nonplanar cap fixture: z = z0 + u*(1-u)/2, with genuinely noncoplanar boundary curves. Retains the carrier through STEP export/import; an interior boundary distortion refuses although endpoints still coincide.
- Whole-boundary embedding: the valid six-face curved-cap loft proves absence; a front-wall graph penetrating the back wall does not pass; an exhausted pair budget does not pass.
- Capacity: 86-section natural loft uses exactly 256 V controls and retains all authored sections; section 87 and control 257 refuse. A 12-section / 10-guide case crosses all former restrictions and retains its straight guides.
- Parameter maps: independent rational/polynomial formulas, weighted piece joins, multi-span affine maps, noncommuting nested maps, complete piecewise-factor extents, degree-nine nonlinear maps and refusal cases. Retention reports from automatic guided loft refer to the mapped definitions; a full composition rounding bound relative to the original source is not yet claimed.

- Knot-preimage continuation: irrational polynomial inverse, rational nested-factor order, piece boundaries/nonunit domains, work exhaustion and binary64 precision refusal. Full `nurbs-core` validation passed: 286 unit tests, 2 capacity tests and 1 doctest.

## Still required by the complete goal

1. Materialize nonlinear maps crossing source interior knots, including use of the new bounded knot-preimage construction and whole-composition numerical retention evidence. The current nonlinear materializer still requires one source Bezier span per map piece.
2. General guided construction for geometrically compatible curve networks with incompatible homogeneous crossing weights. Different-weight BRep sections are supported, but arbitrary guided network weights are not yet resolved. Natural homogeneous weight overshoot also remains a refusal; it must not become an unlabelled fit.
3. Complete Rust/JSON/WASM/TypeScript/Rush integration and examples for the new paths. Rebuild artifacts, append fresh source/artifact evidence without rewriting historical qualification, verify browser/STEP behavior, run CI and publish through a new PR.
4. Cleanup only after explicit confirmation: two merged local branches (`codex/cad-ready-2026-09-30`, `codex/laser-cam-integrate`) and three stale worktree registrations (`open-scad-viewer-cad-ready`, `open-scad-viewer-laser-cam`, `open-scad-viewer-laser-integrate`). Preserve the unmerged foundation branch, active CAD checkout, primary dirty checkout and stash.
