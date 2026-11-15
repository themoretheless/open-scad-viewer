use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            BezierSpanLayout, PcurvePullbackRefusal, pullback_relayout_pcurve,
            relayout_bezier_surface,
        },
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;
const OLD: &[f64] = &[0., 5., 10.];
const LAYOUTS: [BezierSpanLayout<'_>; 2] = [
    BezierSpanLayout {
        breaks: &[10., 20., 40.],
        factors: &[2., 0.5],
    },
    BezierSpanLayout {
        breaks: &[-9., 1., 11.],
        factors: &[0.5, 2.],
    },
];

fn line(start: [f64; 2], end: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![start.to_vec(), end.to_vec()],
        weights: vec![1.; 2],
        periodic: false,
    }
}

fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 5., 10., 10.],
        knots_v: vec![0., 0., 5., 10., 10.],
        control_points: OLD
            .iter()
            .map(|&u| OLD.iter().map(|&v| vec![u, v, u * v]).collect())
            .collect(),
        weights: vec![vec![1., 2., 1.], vec![2., 4., 2.], vec![1., 2., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn transfers_a_curve_through_the_intersection_of_two_grid_seams() {
    let surface = patch();
    let mapped = relayout_bezier_surface(&surface, LAYOUTS, WORK)
        .unwrap()
        .geometry
        .unwrap();
    let source = line([0., 0.], [10., 10.]);
    let result = pullback_relayout_pcurve(&source, [OLD; 2], LAYOUTS, WORK).unwrap();
    let pieces = result
        .pieces
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].domain(), [2., 4.]);
    assert_eq!(pieces[1].domain(), [4., 6.]);
    for p in &pieces {
        assert_eq!(p.evaluate(4.).unwrap().point, [20., 1.]);
        let [a, b] = p.domain();
        for t in [a, (a + b) / 2., b] {
            let original = source.evaluate(t).unwrap().point;
            let new = p.evaluate(t).unwrap().point;
            let left = surface.evaluate(original[0], original[1]).unwrap().point;
            let right = mapped.evaluate(new[0], new[1]).unwrap().point;
            assert!(left.iter().zip(right).all(|(l, r)| (l - r).abs() < 1e-12));
        }
    }
}

#[test]
fn a_curve_on_one_grid_line_keeps_the_shared_new_boundary() {
    let source = line([5., 0.], [5., 10.]);
    let report = pullback_relayout_pcurve(&source, [OLD; 2], LAYOUTS, WORK).unwrap();
    let pieces = report
        .pieces
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(pieces.len(), 2);
    assert!(
        pieces
            .iter()
            .all(|p| p.control_points.iter().all(|point| point[0] == 20.))
    );
    assert_eq!(
        pieces[0].evaluate(4.).unwrap().point,
        pieces[1].evaluate(4.).unwrap().point
    );
}

#[test]
fn refuses_nonbinary_grid_contacts_and_out_of_grid_traces_atomically() {
    let source = line([0., 0.], [7., 0.]);
    let report = pullback_relayout_pcurve(&source, [OLD; 2], LAYOUTS, WORK).unwrap();
    assert!(report.pieces.is_none());
    assert_eq!(
        report.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::NotRepresentable))
    );
    let outside = line([5., 0.], [5., -10.]);
    let report = pullback_relayout_pcurve(&outside, [OLD; 2], LAYOUTS, WORK).unwrap();
    assert!(report.pieces.is_none());
    assert_eq!(
        report.refusal,
        Some(PcurvePullbackRefusal::UnprovedChartDomain)
    );
    let report = pullback_relayout_pcurve(&source, [OLD; 2], LAYOUTS, 1).unwrap();
    assert!(report.pieces.is_none());
    assert_eq!(
        report.refusal,
        Some(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
}
