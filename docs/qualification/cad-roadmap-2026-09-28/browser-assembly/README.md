# STEP assembly downloaded in Chromium

Generate fixtures:

```sh
node --import tsx scripts/export-blender-qualification.mts /tmp/solid-svg-qualification
node --import tsx scripts/export-cad-assembly-fixture.mts /tmp/cad-browser-assembly
node scripts/check-solid-download-browser.mjs /tmp/solid-svg-qualification --assembly=/tmp/cad-browser-assembly
/tmp/cad-roadmap-ocp/bin/python scripts/verify-cad-assembly-occt.py /tmp/cad-browser-assembly
```

The browser script imports scene.json and downloads the current assembly via the Solid file menu. It replaces the qualification directory's scene.step with the actual download before independent OCCT checking. The four source parts are bracket, flange, enclosure and edited box. XCAF checks two groups, four valid shapes, bounds, volumes, names and colors. The report's stepDownloads=3 counts the preceding individual-body exports; assemblyDownloaded=true identifies the fourth STEP download.

Qualified: Chromium 151.0.7922.34, mouse interaction, OCCT 8.0.1.0.0. This does not qualify arbitrary assemblies, preserved linked-instance sharing, other browsers or source material parameters beyond color.
