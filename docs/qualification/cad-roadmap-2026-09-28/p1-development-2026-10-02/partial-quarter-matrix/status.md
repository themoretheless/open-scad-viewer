# Quarter-rim STEP matrix

Eighteen native exports: outer radius 10/20/40 mm, proportional inner radius
2.5/5/10 mm and height 3/6/12 mm; each size has three blend radii and both
angular orientations. Exact parameters are recorded in manifest.json.

Reproduce with the partial-annular-matrix brep-core example, passing this output
directory. Then run scripts/verify-cad-partial-quarter-occt.py using Python with
cadquery-ocp 8.0.1.0.0 and the same directory.

All 18 exports import as one valid OCCT solid. Maximum independent numerical
volume discrepancy is 5.55e-7 mm³, below the unchanged 1e-6 mm³ threshold.
Maximum whole-body bounds discrepancy is 1.01e-7 mm, below 1e-6 mm.
The removed-material integral uses the authored radius law and independently
derived section moments; it does not read exported control points.

This finite matrix does not certify arbitrary parameter ranges, arbitrary
partial arc angles, full-domain continuity, self-intersection absence, source
identity preservation, or UI command behavior. Those requirements remain open.
