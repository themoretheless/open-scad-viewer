# Finite periodic profile-sweep STEP check

The fixture runs the committed Rust/WASM profile sweep with four linear square-profile edges, a radius-5 rational circular path and 17 stations. All four native deviation certificates accept and exact native seam predicates report G2. Rust normalizes each retained 1/16 V patch to [0,1]; the B-rep constructor, STEP export/import and surface evaluation also run in Rust. TypeScript only invokes these APIs and records fixture data.

Native STEP import and independent OCCT import/re-export preserve this one fixture. Both OCCT stages must have a valid 64-face single solid, 960 finite point/derivative samples and 12 closing-jet stations within the stated tolerances. Matching accounts for STEP face orientation by testing exact U reversal and its derivative signs, with unique face matches. Volume agreement is checked across OCCT round-trip; the ideal circular-ring volume is diagnostic, since the retained sweep is a cubic approximation.

This does not establish global regularity, embedding, shell containment, general moving-frame accuracy, all station counts, or a full semantic G1 matrix. The fixture producer and Rust normalization source are preserved as executed diagnostic sources, alongside both STEP byte streams (gzip) and their data. The checker accepts the fixture directory as its argument and needs cadquery-ocp 8.0.1.0.0.

To repeat the independent check, copy the JSON files into a temporary directory, decompress both `.step.gz` byte streams there, then run `verify-occt.py TEMPORARY_DIRECTORY` with the OCCT Python environment. The checker recomputes point, derivative, closing-jet, topology and volume assertions rather than trusting the archived report.
