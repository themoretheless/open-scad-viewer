use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurveSeamSide, UvAxis, split_pcurve_at_seam},
    },
};

const WORK: u64 = 1_000_000;

fn bezier(u: &[f64], weights: Vec<f64>) -> Curve {
    Curve {
        degree: u.len() - 1,
        knots: [vec![2.; u.len()], vec![6.; u.len()]].concat(),
        control_points: u.iter().map(|&x| vec![x, 1.]).collect(),
        weights,
        periodic: false,
    }
}

#[test]
fn splits_at_a_tangency_and_a_crossing_while_retaining_absolute_parameters() {
    let source = bezier(&[-3., 7., -15., 27.], vec![1.; 4]);
    let result = split_pcurve_at_seam(&source, UvAxis::U, 0., WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(pieces.len(), 3);
    for (piece, ([a, b], side)) in pieces.iter().zip([
        ([2., 3.], PcurveSeamSide::Below),
        ([3., 4.], PcurveSeamSide::Below),
        ([4., 6.], PcurveSeamSide::Above),
    ]) {
        assert_eq!(piece.curve.domain(), [a, b]);
        assert_eq!(piece.side, side);
        for t in [a, (a + b) / 2., b] {
            let old = source.evaluate(t).unwrap().point;
            let new = piece.curve.evaluate(t).unwrap().point;
            assert!(old.iter().zip(new).all(|(x, y)| (x - y).abs() < 1e-13));
        }
    }
}

#[test]
fn retains_coincident_and_rational_nonsingular_pieces() {
    let source = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 3., 4., 4.],
        control_points: vec![vec![0., 1.], vec![0., 1.], vec![-2., 1.], vec![1., 1.]],
        weights: vec![1., 1., 1., 2.],
        periodic: false,
    };
    let report = split_pcurve_at_seam(&source, UvAxis::U, 0., WORK).unwrap();
    let pieces = report
        .pieces
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(pieces.len(), 4);
    assert_eq!(pieces[0].side, PcurveSeamSide::On);
    assert_eq!(pieces[1].side, PcurveSeamSide::Below);
    assert_eq!(pieces[2].curve.domain(), [3., 3.5]);
    assert_eq!(pieces[2].curve.evaluate(3.5).unwrap().point, [0., 1.]);
    assert_eq!(pieces[3].side, PcurveSeamSide::Above);
}

#[test]
fn refuses_nonbinary_parameters_or_derived_poles_without_partial_output() {
    let nonbinary = bezier(&[-1., 2.], vec![1.; 2]);
    let mut pole = bezier(&[-2., 1.], vec![1., 2.]);
    pole.control_points[0][1] = 0.;
    for source in [&nonbinary, &pole] {
        let report = split_pcurve_at_seam(source, UvAxis::U, 0., WORK).unwrap();
        assert!(report.pieces.is_none());
        assert_eq!(report.refusal, Some(Refusal::NotRepresentable));
    }
    let report = split_pcurve_at_seam(&pole, UvAxis::U, 0., 1).unwrap();
    assert!(report.pieces.is_none());
    assert_eq!(report.refusal, Some(Refusal::ResourceLimit));
    assert!(report.work_used <= 1);
}
