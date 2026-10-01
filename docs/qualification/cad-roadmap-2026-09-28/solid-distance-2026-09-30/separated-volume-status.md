# Separated filled-volume distance qualification

The fourth native contract fixture uses cuboids [0,0,0]–[2,2,2] and [5,0.5,0.5]–[6,1.5,1.5]. Their independent analytic material-set distance is 3 mm. Native Rust and the existing WASM return separated-volumes, no material overlap, and interval [2.9999999999999982, 3.000000010000003] within the requested 1e-5 mm tolerance. Neither input changes. The reply validator rejects forged zero lower bounds, incomplete shell traversal, contact/containment claims and inconsistent convergence.

This qualifies a separated cuboid case. General curved volumes, original-surface result points and complete self-intersection diagnostics remain open.
