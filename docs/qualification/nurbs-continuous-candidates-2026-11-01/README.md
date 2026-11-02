# Continuous boundary evidence for independent NURBS candidates

Status: partial foundation for general NURBS Boolean. **Not a certified general Boolean solver.** The common quadratic restriction is absent from this new candidate API, but global intersection coverage, UV material classification and ambient embedding remain open obligations.

## Implemented contract

`brep_core::tolerant_boolean::nurbs_boolean_continuous_candidate(a,b,operation,tolerance_mm,max_cells_per_model)` generates a numerical candidate on independent carriers through the existing tracer/UV assembler. Operations currently admit union, difference and intersection. The underlying numerical assembler still requires one body with one outer shell and no inner shells per operand; this API does not remove those limits. Both inputs retain their authored tolerances. The result uses exactly the explicitly requested tolerance (positive, no smaller than operand tolerances, at most 1e-2 mm).

The actual immutable input Models and assembled result are independently checked by `boundary_agreement::verify_with_exact`. Exact identities discharge supported uses; all remaining uses require conservative whole-interval curve/carrier agreement. The check includes actual CoedgeTrim ranges, periodic lifts, reversed uses, refitted closed branches and post-assembly pcurve endpoint changes. It covers each actual coedge use, not unused portions of trimmed edges. No sampled residual authorizes this API's boundary evidence. Mismatch and exhausted/inconclusive budgets refuse.

Each Model has a shared budget across all of its uses (1..1000000 interval cells) and a separate fixed exact-identity budget. Inputs and output have separate bounded ledgers. SHA-256 bindings include full serialized geometry, topology, topology IDs and tolerance. `verify_nurbs_boolean_boundary_candidate` recomputes evidence on the bound operands and candidate. The operation is a proposal label, not proved set semantics.

Evidence is explicitly typed `EvidenceScope::BoundaryAgreementOnly`; no topology/Boolean-set authority is granted. The legacy automatic numerical Boolean fallback is unchanged. Its sampled tolerance estimate must not be confused with this explicit continuously checked API. Neither API proves that all intersection components were found or that their material classification is correct.

## Findings that prevent a universal completion claim

The native typed SS report at `crates/nurbs-core/src/intersection/ss_intersection/report.rs` distinguishes geometric coverage from topology authority. General intersections retain `CoverageProof::Unproven`, unresolved obligations and `topology_proven=false`. Existing legacy numerical reduction/continuation candidates do not prove global 4D parameter-set coverage. Wiring them directly to a full Boolean certificate would be unsound.

Remaining: complete general branch coverage (including tangency/overlap/singular strata), owned branch-to-trim arrangements across seams, globally proved material classification, embedding and Boolean set equality. No universal NURBS/NURBS completion, exact arbitrary algebraic-edge representation, WASM/browser/dist qualification is claimed.

## Qualification

See adjacent logs and scoped source/binary hashes when complete. They do not freeze the transitive dirty checkout or qualify unrelated concurrent work.


## Periodic validation compatibility regression

The first neighboring boundary suite exposed an existing stale expectation: a test expected `Model::validate` to reject a literal whole-period UV lift, although the current validator explicitly calls `wrapped_surface_parameters`. The isolated pre-fix failure is preserved in `periodic-baseline-failure.log`. Only that test contract was updated: a whole-period lift must validate and remain unchanged by diagnostics; a fractional lift that changes the represented geometry must fail validation and continuous agreement. The existing validator and periodic wrapping implementation were not changed by this task.

The independent NURBS candidate tests also require newly retained faces to be authored, so an unchanged-source proposal cannot satisfy the positive crossing regression. Boundary evidence still does not establish full Boolean semantics.


## Final qualified results

Built native Rust tests with `CARGO_TARGET_DIR=/private/tmp/osv-boolean-20261101-target CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 cargo test --manifest-path crates/Cargo.toml -p brep-core --lib --no-run`. Copied the resulting executable before running the three disjoint suite filters; all final logs refer to that copy.

- `tolerant_boolean::`: **20 passed**, 0 failed, 15.89 s. Includes the three new independent-carrier, binding/budget and options regressions. The positive crossings must author additional retained faces; returning an unchanged operand does not pass.
- `boundary_agreement::`: **14 passed**, 0 failed, 3.14 s. Includes the updated whole-period/fractional-period compatibility regression.
- `operations::`: **25 passed**, 0 failed, 2.93 s.
- Total distinct tests: **59 passed**, 0 failed.
- Scoped SHA-256 source and copied executable verification passed; formatting checks for the tolerant Boolean files and scoped diff-whitespace checks passed.

Positive continuous-candidate regressions request **0.001 mm** with 200000 interval cells per Model. They test an off-axis sphere/cylinder cut and independently affine-transformed rational carriers (sheared cylinder and ellipsoid) for which common-quadratic dispatch is unavailable. The evidence verifier accepts the original result and rejects modified result geometry, a substituted operation label, a forged proof ledger, invalid options and an insufficient continuous budget.

This qualifies continuous boundary agreement of those generated candidates. It does not qualify missed-branch exclusion, UV material classification, solid embedding, Boolean set equality, xor, cavity/multiple-shell operands, or general NURBS Boolean completion. Standard automatic Boolean routing, WASM, browser and dist were not changed or qualified by this extension.
