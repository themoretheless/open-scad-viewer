# Original-source tooth instances and common side grids

The catalog remains 197/200. This is progress on NURBS-0088, not a complete wheel promotion.

`AxialSource` owns one of the original physical-Y root, working or tip definitions. `AxialInstance` retains that definition and an integer period. Its nominal angle is -2*pi*period/teeth with certified pi/trigonometric bounds; it does not replace axial phase with a rounded multiple of pi.

`regrid_axial_source` and `regrid_axial_instance` require complete source-domain break grids. `qualify_axial_instance` checks the actual supplied controls, original derivatives, error and rank. `place_axial_surfaces` produces candidates only; admission is mandatory afterwards. Canonical physical-Y controls remain exact at both axial end planes. The placement rotates derivative enclosures as well as positions.

`axial_side_network` constructs all periods and six ordered original side families, copies actual interface control rows in world coordinates, and then qualifies every modified grid against its original instance. The final crest-cylinder/first-descending-root interface is included. The constructor checks all interface rows after individual admissions. Explicit `reverify` freshly replays every original source and actual grid as well as the complete seam ledger. Limits: 12..256 teeth, at most 4096 patches per family, at most 16384 total patches; every requested domain must be complete. Unresolved geometry and insufficient resolution return errors.

`end_contours` extracts actual packed end rows without evaluation or fitting. Ascending families reverse poles, weights and negative-reversed knots exactly. These are planar cap proposals, not trim-region certificates. A complete B-rep still requires two genuinely planar admitted caps, manifold topology, orientation, ownership and invalid-input tests. Global intersection/solid and production qualification are not asserted.

Completed focused evidence before full-period assembly: instances-tests.log has three passing tests (six common stitched grids, nominal instance/world control admission, invalid census/period/budget). compile-regression.log preserves the prior physical-root regressions. Later full-period jobs and final compilation are recorded separately; only a terminal test summary constitutes a pass.
