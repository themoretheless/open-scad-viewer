# Profile panel in-place source invalidation

A regression reproduced stale displayed coverage after changing a selected surface control-point coordinate in place. `options` intentionally used raw surfaces for worker cloning, so its deep controller watch could not observe nested reactive source edits. Replacing the complete surfaces array had worked, hiding this case. Before-fix regression failed at `profileBridgePanel.test.ts`'s stale status assertion; log `/tmp/profile-panel-inplace-source-before-20261010.log`.

The production panel now deeply watches selected surfaces synchronously and incorporates a source revision in its computed transport options. The existing generation cancellation clears both completed and pending reports. Raw transport and Rust geometry ownership remain unchanged.

Regression covers: an in-place control-point edit clears a completed report; an in-place rational weight edit cancels a pending request; that request's late success cannot restore the obsolete report. Existing keyboard focus, errors, Retry and controller stale-generation controls still pass. Two files, four tests passed, zero failed, 5.24 seconds total (`/tmp/profile-panel-inplace-source-after-20261010.log`). This is component/controller evidence with injected worker results, not a new real-WASM browser qualification or the complete P0 matrix.

## Real-worker browser check

On isolated origin 5297, the production panel displayed a real-worker planar result, then a fixture button changed one selected surface control-point coordinate in place by 0.01. The result status disappeared immediately. A subsequent inspection completed and displayed a fresh result; console errors were empty. Public WASM SHA remained `499506fdcb58168d991597288619e5cc90e11394cbc481d40f5f03f6b3b25da1` during this check. Screenshots: `/tmp/profile-inplace-cleared-20261010.png` and `/tmp/profile-inplace-recomputed-20261010.png`. The fixture button changes the isolated surface only, with no model/draft writes. This proves the panel invalidation path with the existing published worker; it does not qualify the later native endpoint-normal-sharing build still in progress.

## Adjacent diagnostic regression

Five diagnostic files passed together: 74 tests, zero failed, 13.49 seconds of tests, 22.30 seconds total. Covers expression controller cancellation/source identity, bounded radius/support/neighbor rendering, profile controller/panel behavior and profile worker transport. Log: `/tmp/diagnostic-source-invalidation-regression-20261010.log`. The expression panel retains reactive nested options through its spread; it does not use the profile panel's raw-surface mapping. No additional production change was made there. Bounded evidence was attached to the 97-command inventory without changing its unverified per-command cells or fully-qualified count.
