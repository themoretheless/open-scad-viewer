# Main-source cancellation and reload qualification — 2026-10-04

Actual browser at localhost:5202. Initial document source was captured from the
visible textarea, with Auto checked, source drawer hidden, and Solid scene group
`model` containing one B-rep. Original rendered viewport had 2560 triangles.
Auto was temporarily unchecked for manual source replacement tests.

Verified actions:

- Wide: replace main source by the compact affine + authored frame + guide
  hollow quadratic fixture, click Build, wait for visible Cancel build, cancel.
- Wide: build again, wait for Cancel build, restore original source during the
  job. After completion the viewport still showed the original 2560 triangles,
  original volume/area and original certificate; superseded fixture not published.
- Wide: main-source To Solid from original source, wait for visible Cancel and
  click it. No new/replaced Solid body published.
- 720 x 900: fixture main-source Build, wait for visible Cancel build, cancel.
- 720 x 900: fixture main-source To Solid, wait for visible Cancel, restore
  original source during job. Cancel disappeared, no body publication.
- Restore Auto checked, hide source, reset viewport, reload. Original `model`
  body restored, scene count one, no selection. Opening source confirmed exact
  textarea equality with the initially captured source, then source hidden again.
  Temporary browser tab closed.

Screenshots: `sweep-main-build-cancel-wide.jpg`,
`sweep-main-build-cancel-narrow.jpg`, `sweep-main-reload-restored.jpg`.

One first attempt at To Solid did not produce a visible Cancel and instead left
the drawer hidden and Solid tool panel expanded. It is not counted as a pass.
The tool panel was restored and drawer reopened before the successful verified
retry. All counted cancellation actions require a currently visible cancellation
control, not inference from a preceding build request.

This slice does not prove successful/rejected main-source To Solid at both
sizes, narrow viewport source replacement, crash recovery, arbitrary source
replacement timings, or the whole geometric mode matrix. Group-source positive,
refusal and active cancellation evidence remains in the separate group reports.
