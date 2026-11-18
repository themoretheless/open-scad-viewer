//! Boundary-only central graph for the trihedral graft regression.
//!
//! The graph is proposed from the current collars with the recipe recorded in
//! `docs/qualification/central-graph-2026-10-28/README.md`: degree 9, two
//! uniform spans with C2 junction knots and 5e-5 junction refinement, 33
//! fitting stations per side, regularization 1e-6, in the exact integer chart
//! `(x-y, x+y-2z, x+y+z)` normalized to an orthonormal frame. Building it here
//! keeps the candidate consistent with the strips and collars it is grafted
//! onto; a stored candidate drifts when those builders change. Every boundary
//! lift is still verified by the graft; the proposal grants no G2, material or
//! solid authority, and its known interior excursion remains.
use brep_core::region_replacement as r;

pub(crate) fn candidate(collars: &r::CornerCollars) -> r::CentralGraph {
    let (s2, s3, s6) = (2f64.sqrt(), 3f64.sqrt(), 6f64.sqrt());
    r::propose_central_graph(
        collars,
        r::GraphOptions {
            frame: [
                [1. / s2, -1. / s2, 0.],
                [1. / s6, 1. / s6, -2. / s6],
                [1. / s3, 1. / s3, 1. / s3],
            ],
            degree: 9,
            spans: 2,
            align_junctions: true,
            junction_refinement: 5e-5,
            stations_per_side: 33,
            regularization: 1e-6,
        },
    )
    .unwrap()
}
