# Worker scene copy reduction — 2026-10-01

applySolidSceneEdit no longer clones the complete document before delegating transform, instance-transform and instance-create to helpers that already produce independent results. Mutating group, sketch, placement and loft branches retain their private clone. Detach retains its existing independent helper path.

33 tests passed across scene cache, instances and actual postMessage boundary; source immutability and exact linked geometry checks remain. Typecheck, production build and distribution budgets passed: 7,155,705 asset bytes.

The first CPU Chromium run completed edit repetitions and orbit, then failed its outliner selection assertion because the source-edit scenario left Properties active. See browser-first-failure.txt. The harness now opens Scene before orbit and retains incomplete operation/orbit measurements before later assertions. Picking and final exact export assertions remain required.

Retry session 57941 completed with exit 0: 1000 linked instances, three source-edit/Undo repetitions, orbit and 200 ms process/heap sampling. Exact final export, compact worker requests and selection after orbit passed. Measurements are in browser-measurements.json. Sampled memory cannot establish an instantaneous peak or absence of leaks. Full P0–P3 remains open.
