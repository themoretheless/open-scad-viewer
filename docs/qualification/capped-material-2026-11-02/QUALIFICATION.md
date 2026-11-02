# Native original two-cap material volume qualification

Implementation: `fbe0e84330341d5cd4c9af3d0fe1dfc8c4f7700a`.
All **6,592 Rust/Cargo/toolchain input files were unchanged** throughout both
commands, including compilation and complete test execution.

- Complete closed/capped family: **46 passed, 0 failed, 0 ignored**.
- Full brep-core library: **2069 passed, 0 failed, 5 ignored**.
- Formatting of the edited closed/capped modules and whitespace checks: passed.

The focused runner initially recorded a dynamic expected count (`null` in the
unaltered result.json). This was independently tightened against the selected
compiled test catalog: exactly 46 named tests and an exact 46/0/0 footer.
catalog-validation.json preserves that catalog and the compiled binary hash;
expected-tests.json contains the independently captured preflight inventory.
No zero-test or omitted-test result is credited.

The new end-to-end case admits genuinely different cap interiors, an outer
contour and a clockwise hole, different raw UV domains and rational gauges,
exact G2 wall joins mixed with explicit C0 corners, and explicit C0 cap/wall
joins. Both original caps, all 20 boundary bindings and the actual source sweep
are retained by the opaque owner. Original inputs remain unchanged.

Accepted work: 383 source/volume cells, 377483 exact work, 100 products;
90 material pairs, 70 material cells, 60 material domain cells. Exact replay of
all six measured budgets passes. Reducing each budget by one refuses without
exceeding that budget. A second audit cannot reset the first audit's ledger.
Closed traversal cannot acquire an open capped owner. A changed independent
boundary clock remains unsupported by this normalized-map factory.

The critical negative case proves both caps' ten individual world boundary
bindings, then refuses because their normalized material regions differ. Both
individual material reports are certified and no volume proof is issued. This
checks the actual missing premise, not merely malformed input or a sampled gap.

result.json records commands, exit codes and complete summaries. Full logs are
compressed losslessly. environment.json records the compiler and test settings;
SHA256SUMS binds all receipts. Source manifests cover tracked and untracked .rs,
Cargo.toml, Cargo.lock and Rust toolchain files, not the host's global Cargo config.

Scope is the exact immutable-profile interval source and modes accepted by the
native filled-volume proof. This does not qualify every spatial frame/cap
combination, moving profile boundaries, general loft/miter binding, smooth
cap/wall G1/G2, packaged WASM, retained Solid, independent STEP or remote CI.
The full loft/sweep goal remains open; this packet is a source-bound native step.
