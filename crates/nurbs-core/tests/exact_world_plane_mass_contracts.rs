use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        exact_planar_face_mass::{ExactWorldPlaneMassOutcome, exact_projected_face_mass},
        planar_trim_projection::{
            PlanarTrimOutcome, ProjectedTrimmedFace, project_coplanar_trim_loops,
        },
    },
    curve::Curve,
    surface::Surface,
};
const WORK: u64 = 1_000_000;

fn chart() -> ProjectedTrimmedFace {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 2.], vec![0., 1., 3.]],
            vec![vec![1., 0., 3.], vec![1., 1., 4.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let report = project_coplanar_trim_loops(&[surface], &[vec![vec![curve]]], WORK).unwrap();
    let PlanarTrimOutcome::Projected(mut faces) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    faces.remove(0)
}

fn q(n: i64, d: i64) -> Q {
    Q::new(n.into(), d.into())
}

#[test]
fn preserves_exact_irrational_area_scale_and_world_centroid() {
    let report = exact_projected_face_mass(&chart(), WORK).unwrap();
    let ExactWorldPlaneMassOutcome::Mass(mass) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(mass.area_scale_squared, q(3, 1));
    assert_eq!(mass.signed_area, q(-1, 1));
    assert_eq!(mass.first_moments, [q(-1, 2), q(-1, 2), q(-3, 1)]);
    assert_eq!(mass.centroid, Some([q(1, 2), q(1, 2), q(3, 1)]));
    assert_eq!(mass.second_moments[2][2], q(-55, 6));
    assert_eq!(
        mass.centroidal_second_moments,
        Some([
            [q(-1, 12), q(0, 1), q(-1, 12)],
            [q(0, 1), q(-1, 12), q(-1, 12)],
            [q(-1, 12), q(-1, 12), q(-1, 6)],
        ])
    );
}

#[test]
fn keeps_boundary_work_and_world_transformation_under_one_budget() {
    let face = chart();
    let budget =
        nurbs_core::editing::exact_derivative::exact_planar_boundary_inertia(&face.loops, WORK)
            .unwrap()
            .work_used;
    let report = exact_projected_face_mass(&face, budget).unwrap();
    assert!(matches!(
        report.outcome,
        ExactWorldPlaneMassOutcome::TransformRefused(_)
    ));
    assert_eq!(report.work_used, budget);
    let report = exact_projected_face_mass(&face, 1).unwrap();
    assert!(matches!(
        report.outcome,
        ExactWorldPlaneMassOutcome::BoundaryRefused(_)
    ));
    assert!(report.work_used <= 1);
}
