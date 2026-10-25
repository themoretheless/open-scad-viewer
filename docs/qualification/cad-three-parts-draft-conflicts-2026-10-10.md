# Three-part draft conflict and history regression

`tests/solidDraftCadRetention.test.ts` now covers bracket, flange and enclosure
using the actual saved CAD documents, production transforms, DirectHistory and
canonical IndexedDB head transactions. Each specimen performs 20 translations,
20 competing writes, explicit conflict resolution, 20 rejected superseded writes,
20 Undo steps and 20 Redo steps. Every stored state is parsed and compared with
its exact serialized document; the winning complete geometry is preserved.

The focused CAD retention suite and both storage suites completed with exit
zero. Log: `/tmp/cad-three-parts-draft-conflicts-20261010.log`.

IndexedDB is supplied by fake-indexeddb. This tests production transaction and
geometry/history logic, but does not qualify browser process crashes, real
concurrent tabs, localStorage notifications or full new-interface command chains.
