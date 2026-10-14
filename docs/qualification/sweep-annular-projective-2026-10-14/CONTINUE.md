# Continuation checkpoint

Full goal remains active. This turn made native mathematical/protocol progress; it did not complete delivery or the full matrix.

## Live build

- Private stage: `/private/tmp/osv-sweep-annular-wasm-20261014`.
- Unified execution session: `68989`. Poll this same handle before deciding its state. It was confirmed running after Rust release compilation completed in50m07s; the driver was optimizing/packing via the default Binaryen JS executable. Do not restart merely because a poll yields no output.
- Build log: `wasm-build.log`. The copied `public/wasm/geometry-kernel.wasm` is still the old artifact until the packer finishes. File existence alone does not mean readiness.
- Fixed source: `frozen-rust-manifest.json`; private bridge profile `opt-level=z`. Main Cargo profile not changed.
- Next actual ABI gate after terminal successful build: `node --import tsx scripts/check-staged-annular-miter-kernel.mts /private/tmp/osv-sweep-annular-wasm-20261014 <new-report-path>`. Script added, **not yet executed**. It enforces30s native silence/120s Solid and checks direct raw/corrected budgeted inputs plus eight progressive joint-law selectors. Fix assertions against actual native report shape if necessary; never weaken completeness or budgets.

## Terminal verified work

Projective primitive tests2passed, annular owner tests3passed, all bridge owner regressions14passed, typed transport check passed. The private native example with `--circle` exported the corrected32-face STEP, proved all496pairs and actual wall/internal-cap G1/G2. Total smoothness work114160within2000000; cap/wall and sharp path stations remainC0. Independent OCCT passed. Saved receipts and logs in this directory.

The three-owner-test binary had been compiled during experimentation with automatic progressive cap correction deferred. That experiment passed but was reverted: **current production preserves existing progressive bounded correction** and defers only old terminal filled-cap proof. The staged eight-law ABI matrix must verify that exact current/frozen path. Direct corrected G2 is unaffected. Source hashes in the receipt are recorded after validation, not a claim of an atomic full working-tree snapshot.

## Critical unchanged-original-source gap

`examples/rush/miter-hollow-body.r` contains no `max_deviation`. `rushGraphNurbsKernel.ts` routes it through legacy `createMiterBrepProfileBody`, whereas the new qualified native tests use an explicitly budgeted direct variant (`0.01`). Do not describe the unchanged .r/UI timeout as fixed or silently give it that tolerance.

Next implement a Rust-owned unbudgeted direct path that preserves this distinction. Options to assess from actual code: a dedicated geometry constructor with no approximation claim, or a native numerical boundary certificate whose `budget`/`withinBudget` are null. If a finite internal acceptance ceiling is used to reuse the direct neutral-law machinery, prove that it affects acceptance only, never invent an author tolerance, and sanitize it entirely inside Rust. Existing legacy APIs stay strict. Read actual `max_deviation` usage before selecting the implementation.

Then integrate logical cap groups and separate planar optimization scope through Rush, viewport and Solid. `sweepSolidAdmission.ts` currently chooses last two faces for plane optimization; all actual faces/pairs are still native audited. New group metadata must never become a transported positive admission premise. Actual source replay and affine/station modifiers must select the same constructor representation and bind final geometry; old replay assumes legacy two-face caps.

## Further native scope

- Annular face planning currently reserves only two legacy caps. Reserve the actual `2*N` physical cap patches from original source span counts within the existing1024-face ceiling; keep refinement's maximum consistent. Standard four-span circles happen to fit, so that fixture does not prove the general case.
- Source ring orientation/cyclic correspondence and differing ring segmentation need explicit full-domain handling; unsafe correspondence must remain unproved.
- General annular physical masks can exceed16 while logical caps remain two. Distinguish logical groups, source cap masks and planar optimization masks without simply increasing proof budgets.
- Remaining smooth-sweep filled caps, common spatial closed RMF/holonomy/correction/decomposition claims, retained G1/G2 and all mode combinations still need complete audit.
- Full updated STEP/UI/cancellation qualification is pending; previous207STEP/468UI belongs to the old isolated artifact.

All work remains Rust-owned for geometry/proof/admission. JS/TS changes are transport, metadata and QA only. No focus-taking UI tools, no subagents, no Git cleanup/commit/push, no main-runtime overwrite without fresh source/artifact/size and end-to-end qualification.
