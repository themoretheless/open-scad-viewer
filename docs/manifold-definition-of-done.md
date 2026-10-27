# Manifold operation Definition of Done

This is the single production-ready qualification checklist for every new or
materially changed manifold-family operation. It applies to native algorithms,
strict-manifold facades, raw ABI, generic value dispatch, TypeScript wrappers,
worker jobs, and user-facing commands.

A checkbox means **evidence exists and the relevant checks passed**, not merely
that code was written. Mark an item “not applicable” only with a short technical
reason; an integration layer that was not added must not be presented as
qualified. Never satisfy a required check by skipping, focusing, ignoring, or
allowing it to fail.

Use `manifold-extension-guide.md` for ownership and connection points. Record
the operation name, change/revision, test environment, artifact identity where
applicable, benchmark report, and unresolved limitations with the completed
checklist.

## 1. Purpose, contract, and ownership

- [ ] The operation has a precise public name, purpose, input/output types,
      units, coordinate conventions, and empty-result semantics.
- [ ] The documentation distinguishes strict-manifold mesh output from a
      best-effort mesh, diagnostic report, or 2D profile. A `slice`/`Rings`
      result is not advertised as a manifold solid.
- [ ] Exactly one canonical native implementation owns the algorithm. The
      lowest appropriate existing crate is used, dependency arrows do not point
      upward, and TypeScript contains no duplicate geometric/topological logic.
- [ ] Admission policy is explicit: reject non-manifold operands, accept a
      weaker input contract, or perform documented repair. The policy is not
      hidden inside a bridge or UI adapter.
- [ ] For a strict mesh-producing operation, the facade performs the required
      output check and returns a fresh report; residual non-manifold output is
      never published as success.
- [ ] Attribute behavior (UVs, face IDs, properties) is defined as preserved,
      transformed, explicitly rejected, or explicitly dropped.
- [ ] Compatibility with existing operation names, result fields, public Rust
      APIs, defaults, and migration requirements has been reviewed.

## 2. Numerical and deterministic behavior

- [ ] Every tolerance and weld epsilon has units, scope, valid range, default,
      and rationale. Relative versus absolute and scale-dependent behavior is
      documented; no claimed approximation/error bound is unsupported.
- [ ] Tangencies, coplanarity, degenerate triangles, duplicate vertices,
      disconnected components, empty inputs/results, and lower-dimensional
      inputs have defined behavior where applicable.
- [ ] Output ordering and validity are deterministic for the same input and
      options. Unordered hash iteration cannot change public output or reports.
- [ ] Finite-number, buffer-length, index-range, integer-overflow, and
      resource-limit validation happens before unsafe or topological use.
- [ ] Native and packaged-WASM behavior is compared for supported f64 and f32
      inputs. Any permitted floating-point difference is bounded and does not
      hide a topology or error-contract mismatch.
- [ ] Unsupported self-intersections, non-simple loops, coplanar contacts, or
      other geometric cases are reported as limitations/errors, not silently
      certified.

## 3. Errors, budgets, cancellation, and lifetimes

- [ ] Stable error codes distinguish invalid input, unsupported geometry,
      kernel/numeric failure, resource exhaustion, cancellation, and residual
      non-manifold output where those outcomes exist.
- [ ] Errors include actionable diagnostics without pretending that partial
      geometry is a successful result. Existing envelope and error mappings
      remain intact.
- [ ] Work, fragment, output-size, point/face, and other relevant budgets have
      explicit defaults and exhaustion tests. A budget error is not converted
      to empty success.
- [ ] Success and every early-return/error path release native handles, input
      allocations, result arrays, temporary state, and worker resources exactly
      once.
- [ ] Returned typed arrays are owned copies or safely transferred fresh
      response buffers. No WASM linear-memory view or scene-owned request
      buffer escapes with invalid lifetime/ownership.
- [ ] Cancellation semantics match the implementation. Pre/post-call lease
      checks are not described as cooperative native cancellation. Long native
      operations expose real checkpoints or carry an explicit non-cooperative
      limitation.
- [ ] Cancellation/budget tests prove that no partial result is committed and
      subsequent operations can still use the kernel/worker.

## 4. Correctness evidence

- [ ] Focused native unit tests cover representative valid inputs and every
      documented boundary/error class.
- [ ] Property tests state mathematical invariants that the algorithm actually
      guarantees: for example containment/extrema for hull, idempotence only
      where established for simplify/repair, or profile invariants for slice.
- [ ] Fixed seeds make failures reproducible; CI also exercises the established
      random-seed policy and prints the seed needed for reproduction.
- [ ] Fuzzing covers malformed transport and native inputs where applicable,
      including lengths, non-finite values, bad indices/tags, budgets, and
      handle cleanup. Crashes, traps, hangs, and invalid successful results are
      failures.
- [ ] Real-world defect/corpus fixtures and expected outcomes are added when
      the operation addresses imported or defective geometry.
- [ ] Differential tests against an independent implementation are added where
      a meaningful oracle exists. Tolerances and intentionally divergent cases
      are documented; an unavailable sidecar is not treated as passing coverage.
