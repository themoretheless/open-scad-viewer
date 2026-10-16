# Rust module layout for the fibered-source checkpoint

The architecture gate enforces source-file limits and existing size pins.
The original-source checkpoint grew five files beyond those limits. This
change reorganizes the same Rust implementation and test cases; it does not
raise limits, pins, proof budgets or tolerances.

- `brep-core` keeps one explicit analytic re-export group and declares source
  modules alongside their related modules. The public names stay the same.
- `step_interchange_v3/units.rs` owns SI prefixes, bounded conversion chains
  and length-context resolution. The parent imports only the needed resolver;
  its tests retain private access to the conversion-chain helper.
- `closed/tests/material_holes.rs` groups hollow material and orientation
  cases; shared fixtures and simple source modes remain in the test facade.
- `closed/smooth/tests/ownership.rs` groups budget, exact binding, continuity
  and nonplanar-carrier regressions, sharing the existing fixtures.
- `surface_injectivity/tests.rs` holds the unchanged injection/normal/polar
  regressions. The production sufficient-condition implementation remains
  in `surface_injectivity.rs`.

Tests are preserved with their original leaf names and assertions. The moved
closed tests have new module prefixes, while shared fixture visibility stays
private to the existing owner. Complete native partition checks still cover
every case. No production geometry moves into a host language.

Historical full-qualification receipts remain bound to their original frozen
source revision. Focused current-layout checks and a fresh CI run are separate
validation; this structural change does not relabel historical fingerprints.
