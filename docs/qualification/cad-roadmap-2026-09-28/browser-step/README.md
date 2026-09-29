# STEP downloaded from Solid after a UI edit

Reproduce after building the application:

```sh
node --import tsx scripts/export-blender-qualification.mts /tmp/solid-blender-qualification
node scripts/check-solid-download-browser.mjs /tmp/solid-blender-qualification
/tmp/cad-roadmap-ocp/bin/python scripts/verify-cad-step-occt.py /tmp/solid-blender-qualification
```

The last command uses the environment described in `../step/README.md` and `scripts/requirements-cad-step.txt`.

The Chromium test imports CAD JSON, downloads current STEP, reloads the document, scales the source body by 2 in the Solid Properties panel and downloads the edited STEP. OpenCascade validates the exact downloaded bytes against the SHA-256 manifest and independently measures bounds and volume. Expected edited dimensions: 14×6×8 mm, volume 672 mm³. All three files passed. Browser version and download count are in `browser-download-verification.json`; numerical evidence is in `occt-report.json`.
