# Face-selection topology failure and Retry

`prepareTopology` previously retained a settled request returning null after worker failure. Subsequent face picks on the same immutable body reused that request and could not retry preparation.

On a current failure the UI now clears the cached request and body, shows a localized message naming the body, and exposes Retry face selection. Cancellation or a new request clears the retry state. A late failure from an old selection cannot change the current error or retry controls. Retry preserves document geometry and history.

Selected component regressions: four passed, 338 unrelated tests skipped (342 total), 0.696 seconds of tests. Includes Russian and English same-body retry, and late successful/failed replies after selection change. Log `/tmp/topology-retry-and-late-ui-20261010.log`. Vue type checking passed (`/tmp/topology-retry-typecheck-20261010.log`). `git diff --check` passed.

This is component-level evidence with mocked worker transport, not all-command browser acceptance or proof of browser renderer crash recovery.

Full current DirectModeler component suite terminated successfully: 342 passed, zero failures, 122.17 seconds of tests, 126.33 seconds total (`/tmp/topology-retry-full-ui-20261010.log`). This is broader regression evidence, not execution of every command in a real browser.

## Actual browser Retry

On isolated origin `127.0.0.1:5297`, `tests/fixtures/topology-retry-browser.html` mounts production DirectModeler with one 20 mm cube. Test-only Worker instrumentation suppresses exactly the first topology request and dispatches an error event; subsequent requests use the actual production worker and WASM. This is injected worker failure, not an OS/browser process crash.

Observed first request count one, localized body-specific error and Retry button. Clicking Retry increments count to two, clears the error, and completes through the real worker. Face-selection mode then exposes six cube faces; selecting face 2 reports centroid 10,10,20 mm. Undo/Redo remain disabled. Console errors are empty. Screenshots: `/tmp/topology-browser-failure-20261010.png`, `/tmp/topology-browser-recovered-20261010.png`. Temporary tab and isolated server were closed; the user's 5195 tab/draft was not changed.
