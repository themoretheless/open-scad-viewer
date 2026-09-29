# Solid input and theme matrix

Eight cases passed in Chromium 151.0.7922.34: mouse activation and keyboard Enter activation in dark, light, Nord and Solarized themes. Each case imports 2×3×4 and 7×3×4 CAD boxes, downloads JSON/Blender/STEP, verifies reload and stable identity, scales the source by 2 in Properties, and downloads its edited STEP. OpenCascade independently validates all three STEP files per case, including 14×6×8 mm and 672 mm³ after editing.

```sh
node --import tsx scripts/export-blender-qualification.mts /tmp/solid-blender-qualification
node scripts/check-solid-interaction-matrix.mjs /tmp/solid-blender-qualification /tmp/solid-interaction-matrix /tmp/cad-roadmap-ocp/bin/python
```

`results.json` contains all eight browser reports and independent OCCT measurements. The Python environment is described in `../step/README.md`.

Scope: command activation and numeric field entry. Playwright locators focus targets; full sequential Tab navigation, pointer viewport manipulation, file chooser keyboard navigation, visual contrast and other browser engines are not qualified by this test. File input assignment prepares the fixture in both modes. This is partial P0 UI acceptance, not completion of the entire keyboard workflow requirement.


## Sequential Tab qualification

`tab-results.json` records the repeated eight-case matrix after replacing locator-driven focus with actual `page.keyboard.press('Tab')` traversal and `Enter` activation. The scale field is reached through Tab and edited with keyboard selection and a digit key. All eight cases passed, with 24 independently validated STEP files. Keyboard cases used 765 Tab presses over the complete repeated scenario; this establishes reachability, not efficient navigation.

Theme selection and file-input fixture assignment remain test setup. Native operating-system file chooser navigation, viewport manipulation, contrast and other browser engines remain unqualified. The earlier `results.json` retains the initial locator-focus evidence for comparison.

## File-menu Escape and focus return

The Solid file menu now handles Escape locally, closes and focuses its summary. It stops propagation so the same Escape does not cancel the modeling command behind the menu. `escape-focus.json` records a Chromium keyboard run in Nord that checks the closed state and actual active element after each download. The same functional scenario now uses Escape to return, reducing Tab presses from 765 to 358. This is a measured route improvement, not a general keyboard efficiency score.
