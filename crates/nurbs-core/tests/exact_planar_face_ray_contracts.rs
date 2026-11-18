use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        planar_face_ray::{PlanarFaceRayOutcome, cast_projected_face_ray},
        planar_trim_projection::{PlanarTrimOutcome, project_coplanar_trim_loops},
    },
    curve::Curve,
    surface::Surface,
};
fn contour(a: f64, b: f64, reverse: bool) -> Vec<Curve> {
    let mut points = vec![vec![a, a], vec![b, a], vec![b, b], vec![a, b], vec![a, a]];
    if reverse {
        points.reverse();
    }
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: points,
        weights: vec![1.; 5],
        periodic: false,
    }]
}

fn face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn projected(reverse: bool) -> nurbs_core::analysis::planar_trim_projection::ProjectedTrimmedFace {
    let report = project_coplanar_trim_loops(
        &[face()],
        &[vec![
            contour(0., 1., reverse),
            contour(0.25, 0.75, !reverse),
        ]],
        1_000_000,
    )
    .unwrap();
    let PlanarTrimOutcome::Projected(mut charts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    charts.remove(0)
}

#[test]
fn certifies_a_hit_with_nonbinary_ray_parameter() {
    let face = projected(false);
    let report = cast_projected_face_ray(
        &face,
        [0.125, 0.125, 2.],
        [0., 0., -3.],
        1e-9,
        1024,
        1_000_000,
    )
    .unwrap();
    let PlanarFaceRayOutcome::Hit { intersection, .. } = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(intersection.parameter, Q::new(7.into(), 12.into()));
    assert_eq!(intersection.point[2], Q::new(1.into(), 4.into()));
}

#[test]
fn distinguishes_a_hole_from_an_uncertain_boundary_contact() {
    let face = projected(false);
    let miss = cast_projected_face_ray(&face, [0.5, 0.5, 2.], [0., 0., -1.], 1e-9, 1024, 1_000_000)
        .unwrap();
    assert!(matches!(
        miss.outcome,
        PlanarFaceRayOutcome::Miss {
            classification: Some(_),
            ..
        }
    ));
    let edge =
        cast_projected_face_ray(&face, [0., 0., 2.], [0., 0., -1.], 1e-9, 1024, 1_000_000).unwrap();
    assert!(matches!(
        edge.outcome,
        PlanarFaceRayOutcome::BoundaryUnresolved { .. }
    ));
}

#[test]
fn distinguishes_ray_direction_cases_and_arithmetic_exhaustion() {
    let face = projected(false);
    let test = |o, d, work| {
        cast_projected_face_ray(&face, o, d, 1e-9, 1024, work)
            .unwrap()
            .outcome
    };
    assert_eq!(
        test([0., 0., 1.], [1., 0., 1.], 1_000_000),
        PlanarFaceRayOutcome::Parallel
    );
    assert_eq!(
        test([0., 0., 0.], [1., 0., 1.], 1_000_000),
        PlanarFaceRayOutcome::Coplanar
    );
    assert_eq!(
        test([0., 0., 1.], [0., 0., 1.], 1_000_000),
        PlanarFaceRayOutcome::Behind {
            parameter: Q::from_integer((-1).into())
        }
    );
    assert_eq!(
        test([0., 0., 1.], [0., 0., 1.], 1),
        PlanarFaceRayOutcome::ResourceLimit
    );
}
