# Independent instance editing — 2026-10-01

The Make independent command now uses the existing cancellable scene-edit worker and async history commit. The UI no longer calls detachSolidInstances(history.document) synchronously. Document, selection, panel and Esc cancellation follow the direct-transform generation checks.

Initial CPU Chromium qualification passed on 1000 linked instances: held detach worker terminated on Esc; unchanged cancelled document; original ID and independently transformed source mesh preserved; Undo/Redo exact; changing the source leaves detached geometry unchanged; final Undo returns the original compact document. One-sample timings include automation and durable saves: detach 2766.92 ms, source edit 6225.72 ms. These exceed responsive editing expectations; performance qualification stays open.

After that run, detach was changed to avoid the second whole-document clone and use the existing worker instance cache. Final qualification passed: 65 tests in four files, typecheck, production build, 137 distribution artifacts within budget, and the same browser scenario on 1000 instances. The final-browser.json report records one sample: detach 3074.45 ms, source edit 8100.14 ms, with maximum frame gaps 583.7 and 1158.4 ms respectively. No speed improvement is established by these two uncontrolled samples. Main-thread history validation, geometry serialization and rendering still require investigation. The before-cache report is historical evidence of the first implementation.

## Validated UI snapshot for worker edits

Numeric transform and instance detach now send snapDocument to the worker. This is the already validated committed document; postMessage snapshots it. The previous history.document read after asynchronous Undo/Redo rematerialized compact history and cloned it on the UI thread.

Both before/after CPU-profile browser runs passed cancellation, exact independent geometry, full Undo/Redo and independent-source-change assertions on 1000 instances. The before profile sampled about 717.6 ms inclusive through get currentDocument / document parsing. The after profile contains no get currentDocument path for this operation. It still includes about 175.5 ms in structuredClone under commitAsync. Profiles contain browser and worker wait time and do not prove overall speed qualification. The summary excludes profiled actions from ordinary timing percentiles by design.

Typecheck, production build and distribution verification passed after this change (7,153,884 asset bytes). The full P0–P3 objective remains open.

## Time-limited B-rep display scan

SolidDisplayQueue now yields after either 32 B-rep keys or 8 ms, preserving atomic prepared-key publication and generation cancellation. A deterministic test simulates expensive keys and cancels a three-body scan before worker dispatch. All 13 display queue tests passed. Typecheck, production build and size verification passed (7,153,952 asset bytes). CPU browser qualification on 1000 linked instances passed import cancellation, restoration cancellation/retry, Undo/Redo and exact final source/instance identity and placement export. This does not qualify a single expensive geometry key or total render latency.

The general CAD regression command now includes SolidGeometryDisplayQueue, scene-edit cache and the real postMessage worker boundary tests. Its final run is in progress.

The first expanded CAD regression run had 779 passing tests and one sphere display assertion failure. Its old oracle assumed segments=12 always fits the 4000-triangle refinement budget; the new exact rational sphere exceeds it. Production correctly retained the working mesh. The tests now separately verify refined geometry and picking at segments=8 within budget, and exact working geometry, null picking remap, normals and source immutability when segments=12 exceeds budget. All seven display preparation tests passed. The full expanded CAD regression is rerunning; not yet qualified as green.

Remaining synchronous P0 paths found in the UI audit: moveSelectionToGroup and addEmptyGroup both read history.document and use synchronous commit(next). Their metadata-only changes still rematerialize and validate the complete instance scene. These paths are the next worker migration target; they are not qualified as responsive by the current display and transform tests.

Expanded CAD regression completed: 781 tests passed across all 65 files, no failures or skips, 68.53 seconds. Evidence: full-regression-final.txt. This validates the current suite, not every open P0–P3 requirement.
