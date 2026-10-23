# Solid GPU recovery, 2026-10-02

## Behavior

A lost or failed Solid GPU layer leaves the CPU scene available and offers a localized Retry WebGPU button. A retry creates a fresh layer and restores the current scene/selection through the normal GPU body publication. Only the GPU failure notice is cleared after success. Duplicate fallback notices are suppressed. Pending initialization is disabled against double activation.

Closing Solid releases its GPU resources and invalidates pending initialization. Solid uses v-show, so reopening starts a new layer against the retained canvas after nextTick. An old result cannot replace the reopened layer or publish a stale failure notice. Failed initialization also exposes retry; it does not automatically spin a retry loop.

## Verification

- Final typecheck and Vite build passed.
- Final DirectModeler UI and SolidGpuFallback suites: 289 tests passed.
- Actual Chrome Canary 157.0.8081.0 / Apple Metal mouse and sequential Tab/Enter keyboard scenarios passed. Two deliberate device destructions exercise CPU fallback and retry.
- At first loss the whole downloaded document equals the baseline; after reconnection it still equals the baseline and the source remains selected.
- A second requestDevice is held after allocation. Solid closes and reopens before release. A fresh layer becomes active; releasing the stale device destroys it (lost reason destroyed), leaves the fresh layer active and preserves the exact whole document. Exactly three Solid canvas configurations are observed: initial, successful retry, reopen.
- The complete subsequent group/history/reload scenario passes in both modes, with 19 document downloads, 18 required-active-GPU checkpoints and 1664 keyboard Tab presses. Existing strict console/rendering failure gates remain intact.
- Keyboard CPU fallback and recovery screenshots were inspected. The final fallback view has one status row and retry button.
- Dist passes at 7,197,390 asset bytes, DirectModeler 381,431 bytes. Budgets move only for the measured 809-byte UI growth; no performance improvement is claimed.

## Limits

The device loss is controlled through actual GPUDevice.destroy; this does not prove recovery from every driver failure. The fixture is a box source and one linked instance, not a large-scene performance proof. Initial GPU acquisition refusal is handled by the same readiness branch but is not separately browser-qualified here. SwiftShader/Chromium 151 GPU qualification remains failed as recorded in the sibling runtime qualification; manual retry is not a browser backend fix. Full P0–P3 remains open.
