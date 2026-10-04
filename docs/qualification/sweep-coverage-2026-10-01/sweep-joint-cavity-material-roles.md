# Closed authored-frame + guide + affine material roles — 2026-10-04

Public TypeScript adapter invokes the packaged Rust/WASM volume audit. Fixture
matches the independently checked closed joint STEP case: periodic hollow circle
profiles, closed square path, varying authored frame axis, authored normal,
closed spatial orientation guide, affine axis scale [2,1,1]. All three applied
mode flags are required by the test.

The new `nurbsSweepEmbedding.test.ts` regression checks three independent cases:

1. Original geometry: boundary embedded, parents [null,0], material roles
   consistent, outer outward/inner inward, solidGeometryCertified true.
2. Reverse every inner face use: boundary and parents/roles still proven,
   inner now outward, solidGeometryCertified false. Both input models preserved.
3. Declare the original cavity shell as a second body's outer shell: boundary
   still embedded and parents still [null,0], material roles inconsistent,
   solidGeometryCertified false. Orientation remains null because the role gate
   refuses first; input preserved.

The third fixture changes body count and intentionally uses the supported legacy
geometry form without the obsolete one-body topologyIds table. Its first attempt
retained that table and was rejected before geometry with a table-count error;
that diagnostic is preserved in `sweep-joint-cavity-orientation-public-final.log`
and is not counted as the material-role proof.

Final public run: all six embedding-suite tests pass in 16.83 seconds, unchanged
timeouts (`sweep-joint-cavity-material-roles-public-final.log`). Earlier original
plus orientation-only run also passed six tests in 15.72 seconds
(`sweep-joint-cavity-orientation-public.log`). Packaged public WASM SHA256:
34a4e5be7bd2b9fb9b4156adb42da4c30400e444d50331103594b12d8b27e0e6.
Typecheck log: `sweep-joint-cavity-material-roles-types.log`; scoped diff-check.

This extends the affine native regression to the joint moving authored-axis/guide
mode through the real public WASM route. It does not prove arbitrary guides,
all closed paths, arbitrary nested regions, or a universal embedding guarantee.
No production algorithm or ABI change was necessary.

Budget follow-up on the same joint-mode geometry:

- Nesting geometry/domain limits 2/2 preserve positive embedded boundary but
  return null parents and roles, null orientations and false Solid certificate.
  Reported nesting cell counts remain within 2/2.
- Orientation geometry/domain limits 1/1 preserve positive boundary and complete
  nesting parents/roles, but at least one orientation remains null and Solid
  certificate remains false. Aggregate orientation counts remain within 1/1.
- Input equality is checked after all audits, including these exhausted cases.
- All six embedding-suite tests pass in 19.55 seconds with unchanged timeouts:
  `sweep-joint-cavity-budget-public.log`. Direct Vue typecheck evidence:
  `sweep-joint-cavity-budget-types.log`.

This validates fail-closed stage composition through the public WASM route for
the selected combined closed mode; it is not an all-mode completeness claim.
