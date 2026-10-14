# Fresh frozen-source artifact 9a3a20c6

Geometry WASM SHA-256: `9a3a20c60a86f63f5148687ad54e118624fe61cde304827c11fc334c56ddaef6`; 10,729,423 bytes. Frozen source: `/private/tmp/open-scad-viewer-sweep-current-source-2026-10-04-1791100402`. Compiler and optimizer provenance is recorded in `fresh-9a3-geometry-build-proof.json`.

- 40 frontend integration tests passed (8 suites), `/tmp/sweep-9a3-tests.log`.
- 33 baseline STEP and 10 reconstructed station STEP cases exported with artifact provenance.
- Independent OpenCascade verification passed for both matrices: `/tmp/sweep-9a3-full-occt.log` and `/tmp/sweep-9a3-reconstructed-occt.log`.
- Independent generator polynomial volume reference generated for guide/frame, closed and conic weight cases.
- Vite production build passed, `/tmp/sweep-9a3-vite.log`.
- Vue/TypeScript checking passed, `/tmp/sweep-9a3-types.log`.

These results qualify the frozen artifact and finite fixture matrices. They do not establish universal embedding, arbitrary rational joins, all-mode continuous bounds, or equivalence to a concurrently replaced primary-workspace artifact. Browser lifecycle verification and PR #30 CI were still running at this checkpoint.

## Native API publication checkpoint

PR #30 head `a08ca2dd9f6d41c8b93bf636d05ef319a333fde2` adds exact/projective strip JSON audits with independent Bernstein seam regularity. Local publication-worktree verification: 326 NURBS unit tests, two integration tests and one doctest passed (`/tmp/sweep-publication-exact-strip-full-tests.log`). This is a separately scoped native API delivery; its source graph is not asserted byte-equivalent to the frozen 9a3 artifact. New-head CI is pending.

## Browser qualification completed

The 9a3 artifact passed 74 cases and 852 assertions at viewport widths 1440 and 600. The exact artifact hash is bound in `fresh-9a3-ui-matrix.json`. Lifecycle cancellation uses held dispatch; this does not prove interruption latency inside a running native kernel. CPU fallback remains the renderer scope.

## Coefficient migration in progress

Native ruled-wall coefficient identity API published in PR #30 commit `853187064cde51e6ae23d823763aa7138ce5bbcf`; three native/JSON regressions passed in the publication worktree. A derived copy of the previously qualified frozen source with only this native operation added is building WASM at `/private/tmp/open-scad-viewer-sweep-current-source-2026-10-04-1791102401` (`/tmp/sweep-wall-coefficients-wasm-build.log`). TS remains on its existing implementation until that build and adapter verification complete. The current primary source compile had unrelated errors in editing/deform, editing/surface_fairing and construct/surfaces/wing_loft; these are not attributed to the new module.
