# Real worker late profile-coverage reply

`tests/mainSolidWorkerRealBoundary.test.ts` now checks the production native
profile-bridge coverage controller with real worker threads and public WASM
SHA `41a8746ec39e719a4bf9866771fe65c4e3dd6f7e3162b531b9bea6431903884a`.
A real complete-exclusion reply is intercepted before delivery and its
positive result is asserted. The controller cancels that request, changes the
coverage cell budget to zero, and retries through a new worker. Delivering
the original positive reply during and after the replacement cannot overwrite
the current incomplete report or promote whole-thickness qualification.
The final result stays unchanged, pending=false, no error. Two workers start.

The selected test passes (1 passed, 33 skipped, 2.50 seconds test time). Log:
`/tmp/profile-coverage-real-late-reply-20261010.log`.
This is a real worker/controller boundary check, not browser mouse/keyboard
acceptance, general geometry admission, or qualification of the newer
fixed-shear/retained-cache WASM generations.

## Browser production panel

The new isolated fixture mounts the actual component with actual production
worker, without document or draft writes. Browser acceptance passes invalid
zero scale alert/focus, Enter launch, Escape cancel/run focus, mouse rerun,
15 unresolved pairs after 2,564 cells on curved profiles, first/last pair UV
coordinates, result invalidation after source replacement, Escape close/toggle
focus, and no captured console errors. The test tab was closed and the user's
model tab was untouched. Report:
`cad-command-coverage-global-root-2026-10-10/profile-coverage-browser-report.json`.
This does not qualify modeler surface selection, scene highlighting, injected
worker errors, or the full command matrix.

## Another active command

The production panel now disables all parameter inputs and its run button
when the host marks it inactive, displays RU/EN guidance to finish the current
command, and guards `start()` before dispatch. Existing cancellation/generation
handling is retained. Four focused panel/controller tests pass; typecheck
passes. Logs: `/tmp/profile-panel-inactive-command-20261010.log` and
`/tmp/profile-panel-inactive-command-typecheck-20261010.log`.
The production component was also checked in the isolated browser fixture by
changing its host active state: controls disable and guidance appears, then
controls return when active=true. Screenshot:
`/tmp/profile-panel-inactive-command-20261010.png`. This is a component host-state
check, not a full-modeler command-selection scenario.
