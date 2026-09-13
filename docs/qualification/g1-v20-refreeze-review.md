# G1 v20 shared-executor re-freeze

After v19 was frozen, the independent BRep work changed
`src/services/semanticProgramExecutor.ts` from 31,410 to 31,536 bytes. Its SHA-256
changed from `f4226f43656108fcc81d404e4991e792461204d6a686b0688da5237fdedbe5c7`
to `e9c816a2db623cedd303bc4cef089f2168dcc48e3e3b4cc58209317a3ba3a668`.
This changed the G1 candidate bundle and correctly failed the v19 current-source
checks. It requires a new candidate even though the existing G0 v3 bindings did
not change.

The source change exports and renames `checkControl` to
`checkSemanticExecutionControl` and updates the same internal call sites so the
native BRep host can reuse cancellation checks. The function body and branch
behavior remain unchanged. This explains the intentional source amendment; it
does not replace the required frozen-candidate clean runs.

V20 retains G0 v1–v3, G1 v1–v19 and the v19 source-snapshot status as exact
historical files. The old source snapshot describes its freeze; it is not a
claim that current files still have those bytes. Active source assertions move
to v20. No old artifact is overwritten.

Only plan/run/result identities, amendment lineage, binding hashes and the
version label in the clean-run protocol change. All **65 oracle cases, 18
comparator mutations and 4740 work units** remain required with identical
scope, boundary matrix, environments, seeds, clean-run counts, budgets, reset
policy and production cutover prohibition. V20 starts with **zero** completed
clean runs and work units. Prior results remain discovery-only,
`qualificationClaim` remains `none`, qualification approval remains
`not-approved`, and u07/G0 remain open.

Preparation is read-only:

```sh
node scripts/refresh-g1-plan.mjs --version 20 --check
```

After the source snapshot is coordinated and stable:

```sh
node scripts/refresh-g1-plan.mjs --version 20 --write \
  --recorded-at YYYY-MM-DD \
  --reason 'Re-freeze the shared semanticProgramExecutor.ts change after v19; retain G0 v3 and restart all required G1 clean work.'
```

The script creates only the v20 plan and its separate status, verifies pinned
historical hashes and unchanged G0/oracle/reference/schema/environment bindings,
and rejects source races, existing outputs or existing v20 execution records.
The two fragment helpers select v20 and new result paths; their execution,
classification and required-work rules are unchanged.

Future re-freezes require an explicit next-version amendment in
`G1_AMENDMENTS`, pinned previous plan/status hashes and a reviewed reason.
Shared file/snapshot/publication mechanics live in `qualificationRefreezeCore.mjs`.
There is no automatic latest-version selection, qualification run, evidence
reuse, counter carry-forward or approval in the refresh process.
