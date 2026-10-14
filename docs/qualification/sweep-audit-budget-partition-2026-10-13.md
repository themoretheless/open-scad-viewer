# Sweep audit — 2026-10-13

Status: incomplete. Fresh full STEP/UI qualification is running on an isolated, frozen candidate; the main application runtime has not been replaced.

## Confirmed defect and fix

Deferred profile seam auditing consumed additional shared work after station auditing. Rust retained the original station allocation in `station.maxWork`. The strict viewport contract requires the final partition `station.maxWork = maxWork - profile.exactWork`, so valid station G2 was presented as C0.

The original `sharp-profile-nonuniform-station-g2-miter.r` reproduces the defect: profile work1621946, station work378052, total1999998 within2000000; published station budget1992229 instead of378054. Rust now publishes the final disjoint envelope after deferred work. Geometry, tolerances, shared work limit, viewport validator and test timeouts are unchanged.

The added Rush-to-viewport regression fails on the previous candidate and passes on the corrected one. Native regression checks cover both deferred profile completion and49 nonuniform rational stations.

## Confirmed checks

- Focused native smoothness:11/11.
- Actual Rust WASM:53/53 across9 files, including the original Rush-to-viewport regression.
- Retained seam coverage and certificate provenance controls:17/17.
- Strict distribution verifier:160artifacts,8075242 asset bytes within8080000. Raw geometry WASM11453053bytes.
- Candidate SHA256:`761100c9eecdb8ddd7363881316d1950be0140c01bf51f05b9dda0ba0cd00aa0`.

Earlier complete native checking of snapshot2 passed1687 NURBS tests,417 bridge unit tests (1ignored),878 BRep unit tests (3ignored), and the remaining focused bridge integrations/docs. The full native command is **not green**: gear Boolean UV-arrangement tangent refusal reproduces in a controlled baseline with both sweep/tessellation changes removed. A main-checkout test attempt also encountered a compilation error in concurrently modified planar scalar intersection code; that unrelated work was preserved. Candidate checks use the isolated source binding.

## Fresh qualification in progress

Evidence directory:
`/Users/themoretheless/.codex/tmp/osv-sweep-isolated-20261013/qualification-budget-fix-revision3/`

`artifact-binding.json` pins native/frontend inputs, runtime and distribution. `step-run.json` schedules207 exports,9 independent audit stages (including pinned OpenCascade8.0.1.0.0), and preservation of157 original scenarios. `ui/inventory-plan.json` confirms468 cases and1432 required authentic native-running cancellation observations, including1440px and600px widths. `ui/fresh-run.json` records actual completion; planned cases are not passed cases.

The previous candidate's partial UI run was stopped only after confirming the metadata defect; its evidence and abort reason remain in `qualification-923-revision2`.

## Remaining before completion

Finish the fresh STEP and UI runs and investigate every failure. Inspect complete-boundary, global, pair-cover and retained-smoothness reports for unproved/unclassified scope; fixture success alone is not a general proof. Reconcile any later main-source drift, make the selected compiler recipe reproducible in production, and deliver the qualified WASM. The original hollow scenario is not declared fixed until its unchanged fresh case passes. The known baseline gear failure remains separately unresolved.


## Continuation — 2026-10-14

The first fresh export completed, and source/retained volume oracles plus OpenCascade passed for **157** cases. The inventory verifier correctly rejected the run: the exporter default omitted50 required companions. The207-case claim was not established. The exporter now has explicit `--full-207`, which appends23station,16miter-law and11spatial-profile-G2 cases and refuses any nonunique/incomplete inventory. A new export and complete independent audit run is executing in `qualification-budget-fix-full207`; the old failed inventory receipt remains intact.

The first9 UI groups passed356cases; the folded refusal group passed2more. Surface checking exposed two outdated assertions: a now-proved closed planar guided error bound (0.9753092634204543 within2mm), and a strictly refused off-plane antiparallel refinement. Both expectations now require the actual strict certificate/refusal behavior and restoration. Two additional actual-WASM regressions passed. The full110-case surface group is rerunning;468/468 is not yet confirmed.

Added `scripts/verify-full-sweep-ui-audit.mjs` to reconcile every original case, product/runtime/source identity, both widths, Metal renderer, lifecycle assertions and1432 authenticated executing-native cancellation observations. Historical harness versions are preserved by their captured SHA256; application source differences are rejected. Final reconciliation remains pending. No runtime or geometric limits changed, and no main WASM was delivered.


### Verified UI completion

