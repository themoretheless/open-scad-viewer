# CAD draft retention: 2026-10-10

Test: `tests/solidDraftCadRetention.test.ts`.

The actual enclosure archive from `cad-ui-history-2026-10-09/enclosure.json`
is translated twenty times. At each step two writers publish different valid
CAD documents against the same IndexedDB revision. Exactly one succeeds and
one reports `DRAFT_CONFLICT`. Explicit conflict resolution publishes the
intended state. A superseded write is rejected and leaves that state unchanged.
Every recovered document is parsed and serialized and compared exactly with
the intended document, covering stored geometry and identities. All twenty
undo states are published and recovered; all twenty redo states are published
and the final state is recovered and compared exactly.

Command:

```sh
npx vitest run tests/solidDraftCadRetention.test.ts tests/solidDraftHeadStore.test.ts
```

Result: 2 files, 8 tests passed, 20.11 seconds. Log:
`/tmp/solid-draft-cad-retention-20261010.log`.

Scope: production document/history/storage code with fake-indexeddb. This does
not qualify physical browser crashes, independent browser tabs, a full browser
reload, STEP measurements, or all commands. Existing storage failure tests
cover abort, quota, corrupt records and wrong document keys separately.

## Real browser tabs

Qualification page: `tests/fixtures/cad-draft-tabs.html`. It warms the production
Rust WASM kernel before parsing the real enclosure document, and uses the
production history and IndexedDB implementation under an isolated key.

Two actual in-app browser tabs loaded revision
`b8f79b77-174e-43fa-bf7c-6247fe712788`. The first translated the enclosure
0.1 mm and published revision `fa1aaaf5-a6ae-4869-8569-7e0a3f8e9a4e`.
The second translated its stale document and received `DRAFT_CONFLICT`.
Reloading the second tab recovered the first tab's committed revision and
passed exact parse/serialize comparison. Browser error log after reload: empty.
Screenshot: `/tmp/cad-draft-tabs-recovered-20261010.png`.
Temporary tabs were closed; the user's model tab was preserved.

This qualifies the storage API in actual tabs and reload, not the full modeler
conflict presentation or physical renderer crash recovery. The test fixture's
initial missing-kernel warm-up was corrected before this sequence.

## Browser latency observation

The fixture now exposes separate timings using `performance.now()` around
production calls. One actual browser run on the enclosure measured:
parse plus history construction 67.5 ms; detached document copy 5.2 ms;
synchronous native transform (including bridge encoding and result handling)
121.5 ms; history commit (including authoritative validation) 84.8 ms;
serialization 12.1 ms; asynchronous strict IndexedDB write 17 ms.
Exact document roundtrip remained true. Screenshot:
`/tmp/cad-draft-latency-20261010.png`.
This is one observation under concurrent Rust/WASM work, not percentile or
isolated performance qualification. It identifies synchronous transform and
history validation as remaining UI latency work; no optimization is claimed.
