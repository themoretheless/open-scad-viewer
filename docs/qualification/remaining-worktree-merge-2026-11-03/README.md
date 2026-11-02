# Remaining worktree integration — 2026-11-03

Local integration source: `b65295054851237eb42861686a88c4d524ae2e4d`.
Merged `codex/nurbs-rational-reduction-20261102` at `32e1a2269`, including
`codex/nurbs-plan-next-20261102` at `2658de895`, and concurrent main
`be3cefb85`. Both merges were conflict-free. Every existing local and
remote branch tip is reachable from this integrated main.

Active uncommitted Rust smoothness, curvature/contact work and the two
modified WASM files in plan-next were preserved separately. No worktree,
branch, stash or tag was deleted. No push was performed.

## Validation boundaries

- Architecture audit: passed, 54 pinned oversized files, zero layer violations.
- Architecture/partition/required-check tests: 10 passed.
- Forma UI: 11 passed. Combined Forma + direct UI: 367 passed, 4 failed.
- Vue and MCP TypeScript checks passed after attaching existing generated
  modules from the primary checkout. This is type checking, not a new WASM build.
- Direct UI failures: numerical-subtraction timeout, runout confirmation text,
  and two language variants of the runout control callback. Baseline runs on
  pre-integration `be3cefb85` also fail these selected scenarios, but earlier
  with missing buttons. This does NOT establish equivalence of failure causes.
- Qualification drift fails on both snapshots: 1,219 missing source paths,
  8,668 records. Archived reports retain the exact missing/mismatched entries.
- Initial native run: 2,252 library tests passed, one existing ignored;
  27 integration contracts passed. It started before concurrent main was
  incorporated, so the final unchanged integration source was retested.

The UI and qualification-drift gates remain open. No full application,
WASM freshness, CI or G1/G2 qualification is claimed by this merge.

## Terminal native result

The rerun on unchanged `b65295054` completed successfully:
2,252 library tests passed, zero failed, one existing ignored; all 27
integration contracts passed (8 conics, 8 quadrics, 2 tensor-denominator,
9 rational-surface-factor). The exact terminal log is `native-final.log.gz`.
Concurrent main advanced independently during this run; this result is
specific to the stated integration source, not subsequent source changes.
