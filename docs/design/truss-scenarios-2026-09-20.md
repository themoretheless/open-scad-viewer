# Explicit truss load scenarios

Follow-up: [nominal axial graph workbench](nominal-truss-workbench-2026-09-20.md)
uses this API in the CAD UI while retaining an explicit nominal-model boundary.

The pinned PR7 audit now also reproduces a support-union defect in its default
`uls` combination. The compression case fixes the -Z face; the bending case
uses the -X face. `resolveActiveCase` concatenates both support lists. A solve
using that union changes the structural boundary conditions instead of combining
responses on the same stiffness model. Reproduce with
`node scripts/audit-lattice-branch.mjs`; the original function bodies are unchanged.

## Replacement contract

`src/services/trussScenario.ts` supplies `resolveTrussScenario`. A scenario owns
one explicit node/member/material model, named cases with XYZ restraint masks
and nodal wrenches, and named linear combinations of those cases.

- The active ID must resolve exactly; no fallback to the first case.
- Case and combination IDs are unique and at most 128 characters. Up to 32
  cases and 32 combinations are admitted, with 1-32 unique case terms in an
  active combination. Nested combinations and unknown case references fail.
- Every referenced case must have identical XYZ restraint masks, even for a
  zero-factor term. There is no support union or implicit grounding.
- Signed finite factors scale both force and moment, without absolute values,
  direction averaging or a minimum positive force. Explicit zero is preserved.
  Overflow and nonzero loads underflowing to zero are refused.
- Wrench origins and selected node sets are preserved. Native resultant
  assembly still owns force/couple distribution and unrealizable-load refusal.
- Admission is bounded at 125 nodes, 400 members and 32 expanded loads. No
  approximation or load merging is performed to squeeze a larger scenario in.
- Unknown fields are refused, including unsupported physical quantities such as
  member bending stiffness or moments in a different unit. Geometry/material
  validity, rank, equilibrium and numerical solve checks remain in Rust.
- Resolution copies the selected geometry, masks, loads and coefficients.
  Inactive load contents are not resolved; selected case contents are checked.

Typed assembly failures are `TRUSS_SCENARIO_INVALID`, `TRUSS_SCENARIO_SUPPORTS`
and `TRUSS_SCENARIO_RANGE`. Native solver error codes are unchanged.

`computeTrussScenario` in `mainSolidWorker.ts` snapshots before dispatch, uses the
existing shared CAD worker and returns `{activeId, terms, model, result}`. The
returned model records exactly what was solved, not a later editable scenario.
Invalid scenarios do not supersede active worker work. Existing abort/deadline
handling and recoverable native failures continue through `computeTrussAnalysis`.

This is the scenario/worker API, not a finished workbench analysis UI. It does
not infer load-bearing geometry from a lattice bounding-box graph or claim
finished-solid strength, buckling resistance, print anisotropy, safety margins
or certified load combinations. Pressure and distributed loads require an
explicit geometry-based conversion before becoming nodal wrenches. The legacy
material/environment heuristics and strength rankings remain unmerged.

## Measurements

`node --import tsx benchmarks/truss-scenario.mts` compares resolution plus the
warm WASM solve against the identical pre-resolved model. Two fresh Node 22.23.2
processes on macOS arm64 used 200 warmup pairs and 31 alternating measured pairs.
Assembly-only samples average 100 calls. Fixtures/assertions are outside timing;
no tests/builds ran concurrently. Analytical tripod member forces and exact
response equality are checked; cross-run result hashes match.

| Nodes / cases | Assembly p50 A/B, ms | Pre-resolved solve A/B, ms | Resolve + solve A/B, ms |
| --- | --- | --- | --- |
| 4 / 1 | 0.00215 / 0.00196 | 0.02271 / 0.02158 | 0.02613 / 0.02396 |
| 43 / 4 | 0.02496 / 0.02496 | 0.76208 / 0.74950 | 0.79125 / 0.77792 |
| 125 / 32 | 0.27398 / 0.26863 | 3.92346 / 3.94687 | 4.19858 / 4.22971 |

These are feature costs, not a solver speedup or worker/UI measurement. The
maximum fixture has 366 members and 32 loads, each selecting 122 nodes. The
geometry WASM remains unchanged at SHA-256
`909b94a4b895db447a184bcc6cfb544a226b51589b94b124424da6670eb0b8ca`.
Reports: `/private/tmp/osv-truss-scenario-final-{a,b}.json`.

## Verification

37 targeted scenario/native-WASM/shared-worker tests pass. They cover analytical
bar extension, signed force/moment superposition, exact-zero singular refusal,
incompatible supports, strict shapes, resource bounds, scaling range, snapshot
isolation, public async API mutation during calculation, pre-abort and worker
reuse. No extra kernel realm or numerical dependency was added.

Chrome 156.0.8063.3 passed the production CAD-worker harness with scenario
superposition, snapshot isolation, incompatible-support refusal, shared-worker
reuse and abort/restart. Its timing loop still measures the existing nodal
fixtures, not scenario resolution. Report:
`/private/tmp/osv-truss-scenario-browser/report.json`.

Vue/MCP and standalone test/benchmark/browser-harness typechecks passed.
Production Vite build and dist verification passed with the unchanged 92
artifacts totaling 15,720,446 bytes. The new host API is not yet called by the
workbench, so its unused code is tree-shaken from the application bundle.
Full Vitest: 3,426 passed and the same nine qualification artifact-binding
failures in four files, in 125.69 seconds. The suite is not globally green;
no qualification archive was rewritten. Log:
`/private/tmp/osv-truss-scenario-full.log`.
