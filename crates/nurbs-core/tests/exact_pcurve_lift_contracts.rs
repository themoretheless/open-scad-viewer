use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurvePullbackRefusal, lift_pcurve_to_surface},
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn uv_line(a: [f64; 2], b: [f64; 2], weights: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights,
        periodic: false,
    }
}

fn ruled(u: &[f64], knots_u: Vec<f64>, degree_u: usize) -> Surface {
    Surface {
        degree_u,
        degree_v: 1,
        knots_u,
        knots_v: vec![0., 0., 4., 4.],
        control_points: u
            .iter()
            .map(|&u| vec![vec![u, 0., 0.], vec![u, 4., 4. * u]])
            .collect(),
        weights: vec![vec![1.; 2]; u.len()],
        periodic_u: false,
        periodic_v: false,
    }
}

fn assert_trace(surface: &Surface, original: &Curve, pieces: &[Curve]) {
    for piece in pieces {
        let [a, b] = piece.domain();
        for t in [a, (a + b) / 2., b] {
            let uv = original.evaluate(t).unwrap().point;
            let expected = surface.evaluate(uv[0], uv[1]).unwrap().point;
            let actual = piece.evaluate(t).unwrap().point;
            assert_eq!(actual.len(), 3);
            assert!(
                expected
                    .iter()
                    .zip(actual)
                    .all(|(x, y)| (x - y).abs() <= 1e-12)
            );
        }
    }
}

#[test]
fn lifts_an_exact_quadratic_space_trace_over_a_non_bezier_knot_line() {
    let surface = ruled(&[0., 1., 3., 4.], vec![0., 0., 0., 2., 4., 4., 4.], 2);
    let uv = uv_line([0., 0.], [4., 4.], vec![1.; 2]);
    let result = lift_pcurve_to_surface(&surface, &uv, WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].domain(), [3., 5.]);
    assert_eq!(pieces[1].domain(), [5., 7.]);
    assert_eq!(pieces[0].degree, 2);
    assert_eq!(pieces[0].evaluate(5.).unwrap().point, [2., 2., 4.]);
    assert_eq!(pieces[1].evaluate(5.).unwrap().point, [2., 2., 4.]);
    assert_trace(&surface, &uv, &pieces);
}

#[test]
fn composes_the_rational_weights_of_both_curve_and_surface() {
    let mut surface = ruled(&[0., 4.], vec![0., 0., 4., 4.], 1);
    surface.weights[1] = vec![2.; 2];
    let uv = uv_line([0., 1.], [4., 1.], vec![1., 2.]);
    let result = lift_pcurve_to_surface(&surface, &uv, WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].degree, 1);
    assert_eq!(pieces[0].weights[1] / pieces[0].weights[0], 4.);
    assert_trace(&surface, &uv, &pieces);
}

#[test]
fn lifts_a_coincident_curve_directly_on_the_shared_knot_line() {
    let surface = ruled(&[0., 2., 4.], vec![0., 0., 2., 4., 4.], 1);
    let uv = uv_line([2., 0.], [2., 4.], vec![1.; 2]);
    let result = lift_pcurve_to_surface(&surface, &uv, WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(pieces.len(), 1);
    assert_eq!(
        pieces[0].control_points,
        [vec![2., 0., 0.], vec![2., 4., 8.]]
    );
    assert_trace(&surface, &uv, &pieces);
}

#[test]
fn reports_nonrepresentable_grid_cuts_and_outside_curves_without_partial_output() {
    let surface = ruled(&[0., 2., 4.], vec![0., 0., 2., 4., 4.], 1);
    let uv = uv_line([0., 0.], [3., 4.], vec![1.; 2]);
    let result = lift_pcurve_to_surface(&surface, &uv, WORK).unwrap();
    assert!(result.pieces.is_none());
    assert_eq!(
        result.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable))
    );
    let outside = uv_line([2., 0.], [2., 5.], vec![1.; 2]);
    let result = lift_pcurve_to_surface(&surface, &outside, WORK).unwrap();
    assert!(result.pieces.is_none());
    assert_eq!(
        result.refusal,
        Some(PcurvePullbackRefusal::UnprovedChartDomain)
    );
    let limited = lift_pcurve_to_surface(&surface, &uv, 1).unwrap();
    assert!(limited.pieces.is_none());
    assert_eq!(
        limited.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
}
