# RushGraph reference implementations

The application executes mechanical geometry through Rust `mechanical-core`.
`rushGraphGears-reference.ts` and `rushGraphPlanetary-reference.ts` retain the
independent TypeScript algorithms used by differential tests and benchmarks.
The loft, polygon validation and earlier named sketch solver and assembly references also live here. These modules are
reference code, not application services. Unit arithmetic, numeric type checks
and interval resolution also remain here as independent compiler oracles. The library structure verifier rejects
production TypeScript imports from benchmark and test directories.

`rushGraph-runtime-reference.ts` is the earlier TypeScript runtime oracle.
Its sampled spinner template is frozen in `planetarySpinnerTemplate-reference.ts`,
recovered from the parent of Git commit `a1e5ad14`. It does not represent the
current native spinner implementation.

Mechanical regression tests import the reference generators explicitly:

```sh
npx vitest run tests/rushGraphMechanical.test.ts
```

`check-rush-mechanical-parity.mts` compares reports, coordinates and topology
against a supplied native harness and writes fixtures owned by `mechanical-core`.
