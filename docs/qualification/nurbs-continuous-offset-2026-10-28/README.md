# Continuous saved-surface offset deviation — Rust and application milestone

This milestone implements priority 3 partially. It does not complete the five-priority plan.

## Contract

`nurbs_core::surface_offset::inspect_continuous_offset` compares the actual saved candidate coefficients with the original normal offset, over the common natural UV domain. A union of original source and candidate knot intervals is covered with outward-rounded rational interval jets and a mean-value bound. Both original span widths are retained when converting derivatives to UV coordinates.

Only complete coverage can return `WithinTolerance` and `error_upper`. A rigorous center witness can return `ExceedsTolerance`; exhausted cell/depth/precision limits return `Unresolved` without a whole-domain upper bound or qualified owner. Caller-controlled report fields and sampled `OffsetReport` flags cannot authorize geometry. A private owner retains both original surfaces and can reverify the calculation.

Source normal regularity, local absence of focal crossing over the signed offset segment, and the candidate Jacobian orientation are separate facts. An accurate inward cylinder offset beyond its focal radius must not acquire fold-free or orientation-preserving claims. Local chart facts do not establish global injectivity, shell closure, or Boolean authority. A literal zero-distance identity does not claim regular source normals.

## Verification

The focused integration target `surface_offset_continuous` passed all six geometry tests:

- signed plane offsets, immutable original inputs and private owner revalidation;
- a narrow knot-local defect missed by all 25 old sample stations;
- rational weights, nonunit UV domains and an independent dense error check;
- work/depth exhaustion and singular source normals;
- forged sampled/exact flags and invalid input rejection;
- rational cylinder outward and inward offsets, including a focal crossing.

Command: `CARGO_BUILD_JOBS=3 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 CARGO_PROFILE_TEST_OPT_LEVEL=1 cargo test --manifest-path crates/Cargo.toml -p nurbs-core --test surface_offset_continuous --offline --locked`.

The broader native run completed successfully before the transport addition: 3012 passed, 0 failed, 1 ignored across 166 result summaries, including doctests (`nurbs-core`, `cad-predicates`, `nurbs-intersect`; offline and locked). The transport extension is checked separately below. The application checks below cover this one diagnostic operation. STEP, global self-intersection cleanup and complete shell qualification remain unfinished.

## Remaining work

Connect the private deviation proof to construction acceptance without promoting accuracy into topology authority. JSON diagnostics and the application path are implemented; the private native owner is not transferable through the wire. Add independent sphere coverage, global self-intersection localization and trim cleanup, then shell wall connections and edited-document export qualification. The central trihedral patch and broader surface-intersection Boolean classes remain independent unfinished priorities.

## JSON transport extension

`surface_offset_continuous_deviation` accepts source/candidate surfaces, distance, tolerance and explicit maxCells/maxDepth. It recomputes the original coefficient proof and returns only diagnostics; topologyAuthority and globalEmbeddingCertified remain false. Caller-supplied exact/error fields have no effect. The native transport regression checks accepted, work-limited, mismatching and invalid-budget requests. All seven tests in the integration target passed with the transport feature enabled.

## Inherited release gates

All 807 effective frozen archive hashes match the existing registry after restoring 47 overwritten historical records from reachable commits. `archive-restoration.json` records original and restored digests plus provenance; expectations are unchanged. Architecture audit still fails on inherited oversized source files and growth beyond pinned limits. The baseline was restored unchanged; this gate has not been weakened or declared passing.

## WASM, TypeScript, worker, Rush, MCP and editor

Built `geometry-wasm` offline/locked in release with the existing production size profile, remapped source paths and isolated target directory. Binaryen `-Oz` reduced 17,708,213 bytes to 15,811,953 bytes. The packed/public artifact SHA-256 is `c8a299c3e8e2bbccc5cc6a594b735bd27fb47daa5ea7aa7e45d2c027cf1af992`.

`inspectNurbsOffsetDeviation` exposes the diagnostic through the existing geometry ABI. `offsetDeviation` is a CAD-worker job with structural response validation. The common request-identity inspection lifecycle discards late success/failure after cancellation or a source/options change. A selected surface pair exposes signed distance, tolerance, budget, cancellation and retry in the editor. Existing profile-transition command forwarding and distance measurement are retained through separate components.

Rush graph evaluation accepts `offsetDeviation` with source/candidate node IDs. MCP exposes `rush_nurbs_offset_deviation`, using the actual disposable Node process. Production Node entry points now register an explicit local raw-source loader before importing shader dependencies; this fixes the reproduced WGSL import startup failure without a shader or geometry substitute.

Final integration run: **9 files, 150 tests passed**, zero failures. It includes actual WASM, shipped worker execution, cancellation/recovery, native error/retry, old offset-contact regressions, Rush process execution, MCP discovery/calls and stale UI result invalidation. Vue/app and MCP TypeScript checks passed. The MCP CLI `--help` entry point starts successfully with the production loader.

Both development and final production Chromium 151 smoke tests passed. The production run used ordinary browser compilation settings (no unlimited-WASM flag), confirmed the known 0.25 mm offset and distance, rejected a 1 mm claimed offset, disabled zero tolerance, cleared stale reports and preserved both saved surfaces. No page errors occurred. Production screenshots were visually inspected.

## Release gates remain open

Vite production bundling passes. `verify-dist.mjs` **fails**: the packed geometry JS chunk is 5,330,908 bytes against 4,680,000 bytes. The original checkout already carried a 5,316,671-byte generated payload source; no full baseline production-build equivalence is claimed. Limits are unchanged. Native/transport/browser success is not release qualification.

A coarse chunk grouping trial caused a production startup initialization error. That trial was replaced by separate worker-result-protocol groups, and the final built application passed the complete browser smoke, including the extracted distance controls. The full five-priority plan remains active; this milestone does not close Boolean, central trihedral G2 filling, global offset cleanup, simplification or edited-body STEP export.

## Preservation checkpoint

The later [matrix.json](matrix.json) records fresh Vue/MCP type checks, 62 MCP/offset/UI tests, seven continuous-offset native tests in the wider native snapshot, the exact saved WASM artifact and a fresh check of all 807 immutable archive hashes. The original browser runs remain the evidence in application-evidence.json; they were not rerun at this checkpoint. Additional SS extensions and wire-mapper extraction are separately qualified and do not become new WASM features through this commit. Whole-plan and release qualification remain open.
