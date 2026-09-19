# STEP V10 runner diagnostics and Solid UI restoration

The pushed commit `d890a06a` failed the STEP V10 workflow at
`browser-workbench-indexeddb`:
[run 35464231173](https://github.com/themoretheless/open-scad-viewer/actions/runs/35464231173).
The parent saved child stderr/stdout in a runner-local file but printed only
the step name and exit status. The workflow did not upload the file, so that
run does not identify the underlying browser error.

## Changes

- Extract the synchronous step runner into `scripts/qualification-step.mjs`.
  Successful log bytes and SHA-256 remain unchanged. Failures print a bounded
  16,384-character tail plus the exit/signal/spawn error and log location.
  Captured output remains on disk; buffer exhaustion is reported, not hidden.
- Provision the isolated locked Playwright package and Chromium in the STEP
  workflow, using the same package location as the existing G1 workflow.
  The previous cold-checkout recipe installed only root dependencies, although
  the browser script imports Playwright from the isolated package.
- Start Vite through its programmatic API after the parent has built the
  kernels. This avoids invoking `predev` and rebuilding all kernels a second
  time. Await listen/close, refuse an occupied port, and close the server even
  if browser startup or execution fails.
- Preserve browser error metadata and a failure screenshot. Upload logs and
  diagnostic JSON/PNG with `if: always()`, rather than losing failure evidence.
- Honor `STEP_V10_OUTPUT` for browser evidence, allowing local reproduction
  without rewriting historical qualification files. An explicit
  `CHROMIUM_EXECUTABLE` supports an installed local Chrome; CI uses the
  Playwright-provisioned browser by default.

## Verification and limits

Four Node unit tests pass: exact successful log/hash, nonzero exit with a
bounded console tail, missing executable with `ENOENT`, and buffer exhaustion
with `ENOBUFS`. Run `node --test tests/qualificationStep.test.mjs`; the workflow
also runs these tests. Both runner entrypoints pass `node --check`.

A real local Chrome/Vite run reached the application but timed out waiting
for the visible `CAD operations` button. Log:
`tmp/performance/step-v10-browser-local.log`. The current application opens
Solid by default; the old workbench action now exists in the source command
list rather than as the direct button expected by this harness. Restoring and
testing an accessible STEP import route remains necessary. Do not replace this
with a hidden-element click or direct service call and call it UI coverage.

A concurrent second check refused the occupied port and exited without opening
a browser or using the first server. Evidence:
`tmp/performance/step-v10-port-conflict/browser-failure.json`.

A second full failure reproduction saved `browser-failure.json` and an inspected
`browser-failure.png` under `tmp/performance/step-v10-browser-diagnostics/`.
It shows the default Solid workspace; the browser reported no page errors.
After the runner exited, a fresh TCP listener successfully bound port 4182,
confirming server cleanup on the failure path.

## Solid route follow-up

The earlier missing-route reproduction above is now resolved locally. Solid's
File menu imports STEP and exports the retained AP242 original. The shared
service validates the complete scene addition before saving the original;
storage failure leaves the scene untouched. Edits made during asynchronous
persistence are preserved, with an explicit message instead of replacement.
Undo affects the editable scene, not the independently saved original. Export
does not claim to incorporate subsequent Solid edits into the AP242 graph.

The parent now builds the production app and the browser uses Vite preview,
not the development server. Real file chooser and download actions verify
import, preservation of existing bodies, exact original bytes, invalid-input
refusal and persistence after reload. IndexedDB is read directly for evidence,
without invoking internal page services. Desktop and 390x844 mobile screenshots
were inspected; a narrow responsive header/menu fix keeps File accessible.
This scenario does not reimport the downloaded file into a fresh scene.

Local evidence: `tmp/performance/solid-step-browser-production-final/`.
The production browser check passed without page errors. UI/MCP typechecks and
the Vite build passed. Full regression: 3,235 passed, nine historical
engine-manifest/G0/G1 fingerprint failures across 335 files. The delivery gate
still rejects 5,776,821 bytes against its unchanged 5,600,000-byte budget.
Frozen qualification evidence was not refreshed. These local checks do not
establish a successful GitHub Actions run.
