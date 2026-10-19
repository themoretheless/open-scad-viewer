use super::*;
use crate as nurbs;
#[path = "../../../../../tests/support/general_frame_source.rs"]
mod source;
#[test]
fn smooth_authored_frame_cannot_certify_a_rank_two_filled_body() {
    let path = crate::primitives::line([0.; 3], [2., 0., 0.]).unwrap();
    let profiles = [crate::primitives::line([-0.25, -0.25, 0.], [0.25, -0.25, 0.]).unwrap()];
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.25, -0.25, 0.], vec![-0.25, 0.25, 0.]],
            vec![vec![0.25, -0.25, 0.], vec![0.25, 0.25, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = constant_vector_law([0., 0., 1.]).unwrap();
    let n = constant_vector_law([0., 1., 0.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 1., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&a, &n)
    .unwrap();
    let r = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            3000,
            1000000,
        )
        .unwrap();
    assert!(
        !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
        "{r:?}"
    );
    assert!(r.cells <= 3000 && r.exact_work <= 1000000);
}
#[test]
fn tilted_authored_spatial_arc_body_has_original_global_and_frame_g2_certificates() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (profiles, cap, _) = source::material();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&a, &n)
    .unwrap();
    let g2 = sweep.sweeps[0]
        .certify_closed_authored_frame_smoothness(2, 100000, 1000000)
        .unwrap();
    assert!(g2.closed_source_frame_smoothness_certified, "{g2:?}");
    let boundary = sweep
        .certify_original_closed_boundary_smoothness(
            &[4],
            super::super::super::OriginalBoundarySmoothnessLimits {
                order: 2,
                cells: 100000,
                exact_work: 1000000,
                products: 1000000,
            },
        )
        .unwrap();
    assert!(
        boundary.differential_compatibility_certified
            && boundary.frame_certified
            && boundary.laws_certified
            && boundary.profiles_certified,
        "{boundary:?}"
    );
    let r = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            1000000,
            1000000,
        )
        .unwrap();
    assert!(
        r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,
        "{r:?}"
    );
    assert!(r.cells > 0 && r.exact_work > 0);
    eprintln!("tilted authored original global atlas: {r:?}; frame={g2:?}");
}
