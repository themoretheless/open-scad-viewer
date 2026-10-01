# GPU runtime qualification, 2026-10-02

## Findings

The original GPU failure is reproducible without CAD, geometry workers, shaders or Rust: the minimal script clears two canvas textures for 24 frames after staggered device creation. Chromium 151.0.7922.34 reports the google/swiftshader adapter, invalid texture validation, and device loss with `A valid external Instance reference no longer exists`. A one-device control also fails. Both scripts keep strict error/lost-device assertions; these are recorded failures, not passing gates.

Installed Chrome Canary 157.0.8081.0 reports apple/metal-3. The identical two-device control passed all 24 frames without validation errors or loss. Both browser version and backend differ: this evidence does not isolate a browser-version fix or prove compatibility with every Chrome version/driver. The phrase is also discussed by GPUWeb maintainers: https://github.com/gpuweb/gpuweb/discussions/4676 . Local reproduction, rather than that discussion, establishes this environment boundary.

Per-texture diagnostic scopes associate the actual failing color view with SolidGpuLayer, bgra8unorm, nonzero dimensions. A pagehide suspension experiment did not fix it and was reverted. Product rendering/math was not changed.

## CAD proof on Metal

Both mouse and actual Tab/Enter keyboard group scenarios passed on Canary/Metal with --require-gpu --trace-gpu. At all 16 document checkpoints per mode the Solid GPU layer remained active; captured and uncaptured validation failures remain failing assertions. Keyboard used 1478 Tab presses. Group creation, assignment, source/locked deletion refusal, hide/lock/isolation/context recovery, detachment, group deletion, exact Undo/Redo and document reload remain qualified for the two-body fixture. Both final workspace screenshots were inspected. This is not physical FPS or large-scene rendering evidence.

The qualifier no longer waits for CPU-only SVG polygons to establish body readiness: it waits for the scene object and primitive completion. Keyboard activation waits until the control is enabled before sequential Tab navigation. The local qualification HTTP server returns 204 for favicon.ico and records other failed resources; no general console/resource error filtering was added.

## Reproduction

Run `node scripts/check-webgpu-two-devices-browser.mjs OUTPUT` (or --single-device). To use the installed browser, set CHROMIUM_EXECUTABLE to its executable. Run `node scripts/check-solid-scene-state-browser.mjs OUTPUT --require-gpu --trace-gpu` and add --keyboard for keyboard. Diagnostic interception is opt-in and captured GPU errors are still failing gates.

## Remaining scope

SwiftShader/Chromium 151 GPU qualification remains failed; CPU fallback preserves geometry but is not native GPU proof. No minimum supported Chrome version is inferred. Full P0 command/error/latency coverage, general P1/P2 geometry, large-scene native GPU performance and P3 integrations remain open.
