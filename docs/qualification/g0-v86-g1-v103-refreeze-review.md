# Exact observed Windows runner rollout

G1v102 at source cd81d41dc0a4285816100a4a2649a195be4ef1ea was stopped after a Windows preflight rejected an unrecorded hosted image. The fragment did not execute qualification work. Preserve G0v85/G1v102/OwnRust71 and the failed observation; import zero fragments into the successor.

Publish OwnRust72, G0v86, G1v103 and environment V38. V38 adds only the exactly observed Windows image pair, keeps the previous observed pair for the hosted rollout, and retains all other runtime/action/browser bindings. Unknown images continue to fail; positive and negative Windows image reconciliation are separately tested. All 4740 units, 132 fragments, seeds, numerical/work budgets and qualification boundaries remain unchanged. No test deadline or CI setting is changed in this update.

The Rust subtree c546568801f2c67f3ea20ed29afd0f32dbfcfe79 and all WASM source/build inputs remain byte-identical to cd81d41dc; retain its rebuilt primary artifact and recorded local evidence. No new geometry build or imported G1 credit is claimed. Formal qualification approval remains open.

The Node20 CI also found three stale transport-test expectations after the native correlated-minor proof strengthened admission. A quarter chart now must prove in exactly one cell. The three-cavity refusal cases use one less than the reported consumed linear-cell budget, retaining refusal and upper-budget assertions. Added only Rust-emitted counters to the transport interface; no geometry algorithm moved to TypeScript. Preserve the raw failed CI log.
