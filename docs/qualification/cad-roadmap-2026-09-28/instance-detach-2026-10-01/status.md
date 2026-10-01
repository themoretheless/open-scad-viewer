# Independent instance editing — 2026-10-01

The Make independent command now uses the existing cancellable scene-edit worker and async history commit. The UI no longer calls detachSolidInstances(history.document) synchronously. Document, selection, panel and Esc cancellation follow the direct-transform generation checks.

Initial CPU Chromium qualification passed on 1000 linked instances: held detach worker terminated on Esc; unchanged cancelled document; original ID and independently transformed source mesh preserved; Undo/Redo exact; changing the source leaves detached geometry unchanged; final Undo returns the original compact document. One-sample timings include automation and durable saves: detach 2766.92 ms, source edit 6225.72 ms. These exceed responsive editing expectations; performance qualification stays open.

After that run, detach was changed to avoid the second whole-document clone and use the existing worker instance cache. Final qualification passed: 65 tests in four files, typecheck, production build, 137 distribution artifacts within budget, and the same browser scenario on 1000 instances. The final-browser.json report records one sample: detach 3074.45 ms, source edit 8100.14 ms, with maximum frame gaps 583.7 and 1158.4 ms respectively. No speed improvement is established by these two uncontrolled samples. Main-thread history validation, geometry serialization and rendering still require investigation. The before-cache report is historical evidence of the first implementation.

## Validated UI snapshot for worker edits

Numeric transform and instance detach now send snapDocument to the worker. This is the already validated committed document; postMessage snapshots it. The previous history.document read after asynchronous Undo/Redo rematerialized compact history and cloned it on the UI thread.

Both before/after CPU-profile browser runs passed cancellation, exact independent geometry, full Undo/Redo and independent-source-change assertions on 1000 instances. The before profile sampled about 717.6 ms inclusive through get currentDocument / document parsing. The after profile contains no get currentDocument path for this operation. It still includes about 175.5 ms in structuredClone under commitAsync. Profiles contain browser and worker wait time and do not prove overall speed qualification. The summary excludes profiled actions from ordinary timing percentiles by design.

Typecheck, production build and distribution verification passed after this change (7,153,884 asset bytes). The full P0–P3 objective remains open.
