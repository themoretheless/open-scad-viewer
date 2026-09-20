# Truss WASM boundary

The bounded `mechanics-core::truss` solver now uses the existing binary geometry
transport through `geometry-bridge::truss` and `src/services/trussAnalysis.ts`.
There is no second solver, JS fallback, separate WASM runtime or inferred support.
The adapter does not yet connect to workbench controls or PR7 scenario semantics.

## Contract

`truss_solve` requires exactly `op`, `nodesMm`, `members`, `restrained` and
`forcesN`. Each member requires exactly `nodes`, `youngMpa` and `areaMm2`.
Coordinates/displacements are mm; loads/reactions/axial forces are N; modulus
and stress are MPa. Member forces are positive in tension. Support masks are
explicit zero-displacement XYZ constraints, one mask and force vector per node.
No moments, springs, material defaults or support heuristics are silently used.

The bridge checks collection limits before cloning numerical arrays; the common
binary decoder necessarily runs first. Native validation remains authoritative
for node/member budgets (125/400), topology, numeric ranges and singular modes.
Malformed transport has `GEOMETRY_INVALID_INPUT`; native `TRUSS_*` errors keep
their codes through `GeometryKernelError`. Nonfinite JS numbers are rejected by
the common binary encoder before entering Rust, with its existing host error.

The response includes displacements, signed support reactions, ordered member
forces/stresses, maximum deflection, free DOFs and relative residual. This is
linear axial-bar analysis, not bending, buckling or certified strength.

## Evidence

- All 262 native geometry-bridge tests pass, including two new adapter tests.
- Four real-WASM tests pass: analytical bar, typed singular refusal/recovery,
  malformed inputs, and the 125-node/366-member boundary.
- Boundary checks compare each member against an independent analytical
  statically determinate tripod solution and check global force/moment balance.
- Vue, MCP and standalone benchmark TypeScript checks pass.
- Full Vitest with local HTTP access: 3,339 passed, 9 failed across 348 files
  in 108.93 seconds. Failures remain the same qualification bindings:
  `engineManifest` (1), `g0ToolchainFingerprints` (6),
  `g1GithubActionsQualification` (1), `qualificationPlanArtifact` (1).
  Log: `/private/tmp/osv-truss-full-tests.log`. No archives were rewritten.
- Vite build and `verify-dist` pass: 92 artifacts, 5,942,640 asset bytes plus
  9,729,072 raw WASM bytes, totaling 15,671,712 bytes. This includes the new
  144,525-byte complete numerical-dependency license bundle.
- Node tests execute actual WASM, but are not browser UI/worker proof.

The public geometry WASM is 7,701,210 bytes, SHA-256
`329d16db9f7d27e20c9ec457d6da04508121ad517dac5961aa01eb98b9079580`.
Compared with the preceding 7,685,259-byte artifact, the increase is 15,951 bytes
(about 0.21%). Its previous hash was
`b4ad458cd84207f41e788ece11208b1018fa151552e8985512e844116e36a8bb`.
All normal wasm32 nalgebra dependency license files were copied unmodified from
the pinned Cargo registry packages to `public/third-party/truss.txt`.

## Measurements

Run `node --import tsx benchmarks/truss-boundary.mts` after building geometry.
The script explicitly loads and hashes the public WASM, uses the same three
supported-tripod fixtures as the native example, warms 20 calls and measures
31 calls per case. Fixture construction and correctness assertions are outside
timing. No builds or tests ran concurrently with these two benchmark processes.

Node 22.23.2, macOS arm64, complete warm encode/solve/decode call:

| Nodes / free DOFs | p50 run A / B, ms | p95 run A / B, ms |
| --- | ---: | ---: |
| 4 / 3 | 0.03838 / 0.03825 | 0.05096 / 0.09167 |
| 43 / 120 | 0.77546 / 0.77362 | 0.96308 / 0.99800 |
| 125 / 366 | 12.57396 / 12.49867 | 13.18846 / 12.95358 |

Cold raw-module compile/instantiate was 6.25/6.27 ms in these processes, excluding
file reading, network and embedded Brotli fallback. This is not cold page load.
Reports: `/private/tmp/osv-truss-boundary-{a,b}.json`.

This establishes integration cost, not a TS speedup. The native maximum workload
previously measured about 2.8 ms; the full WASM call is about 12.5 ms. These are
different execution boundaries and do not isolate transport versus arithmetic.
Before UI integration, use a worker, resolve explicit load/support semantics,
and profile arithmetic/compiler settings with before/after controls. Do not
run repeated maximum-size solves on the viewport thread.

Follow-up: `truss-wrench-loads-2026-09-20.md` documents explicit force/moment
assembly and a V8 tiering control. The 20-warmup measurements above capture an
earlier execution tier; they must not be treated as fully optimized steady-state
throughput or used to attribute the full native/WASM difference to transport.
