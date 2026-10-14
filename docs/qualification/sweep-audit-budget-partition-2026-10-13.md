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
