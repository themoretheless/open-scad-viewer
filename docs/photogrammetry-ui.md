# Photo reconstruction: browser boundary

The photo panel is responsible for presentation and run lifetime. It does not implement reconstruction, normalize exports, or infer CAD validity itself.

| Module | Responsibility |
| --- | --- |
| `photoInput.ts` | Bounded EXIF hints, immutable photo entries, ownership of thumbnail URLs, atomic batch import, RGB decoding and cancellation |
| `photoPreview.ts` | One result-scoped coordinate transform and cache of render meshes; recovered camera framing |
| `photoExport.ts` | Full PLY export; a separately validated, measured and bounded OpenSCAD copy |
| `photoReport.ts` | Versioned metadata-only diagnostic report and human-readable camera refusal reasons |
| `photoWorkerProtocol.ts` | Shared request/event types for the independent worker |
| `photoReconstruction.ts` | Compatibility exports for existing call sites |

`PhotoCollection` releases an incomplete batch when any read fails. Disposing during a pending read prevents that read from creating or publishing another object URL. `decodePhoto` checks its abort signal after the asynchronous browser decoder returns and always closes its bitmap. Invalid focal edits do not mutate the last accepted input.

`PhotoPreview` is scoped to one result, so switching between points and surface reuses prepared geometry. It computes the coordinate system once for both rendering and source-camera framing. It never modifies the recovered coordinates or the exported arrays. Point materials are created directly: the previous full-cloud topology and BVH were immediately discarded before building the same colored batches.

PLY keeps every point, color and face. The document path checks solid compatibility and finite positive measured width; scaling uses the full observed result's X extent even when the inserted copy is simplified. The source budget is enforced before publishing text to the editor.

## Local performance check

Measured on 9 September 2026 with the existing Node/tsx runtime, identical generated input and code path. This is an adapter microbenchmark, not a browser rendering or photogrammetry speed claim.

Input: 20,000 points, positions `[sin(i*0.1)*10, cos(i*0.13)*12, i*0.002]`, colors `[i%256, (i*17)%256, (i*7)%256]`. Both versions displayed 5,000 points in 52 color batches, 20,000 vertices and 20,000 triangles. Five measured runs followed two warmups.

| Adapter | Samples, ms | Median, ms |
| --- | --- | --- |
| Previous `photoCloudMeshes` | 31.279, 28.555, 28.409, 34.788, 32.467 | 31.279 |
| Direct batches | 6.120, 5.054, 5.637, 4.535, 4.920 | 5.054 |

The measured adapter time decreased by about 6.2 times on this synthetic workload. A cached mode lookup does no geometry work. No general end-to-end speed improvement is inferred.

## Regression coverage

`tests/photoAdapters.test.ts` checks valid and out-of-bounds EXIF metadata, rollback and disposal during reads, invalid focal edits, bitmap cancellation/cleanup, unusable image aspect ratios, full versus simplified export scale, document limits/solid rejection, preview cache/color geometry, display sampling versus full PLY, and diagnostic export after failed reconstruction. Existing photogrammetry and camera-gesture tests remain applicable.

These are local unit tests of adapters and resource lifetime. Real camera registration and dense geometry quality require the separate fixed-pixel reconstruction corpus and browser run.

A real browser lifecycle check on the development app loaded six Monstree JPEGs, read the 28 mm EXIF equivalent, rejected a blank focal edit while preserving 28 mm, rolled back a deliberately failed two-file import with its temporary URL revoked, and cancelled while the browser decoder was pending. All six original photos remained; the bitmap closed; no reconstruction worker was launched after cancellation; no JavaScript exceptions occurred. The photo panel itself fit a 390-pixel viewport. The surrounding application toolbar overflowed to 420 pixels and is outside this module's changes.
