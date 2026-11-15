use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurvePullbackRefusal, pullback_mobius_pcurve},
    },
};

const WORK: u64 = 1_000_000;
const DOMAIN: [[f64; 2]; 2] = [[0., 5.]; 2];
const CLOCKS: [f64; 2] = [2., 0.5];

fn polyline(points: [[f64; 2]; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 4., 5., 5.],
        control_points: points.map(Vec::from).to_vec(),
        weights: vec![1.; 3],
        periodic: false,
    }
}

fn forward(new: f64, [a, b]: [f64; 2], lambda: f64) -> f64 {
    let x = (new - a) / (b - a);
    a + (b - a) * lambda * x / (1. + (lambda - 1.) * x)
}

#[test]
fn transfers_both_uv_coordinates_and_preserves_the_original_parameter() {
    let source = polyline([[2., -3.], [7., 2.], [2., -3.]]);
    let domain = [[2., 7.], [-3., 2.]];
    let result = pullback_mobius_pcurve(&source, domain, CLOCKS, WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(pieces.len(), 2);
    for (index, piece) in pieces.iter().enumerate() {
        let start = 3. + index as f64;
        assert_eq!(piece.degree, 2);
        assert_eq!(
            piece.knots,
            [start, start, start, start + 1., start + 1., start + 1.]
        );
        for t in [start, start + 0.25, start + 0.5, start + 0.75, start + 1.] {
            let new = piece.evaluate(t).unwrap().point;
            let old = source.evaluate(t).unwrap().point;
            for axis in 0..2 {
                assert!((forward(new[axis], domain[axis], CLOCKS[axis]) - old[axis]).abs() < 1e-13);
            }
        }
    }
    assert_eq!(
        pieces[0].evaluate(4.).unwrap().point,
        pieces[1].evaluate(4.).unwrap().point
    );
}

#[test]
fn late_chart_failure_withholds_the_entire_curve() {
    let source = polyline([[0., 0.], [5., 5.], [6., 6.]]);
    let result = pullback_mobius_pcurve(&source, DOMAIN, CLOCKS, WORK).unwrap();
    assert!(result.pieces.is_none());
    assert_eq!(
        result.refusal,
        Some(PcurvePullbackRefusal::UnprovedChartDomain)
    );
}

#[test]
fn refuses_unrepresentable_poles_and_exhausted_work() {
    let source = polyline([[0., 0.], [1., 1.], [0., 0.]]);
    let result = pullback_mobius_pcurve(&source, [[0., 1.]; 2], CLOCKS, WORK).unwrap();
    assert!(result.pieces.is_none());
    assert_eq!(
        result.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable))
    );
    let limited = pullback_mobius_pcurve(&source, DOMAIN, CLOCKS, 1).unwrap();
    assert!(limited.pieces.is_none());
    assert_eq!(
        limited.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
    assert!(limited.work_used <= 1);
}

#[test]
fn keeps_a_rational_boundary_curve_on_its_chart_edge() {
    let mut source = polyline([[0., 5.], [5., 5.], [0., 5.]]);
    source.weights[1] = 2.;
    let result = pullback_mobius_pcurve(&source, DOMAIN, CLOCKS, WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    for (index, piece) in pieces.iter().enumerate() {
        assert_eq!(piece.degree, 1);
        assert_eq!(piece.weights[0], piece.weights[1]);
        let t = 3.5 + index as f64;
        let midpoint = piece.evaluate(t).unwrap().point;
        assert_eq!(midpoint, [2.5, 5.]);
        let old = source.evaluate(t).unwrap().point;
        assert!((forward(midpoint[0], DOMAIN[0], CLOCKS[0]) - old[0]).abs() < 1e-13);
    }
}
