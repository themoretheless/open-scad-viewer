# Oracle capture deadline

CI run 35484235276 at commit 0ec4d1aa completed the independent two-process
capture test in 5204 ms on Node 20.19 and 5655 ms on Node 22. Both exceeded
Vitest's default 5000 ms test timeout. Each synchronous child process already
had a separate 60000 ms timeout; the outer deadline did not match that budget.

The test now explicitly allows 125000 ms: two existing child budgets plus
5000 ms for assertions and overhead. This increases the outer timeout; it is
not a performance improvement. Child limits, independent fresh processes,
complete-result comparisons and immutable baseline behavior are unchanged.

Red/green validation injected a 3000 ms startup delay into each capture child
using a temporary NODE_OPTIONS import, without modifying the producer:

- Before: default-timeout failure; actual test duration 7848 ms.
- After: passed with both delayed captures; test duration about 7940 ms.
- Without injection: all 44 tests in legacyDirectEvaluatorOracle.test.ts passed
  in 2.63 seconds total.

The delay hook is not checked in. Evidence logs are
`/private/tmp/osv-oracle-deadline-before.log` and
`/private/tmp/osv-oracle-deadline-after.log`.
The nine separate historical qualification-binding failures in the same CI
run remain unresolved; this change does not modify their artifacts or checks.
