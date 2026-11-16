use nurbs_core::{
    analysis::exact_affine_hull::{
        PlanarLiftRefusal, PlanarProjectionOutcome, PlanarSurfaceProjection, lift_planar_surface,
        project_planar_surface,
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

fn project(surface: &Surface) -> PlanarSurfaceProjection {
    let report = project_planar_surface(surface, WORK).unwrap();
    let PlanarProjectionOutcome::Projected(projection) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    projection
}

#[test]
fn roundtrips_a_rational_net_with_nonbinary_inverse_coefficients() {
    let source = face([[2., 0., 1.], [2., 3., 3.], [5., 0., 2.], [5., 3., 4.]]);
    let chart = project(&source);
    let report = lift_planar_surface(&chart, WORK).unwrap();
    let lifted = report
        .surface
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(lifted.control_points, source.control_points);
    assert_eq!(lifted.weights, source.weights);
    assert_eq!(lifted.knots_u, source.knots_u);
    assert_eq!(lifted.knots_v, source.knots_v);
}

#[test]
fn lifts_an_edited_face_when_the_omitted_axis_is_first() {
    let source = face([[7., 0., 0.], [8., 0., 2.], [7., 3., 0.], [8., 3., 2.]]);
    let mut chart = project(&source);
    chart.surface.control_points[1][1] = vec![5., 4., 0.];
    let report = lift_planar_surface(&chart, WORK).unwrap();
    let lifted = report
        .surface
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(lifted.control_points[1][1], [9., 5., 4.]);
    assert_eq!(lifted.control_points[0], source.control_points[0]);
    assert_eq!(lifted.weights, source.weights);
}

#[test]
fn refuses_nonbinary_edits_and_invalid_projection_axes() {
    let source = face([[2., 0., 1.], [2., 3., 3.], [5., 0., 2.], [5., 3., 4.]]);
    let mut chart = project(&source);
    chart.surface.control_points[1][1][0] = 6.;
    let report = lift_planar_surface(&chart, WORK).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(report.refusal, Some(PlanarLiftRefusal::NotRepresentable));
    let exhausted = lift_planar_surface(&chart, 1).unwrap();
    assert!(exhausted.surface.is_none());
    assert_eq!(exhausted.refusal, Some(PlanarLiftRefusal::ResourceLimit));
    assert!(exhausted.work_used <= 1);
    chart.omitted_axis = chart.retained_axes[0];
    assert!(lift_planar_surface(&chart, WORK).is_err());
}
