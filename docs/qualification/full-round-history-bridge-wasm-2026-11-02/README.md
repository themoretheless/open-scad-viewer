# Full Round history bridge and actual WASM ABI

The public `cad_full_round_exact_history` router delegates creation, whole-document replay, source rebasing, authored-ID reselection, undo/redo and explicit prefix branches to Rust. Caller budgets are applied before restoring untrusted artifacts. Replies are bounded; TypeScript is transport only.

Three native bridge tests passed. Fourteen actual import-free release WASM ABI cases passed; `report.json` identifies the exact tested bytes and native fixture. Native and WASM history documents and actual work counters compare exactly. Hidden redo forgery, inconsistent operation tags, implicit edit-after-undo and caller budget violations refuse.

This is the admitted exact extrusion family. Installed-package delivery of the history endpoint, editor/scene history integration, general dependency history and original full Plasticity library parity remain open. The release candidate is being optimized separately; raw WASM ABI evidence does not qualify installed delivery. Plasticity runtime behavior equivalence has not been observed.

`vue-tsc --noEmit` passed on the current worktree.

Added direct untrusted-document restore transport. Two real-WASM TypeScript tests passed on the raw SHA-pinned candidate: immutable result/current cursor and undo/redo, hidden-redo forgery and byte-budget refusal. Ordinary tests target the installed package; until history delivery is installed they require the explicit qualified candidate. Final TypeScript check passed.

Frozen clean baseline: all 11 files unchanged. Global architecture gate remains red on six pre-existing concurrently edited files; no limits changed. See retained audit log.

## Installed delivery

Optimized SHA `e552c5d978af4690563c20211a9e40e745b87a832fe265129d5ff72ee5523fac`, 16,243,008 bytes within the unchanged 16 MiB limit. All 14 ABI cases passed again after optimization. Installed the standard bounded package; previous package preserved at `/tmp/rea-full-round-history-delivery-before`. All 14 ordinary history/snapshot/scene tests passed without candidate substitution, plus TypeScript checking. Real Chromium passed five owned Workers on the installed package, including cancellation/recovery and teardown. Browser qualification covers existing scene delivery, not editor history actions. Editor history integration remains open.
