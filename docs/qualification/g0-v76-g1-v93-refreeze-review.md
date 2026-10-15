# G0 v76 / G1 v93 re-freeze review

OwnRust60 binds the removal of unused GPU backend dependencies from the CPU-only osv-math manifest and the resulting Cargo.lock. Backend features remain in osv-math-compute. The archived c71fecea1cb784e8dc2da08909a1c7f73d9774a038ad30fe049029b4e1817892 WASM is retained from OwnRust59; this metadata refresh does not claim a new rebuild or a new STEP/browser execution.

OwnRust59 / G0 v75 / G1 v92 and their publication scripts remain immutable. The source bundle and dependency fingerprints bind the updated manifest, lockfile and runtime dependency identity. All 4740 clean semantic work units remain incomplete, with no prior result imported.
