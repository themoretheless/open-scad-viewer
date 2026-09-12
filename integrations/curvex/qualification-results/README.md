# Frozen Curvex qualification evidence

`summary.json` is the compact outcome. `report.json` retains the actual commands,
exit codes, test names and source hashes from the final isolated run. Original
absolute paths are preserved for provenance; copies of the referenced command
logs are in `logs/`. `sha256.json` covers the saved artifacts.

`curvex-migration.patch` includes the application source, dependency manifest
and resulting lockfile changes. Recreate it for another local path using
`../prepare.py`; its default library dependency path is the current checkout's
absolute path. The original Curvex was not modified.

The original and migrated release renderer captures are retained as JSON.
`render-comparison.json` contains coverage/cache/color metrics, and
`render-contact-sheet.png` is the visually inspected comparison. Nonlinear
gradient differences are compared against analytic color, because the original
boundary-only sampling was dependent on triangulation diagonals.

Performance results live in `../benchmark-results/`; they distinguish actual
Boolean application operations, uncached tessellation, single-shape frame
observations, and original drag/cache interaction workloads. See the replacement
contract for the measured costs and limits. `final-artifact-checks.json` records
the final source/hash audit and read-only patch application check.
