# Complete native validation of the CI repair

Both scheduled partitions are complete: 44 serial recognition cases and
5935 remaining cases (5930 passed, five existing manual diagnostics or
benchmarks ignored). The exact target/name execution cover matches the
frozen 5979-case catalog without omissions, additions or duplicates.
The complete remaining retry exited zero in 569.874 seconds on this local
host, retaining its original geometry budgets, timeouts and assertions.
No prediction of hosted CI timing or cross-host success is inferred.

The original sandbox attempt remains in the preceding runtime record:
2049 passed, two MLX failures and three ignored before abort because Metal
was unavailable. All twelve MLX cases and the full remaining partition
subsequently pass with host device access. No test is removed or newly
ignored to make the retry pass. OwnRust75 source bytes remain unchanged.

The public WASM/browser/distribution and source-binding results are recorded
separately. This completes the local native gate for the CI repair only;
it does not execute semantic G1 work or qualify the newer interpolated-
carrier Rust source. Neither a new GitHub CI run nor publication is claimed.
