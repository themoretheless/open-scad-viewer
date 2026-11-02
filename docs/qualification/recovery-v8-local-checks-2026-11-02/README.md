# Local recovery v8 checks, 2026-11-02

The recovery worktree bound by OwnRust82 passes both complete local suites:

- Frontend: 724 files, 6540 passed, 0 failed, 10 existing skipped tests.
- Rust workspace libraries: 52 targets, 6236 passed, 0 failed, 8 existing ignored tests.

The exact 7593-path source bundle was recomputed and matches
`6d502e699f0b3fc34e9f0dc9a7e6f546114e76d5333c531d303a4378364f6e0d`.
The generated and published geometry WASM both match
`442d2703d243fe29ed8434dcbce04e7cf06c2be36be149af07b51b00e45160f6`.
See `local-suites.json` for commands, source binding and log/report hashes.
Compressed complete outputs are retained alongside it.

The frontend uses the same resolved-project concurrency control as CI:
`VITEST_MAX_WORKERS=2`; a root CLI `--maxWorkers 2` alone is insufficient
for the inline projects with `extends:true`. Test cases, proof/work budgets,
fixtures and explicit test deadlines were preserved. `caffeinate -i` prevented
idle sleep only for the lifetime of each command. The earlier interrupted
run crossed host hibernation and is retained separately as diagnostic evidence.

This checkpoint concerns the recovery worktree, not the separate native-1000
feature worktree. Linux exact numeric golden reconciliation and current hosted
CI remain outstanding. It does not claim G0/G1 qualification, arbitrary geometry
coverage, completion of the 1000-capability plan, or a push to main.
