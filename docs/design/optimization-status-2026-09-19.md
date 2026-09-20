# Optimization completion audit

Subsequent local progress: [MCP cold-start phase](mcp-cold-start-phase-2026-09-19.md)
reproduces and fixes the disposable-worker readiness failure without changing
the global 250 ms limit. The full-suite snapshot below predates that change;
its focused lifecycle/readiness/isolation verification passes 52 tests.

Snapshot: main e2283155426f2afaa40a9d558b2d604e65ad3a28 plus local optimization
changes on 2026-09-19. This supplements, not replaces, the historical
[initial audit](optimization-audit-2026-09-19.md). Its original branch, code
counts, absolute performance results and backlog observations are not a current
release certification.

## Current verification

Latest integrated control, completed 2026-09-20: 3289 tests passed and the same
nine evidence gates failed (341 passing files, four failing; 110.89 seconds).
This includes the isolated historical manifests, shared bounded initialization,
cancellation-preserved warmup and [mesh history optimization](mesh-history-2026-09-20.md).
Log: `/private/tmp/osv-mesh-history-integrated-tests.log`.
Remote control run 35475827899 on fd03ebcf has completed: Rust, official OpenSCAD,
macOS and Windows smoke passed; Node 20/22 checks failed. That older remote run
does not contain the later startup corrections or the mesh-history change.

A later uninterrupted full run on published `8f96044d` passed 3271 tests and
failed 10 (336 passing files, five failing; 110.18 seconds). In addition to the
nine evidence failures below, `coreMesh.test.ts` detected a new direct parser
import in the MCP worker startup path. The worker now warms the existing
`defaultGeometryKernel` facade directly, preserving parser-consumer boundaries
without widening the allowlist. After that correction, all 39 tests across
coreMesh, MCP supervisor, production isolation and MCP CLI passed, as did MCP
type checking. A second full run after this correction has not been performed.
Log: `/private/tmp/osv-8f96044d-full-tests.log`.

The previous remote run 35473144674 has now completed: Rust, macOS native smoke,
Windows native smoke and official OpenSCAD passed; both Node jobs failed.
Run 35475213799 for `8f96044d` was still in progress when inspected and does not
contain the subsequent parser-import correction.

Pre-publication focused verification passed 42 tests across compiler registration,
streaming, surface groups, MCP supervisor and production isolation, plus six Node
tests for package integrity and the new read-only drift auditor. UI type checking
also passed. These checks do not replace the full-suite snapshot below.

`npm run audit:qualification-drift` checks all bindings without writing evidence.
The current audit reports 125 records: 88 matches and 37 mismatches. All 67
historical archives match. Drift comprises 32 frozen-input records, one kernel,
three dependency bindings and one source bundle; duplicate input records are
retained. Exit status 1 is expected while drift remains. The v9 release generator
uses fixed output names and inherits dependency fingerprints from v8, so rerunning
it is not a safe refresh. A new versioned evidence publication and reviewed
qualification runs are still required; updating hashes alone is not qualification.

An uninterrupted full `vitest run --maxWorkers 2` on the integrated local
source state passed 3264 tests and failed 9 (337 passing files, 4 failing;
101.20 seconds). No source edits or other builds ran during this control.
Log: `/private/tmp/osv-optimization-control-tests.log`.

Remaining failed suites:

- `engineManifest.test.ts`: current own-Rust evidence binding.
- `g0ToolchainFingerprints.test.ts`: exact artifact binding and five refresh
  generator scenarios depending on frozen inputs.
- `g1GithubActionsQualification.test.ts`: frozen evidence-producer binding.
- `qualificationPlanArtifact.test.ts`: historical v27/current-byte assumptions.

These are real failing gates, not waived tests. Do not rewrite historical
evidence in place or claim qualification by updating hashes alone. New evidence
versions need the repository's reviewed refresh process and the required runs.

Remote CI 35473144674 on e2283155 had successful Rust and official-OpenSCAD
jobs when inspected. Node20/22 jobs failed; macOS/Windows native jobs were
still running. This remote run does not include the local fixes. In particular,
its readiness timeout failures are not disproved by this local green functional
subset. There is no green overall-CI or deployment claim.

## Requirement Status

| User objective | Current evidence | Still required |
| --- | --- | --- |
| Move useful hot work into Rust | `abi_render_mesh` and `abi_analyze_solid` exist in geometry-wasm and are used by meshAnalysis; measured base85 optimization retained | `inferSurfaceIds` still runs in TS; profile its complete publication cost before another ABI port |
| Benchmarks and measured wins | Alternating baseline/candidate bootstrap runs, exact output checks, cold-readiness controls and full artifact accounting | CI contention report has not run with the new workflow; no universal speedup claim |
| SOLID, DRY, modularity | Browser-only compiler composition root; pure shared kernel compiler interface; bounded surface-group cache | Two TS OpenSCAD semantic paths remain; scene ownership and broad frontend decomposition are not complete |
| System design | Host I/O isolation, streaming fallback deadline, raw/packed identity gates, disposable-worker lifecycle checks | Cold startup versus 250 ms provider-readiness policy remains unresolved |
| Competitor comparison | Initial audit records a comparison and possible product gaps | No current competitive performance parity proof; historical comparison is not a release acceptance test |
| Greenfield design | Initial audit proposes one semantic frontend, typed/handle transport and consolidated solid analysis | Only parts are implemented; replacement of the semantic frontend needs differential conformance evidence |
| Bottleneck/problem search | Duplicate 2.79 MB literals removed; decoder phases measured; transitive I/O regression fixed | Frozen-evidence drift and CI readiness failures remain open |
| Functional expansion | Prior committed work includes Solid STEP import/export and fresh-context roundtrip qualification | Full prioritized feature-gap program is not finished; do not count a proposal as implementation |

## Source Revalidation

- `src/services/meshSurfaceGroups.ts` still calls `inferSurfaceIds`; its content
  cache now uses `SurfaceGroupContentCache`, so the old unbounded-cache finding
  is resolved but the CPU placement finding is not.
- `src/services/geometry/meshAnalysis.ts` calls `abi_analyze_solid` and
  `abi_render_mesh`; the broad recommendation to introduce them is no longer
  an untouched task.
- Both `src/workers/geometry.worker.ts` and
  `src/services/buildCoordinator.ts` invoke `isGeometryWorkerEvent`.
  Removing a validation pass requires preserving boundary trust and measuring
  its cost, not merely deleting one call.
- Production source search finds `scadCompileRust` / `scadEvalRust` definitions
  but no product call sites. The Rust frontend is not yet the shared semantic
  execution path.
- `.github/workflows/ci.yml` runs Clippy for `osv-math` with warnings denied;
  the initial audit's blanket statement that CI does not run Clippy is stale.
  This is not a workspace-wide clean-Clippy claim.
- `src/App.vue` still uses textarea-based source inputs. An editor replacement
  remains separate product work, not a completed optimization.

## Next Decisions

1. Separate disposable-worker cold initialization from request readiness only
   after a controlled reproduction and explicit lifecycle/deadline tests. Do
   not globally raise the 250 ms limit merely to turn CI green.
2. Reconcile live manifest/evidence versions through the existing qualification
   process. Keep historical artifacts immutable and preserve no-claim status
   until full qualification is actually complete.
3. Profile publication/grouping and retain only end-to-end improvements with
   selection/provenance parity. Avoid removing consumer-side validation of
   transferred worker data.
4. Treat a unified Rust semantic frontend and larger product gaps as separately
   verified implementation work. The broad optimization goal remains active.
