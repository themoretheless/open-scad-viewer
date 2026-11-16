use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
use nurbs_core::{
    analysis::exact_affine_hull::{
        PlanarProjectionOutcome, SurfaceAffineHull, project_planar_surface,
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
fn keeps_nonbinary_inverse_coefficients_exact_with_the_original_rational_basis() {
    let source = face([[2., 0., 1.], [2., 3., 3.], [5., 0., 2.], [5., 3., 4.]]);
    let result = project_planar_surface(&source, WORK).unwrap();
    let PlanarProjectionOutcome::Projected(chart) = result.outcome else {
        panic!("{:?}", result.outcome)
    };
    assert_eq!(chart.retained_axes, [0, 1]);
    assert_eq!(chart.omitted_axis, 2);
    assert_eq!(chart.offset, Q::new(1.into(), 3.into()));
    assert_eq!(
        chart.slopes,
        [Q::new(1.into(), 3.into()), Q::new(2.into(), 3.into())]
    );
    assert_eq!(chart.surface.knots_u, source.knots_u);
    assert_eq!(chart.surface.knots_v, source.knots_v);
    assert_eq!(chart.surface.weights, source.weights);
    for (old, new) in source
        .control_points
        .iter()
        .flatten()
        .zip(chart.surface.control_points.iter().flatten())
    {
        assert_eq!(*new, [old[0], old[1], 0.]);
        let x = Q::from_float(new[0]).unwrap();
        let y = Q::from_float(new[1]).unwrap();
        let z = &chart.offset + &chart.slopes[0] * x + &chart.slopes[1] * y;
        assert_eq!(z, Q::from_float(old[2]).unwrap());
    }
    for (u, v) in [(3., -1.), (4.25, 0.5), (6., 0.25), (7., 1.)] {
        let before = source.evaluate(u, v).unwrap().point;
        let xy = chart.surface.evaluate(u, v).unwrap().point;
        let z = chart.offset.to_f64().unwrap()
            + chart.slopes[0].to_f64().unwrap() * xy[0]
            + chart.slopes[1].to_f64().unwrap() * xy[1];
        assert!((before[2] - z).abs() < 1e-13);
    }
}

#[test]
fn can_drop_the_first_cartesian_axis() {
    let source = face([[7., 0., 0.], [8., 0., 2.], [7., 3., 0.], [8., 3., 2.]]);
    let result = project_planar_surface(&source, WORK).unwrap();
    let PlanarProjectionOutcome::Projected(chart) = result.outcome else {
        panic!("{:?}", result.outcome)
    };
    assert_eq!(chart.retained_axes, [1, 2]);
    assert_eq!(chart.omitted_axis, 0);
    assert_eq!(chart.surface.control_points[1][1], [3., 2., 0.]);
    assert_eq!(chart.offset, Q::from_integer(7.into()));
    assert_eq!(
        chart.slopes,
        [Q::from_integer(0.into()), Q::new(1.into(), 2.into())]
    );
}

#[test]
fn distinguishes_nonplanar_and_unproved_inputs() {
    let source = face([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 1e-250]]);
    assert!(matches!(
        project_planar_surface(&source, WORK).unwrap().outcome,
        PlanarProjectionOutcome::NotPlane(SurfaceAffineHull::Spatial(_))
    ));
    let exhausted = project_planar_surface(&source, 1).unwrap();
    assert!(matches!(
        exhausted.outcome,
        PlanarProjectionOutcome::ResourceLimit
    ));
    assert!(exhausted.work_used <= 1);
}