- [ ] Regression tests reproduce each fixed defect before relying on the fix.
- [ ] The operation composes correctly with existing check/repair/metrics/
      boolean workflows, and strict outputs pass the canonical checker rather
      than a second test-only topology implementation.

## 5. Performance and resource evidence

- [ ] A representative benchmark measures the layer being changed. Core
      microbenchmarks alone do not qualify ABI, TypeScript, or worker
      performance.
- [ ] Native performance changes have focused rbench coverage with output
      validation and the same profile/contract for baseline and current runs.
- [ ] Base/current measurements run sequentially on the same machine; medians
      use normalized `ns/op`, and the comparison JSON is retained. The standard
      regression gate is 25% unless a stricter reviewed policy applies.
- [ ] Input setup is separated from the measured lifecycle as intended, and
      output drop, copies, repair, and worker transfer are included or excluded
      explicitly rather than accidentally.
- [ ] Suspicious regressions are reproduced under stable load/thermal
      conditions. Baselines are never updated just to make an unexplained
      regression pass.
- [ ] Large-input behavior, memory ownership, and output growth are bounded by
      the contract; no quadratic behavior is introduced without analysis and
      an appropriate budget.

## 6. ABI, artifact, TypeScript, and worker integration

Complete the rows for every exposed layer; justify layers not exposed.

- [ ] **Generic dispatch:** the stable `op`/action is registered in exactly one
      existing router, request/response codecs are typed, error envelopes pass
      through unchanged, and `abi_request` operation 0 remains unchanged.
- [ ] **Raw ABI, if used:** export signatures, format/mode tags, result slots,
      element types, allocation ownership, and cleanup rules are updated
      together without renumbering existing contracts.
- [ ] **Artifact:** required exports, generated identity, package contents, and
      a real check/repair/metrics/boolean plus changed-operation smoke pass on
      the packaged `geometry-kernel.wasm`, not only native Rust.
- [ ] **TypeScript:** public types/defaults, typed-array conversion, output
      copies, error propagation, and cleanup are covered by direct integration
      tests against the packaged kernel.
- [ ] **Worker, if used:** `MainSolidJob`, `MainSolidResults`,
      `MAIN_SOLID_JOB_KINDS`, runtime dispatch, request-specific expectation,
      result validation, and transfer preparation are updated exhaustively.
- [ ] Real packaged-WASM worker tests cover success, malformed/failed results,
      error propagation, cancellation/budget behavior, and buffer ownership.
      Only newly created response buffers are transferred.
- [ ] A changed artifact is rebuilt reproducibly with the pinned toolchain and
      native `wasm-opt`; tracked WASM and generated identity files are updated
      together rather than leaving mixed revisions.

## 7. UI and application behavior, when exposed

- [ ] UI invokes only the public wrapper/worker path and performs no duplicate
      geometry calculation.
- [ ] Command enablement, selection, progress, cancellation, errors, residual
      diagnostics, and empty results match the operation contract.
- [ ] Undo/Redo and document/scene commit behavior are atomic; failed or
      cancelled work does not partially mutate the model.
- [ ] Mesh and 2D-profile results use the appropriate renderer/export path and
      do not display an unjustified watertight/manifold status.
- [ ] Integration tests exercise the actual command path and transferred data,
      not just a mocked UI callback.

## 8. Documentation, release, and final gate

- [ ] User documentation includes a minimal example, units, defaults,
      limitations, error behavior, and repair/tolerance side effects.
- [ ] Developer/extension documentation identifies the canonical native owner,
      facade, bridge route, TypeScript wrapper, worker path, and lifetime model.
- [ ] The error catalog and tolerance policy contain the new/changed behavior.
- [ ] Benchmark documentation explains any new case, measured lifecycle,
      profile, and baseline/report location.
- [ ] `manifold-changelog.md` records actual crate/API/ABI/TypeScript/worker
      changes, compatibility and migration impact, and qualification evidence.
      Versions and dates are not invented.
- [ ] No required check is skipped, focused, ignored, todo, or allowed to fail;
      unrelated pre-existing failures are reported separately, not claimed as
      green qualification.

From the repository root, the standard manifold release gate is:

```sh
# Native core/ops, raw ABI, property, and TypeScript/real-worker integration.
npm run test:manifold

# Packaged-artifact exports, identity, and functional smoke.
npm run test:manifold:artifact

# Architecture and required-check/no-skip policy.
npm run test:architecture
npm run audit:architecture

# After a current packaged build: native/WASM and f64/f32 differential matrix.
node --import tsx tools/manifold-bench/environment-matrix.mts

# Optimizer contract; provide the trusted native Binaryen 116 executable.
OSV_TEST_WASM_OPT_NATIVE=/absolute/path/to/wasm-opt npm run test:wasm-optimize

# Benchmark comparator tests and a same-machine base/current regression check.
cd tools/manifold-bench
npm run test:rbench
npm run bench:kernel:check -- \
  --baseline rbench-baseline.macos-aarch64.json --max-regression 0.25
```

Use the baseline appropriate to the actual OS/architecture and machine; the
filename above is an example, not a portable universal baseline. Run the
project's normal typecheck/build/UI suites when those layers changed. Attach
command results and reports to the change. A local command list does not prove
that hosted CI passed, and a previous green run does not qualify a changed
artifact.