Full reconciliation passed **468/468 cases and1432 authenticated native observations** for isolated WASM `761100c9eecdb8ddd7363881316d1950be0140c01bf51f05b9dda0ba0cd00aa0`. Receipt: `qualification-budget-fix-revision3/ui/full-reconciliation.json`. Both1440px/600px widths and AppleMetal passed. Surface110 was reconciled from31contiguous successful prefix cases plus79continuation cases; the original30s navigation failure remains preserved. No omitted or duplicated cases, changed product inputs, enlarged timeout, or edited original reports were accepted.

Minimum native sample count6; maximum CPU profile tail2.393ms; maximum actual Worker destruction2033ms. CPU tail freshness is not cancellation latency. Four corruption controls rejected missing cases, insufficient samples, paused Debugger and stale tails.

The full207 export and exact inventory/50companion checks passed, as did both independent volume references. OpenCascade and subsequent global/boundary/smoothness reconciliation remain in progress.

**Original hollow still has a real geometry gap:** the unchanged spatial source builds and renders on both widths, but Solid is refused. Diagnostic `qualification-budget-fix-revision3/original-spatial-hollow-diagnostic.json` shows8exact coedge identities reported `different` at the final cap and48unresolved face pairs. All faces are injective, all local retained charts and trim coverage are certified. The UI success includes this correct refusal; it is not proof that the original hollow produces a certified Solid. Full-library completion remains unproved, and main WASM is not delivered.


### Exact root-cause counterexample

The native-built final section of the unchanged original hollow is not exactly coplanar. Independent rational arithmetic on the exact binary64 values found19nonzero orientation determinants among24control points relative to a noncollinear anchor. This explains why a single planar cap cannot have exact coedge identity with all retained curves. Receipts `original-hollow-cap-exact-planarity.json` and `original-hollow-cap-source-binding.json` bind the counterexample to source, native WASM and emitted control data. A valid completion requires a coherent geometric representation and a proved correction bound; enlarged proof/tolerance limits cannot establish the false exact-planarity premise.


## STEP audit completed; native cap representation started

`qualification-budget-fix-full207/step-run.json` passed export207, all9 independent stages and original157 preservation. OpenCascade passed207/207. Complete-boundary audit reports203certified and4unproved; global audit reports202certified and5refused. These are finite-case results, not whole-library completion.

Unproved whole-boundary cases: `rmf-oblique-linear-scale-hollow.step`, `frenet-quadratic-hollow.step`, `rmf-planar-quadratic-hollow.step`, `open-spatial-hollow-miter.step`. The global audit additionally refuses `affine-oblique-hollow-miter.step`.

New Rust module `crates/brep-core/src/retained_annular_cap.rs` constructs radial rational patches retaining both authored boundary rings exactly, including independent positive weights. Three focused tests passed inside the actual main `brep-core`: original spatial hollow boundaries, local chart regularity/injectivity, exhausted shared work/mismatched correspondence refusal, and independent ring weights. Four patches retain16exact boundary uses without moving source curves. The simpler generic contraction checker was insufficient; the existing positive symmetric-Jacobian chart checker proves these patches. No limits were increased.

This is a native foundation, not an admitted Solid: patch-to-shell topology, global pair classification, filled-cap source-error proof, native command/WASM/Rush/viewport/Solid integration remain open. Receipt: `/Users/themoretheless/.codex/tmp/osv-annular-cap-native-20261014/native-foundation-receipt.json`.

Main public/generated WASM now has SHA `ebd2ebd4ca200beddb75a9c38cb0088b1b23c4b9769b7477402aea626a6eba5f` from concurrent workspace work. The completed207STEP/468UI results apply to isolated SHA `761100c9eecdb8ddd7363881316d1950be0140c01bf51f05b9dda0ba0cd00aa0`; they do not qualify the newly observed main artifact or the new cap module.


## Native topology and global proof advanced

The original spatial hollow now passes a fresh native global volume audit after replacing its inconsistent planar final cap by four retained annular patches. All original wall surfaces, existing edges and vertices are preserved. The29-face model has exact coedges/trim joins, injective faces, all406pairs classified, and certified orientation. Replacing both caps yields32faces with all496pairs certified through the generic algorithm, without planar cap specialization.

Nine cap tests plus43existing native geometry regressions passed. A single-bit boundary change invalidates the global certificate; insufficient real work and overlap remain refused. See `sweep-annular-topology-2026-10-14/README.md` and `native-receipt.json` for the proof scope and source/log bindings.

These native results do not establish original-source filled-cap error, qualify a new WASM artifact, or replace the earlier finite207STEP/468UI receipts. Remaining work includes original-reference filled-domain/error proof, constructor-owned cap grouping, native→WASM→Rush→viewport→Solid delivery, other unresolved modes and fresh qualification. The full goal remains active.
