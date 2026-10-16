# Current local runtime checks

The current public a9acf6ee...0c2771f WASM and final rebuilt distribution pass
11 Chromium native protocol checks plus four Solid UI scenarios at widths
1440 and 600. Each UI case checks native body evidence, build cancellation,
Solid cancellation, source supersession, publication, visible drawing,
persistence/restoration, refusal preserving the previous body and recovery.
Four captured views were inspected: geometry and controls are visible at both
sizes. Captures precede the persistence wait and retain a transient storage
warning; the subsequent saved-state and restored-body assertions pass.
This finite browser matrix is not full semantic G1 qualification.

The exact native catalog has 5979 cases: 44 serial recognition plus 5935
remaining, with no omissions or overlap. The first opt-level=1 remaining run
stopped after 2049 passed, two MLX failures and three ignored because its
sandbox could not access Metal. That failed log is preserved. All twelve MLX
cases subsequently pass with host device access. A full remaining retry runs
with the same source/partition and host access; no full-workspace success is
claimed before its final receipt. The 44 recognition cases already passed.

An independent two-test compiler probe confirms that test optimization keeps
both debug assertions and overflow checks. Production release/WASM profiles,
geometric tolerances, proof budgets, individual deadlines and all cases are
unchanged. Hosted CI for this local successor has not run yet.
