use nurbs_core::{
    analysis::exact_affine_hull::{
        SharedPlanarProjectionOutcome, SurfaceAffineHull, lift_planar_surface,
        project_coplanar_surfaces,
    },
    surface::Surface,
};
const WORK: u64 = 1_000_000;
fn face(points: [[f64; 3]; 4]) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![3., 3., 7., 7.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: points
            .chunks_exact(2)
            .map(|row| row.iter().map(|p| p.to_vec()).collect())
            .collect(),
        weights: vec![vec![1., 2.], vec![4., 0.5]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn projects_distinct_rational_faces_into_identical_world_coordinates() {
    let surfaces = [
        face([[2., 0., 1.], [2., 3., 3.], [5., 0., 2.], [5., 3., 4.]]),
        face([[5., 3., 4.], [5., 0., 2.], [8., 3., 5.], [8., 0., 3.]]),
    ];
    let report = project_coplanar_surfaces(&surfaces, WORK).unwrap();
    let SharedPlanarProjectionOutcome::Projected(charts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(charts.len(), 2);
    assert_eq!(charts[0].retained_axes, charts[1].retained_axes);
    assert_eq!(charts[0].offset, charts[1].offset);
    assert_eq!(charts[0].slopes, charts[1].slopes);
    assert_eq!(
        charts[0].surface.control_points[1][0],
        charts[1].surface.control_points[0][1]
    );
    for (projection, source) in charts.iter().zip(&surfaces) {
        let lifted = lift_planar_surface(projection, WORK)
            .unwrap()
            .surface
            .unwrap();
        assert_eq!(lifted.control_points, source.control_points);
        assert_eq!(lifted.weights, source.weights);
    }
}

#[test]
fn discovers_a_shared_plane_from_members_that_are_each_lines() {
    let a = face([[0., 0., 0.], [0., 0., 0.], [1., 0., 0.], [1., 0., 0.]]);
    let b = face([[0., 1., 0.], [0., 1., 0.], [1., 1., 0.], [1., 1., 0.]]);
    let report = project_coplanar_surfaces(&[a, b], WORK).unwrap();
    let SharedPlanarProjectionOutcome::Projected(charts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(charts.len(), 2);
    assert_eq!(charts[0].omitted_axis, 2);
    assert_eq!(charts[1].surface.control_points[1][0], [1., 1., 0.]);
}

#[test]
fn distinguishes_parallel_planes_with_a_tiny_gap_from_shared_budget_exhaustion() {
    let a = face([[0., 0., 0.], [0., 1., 0.], [1., 0., 0.], [1., 1., 0.]]);
    let mut b = a.clone();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = 1e-250;
    }
    let surfaces = [a, b];
    let report = project_coplanar_surfaces(&surfaces, WORK).unwrap();
    assert!(matches!(
        report.outcome,
        SharedPlanarProjectionOutcome::NotPlane(SurfaceAffineHull::Spatial(_))
    ));
    let limited = project_coplanar_surfaces(&surfaces, 1).unwrap();
    assert!(matches!(
        limited.outcome,
        SharedPlanarProjectionOutcome::ResourceLimit
    ));
    assert!(limited.work_used <= 1);
}
