use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{
        PcurveEdgeAgreement, PcurveOrientationOutcome, Refusal, compare_pcurve_seam_orientations,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn trace([a, b]: [f64; 2], u: [f64; 2], weights: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![a, a, b, b],
        control_points: u.into_iter().map(|x| vec![x, 0.]).collect(),
        weights,
        periodic: false,
    }
}

#[test]
fn detects_reversed_rational_traces_with_different_clocks() {
    let patch = face();
    let left = trace([3., 7.], [0., 1.], vec![1., 2.]);
    let right = trace([10., 18.], [1., 0.], vec![2., 1.]);
    let report = compare_pcurve_seam_orientations((&patch, &left), (&patch, &right), WORK).unwrap();
    assert_eq!(
        report.forward,
        PcurveOrientationOutcome::Compared(PcurveEdgeAgreement::Different {
            parameter: [3., 7.],
            coordinate: 0
        })
    );
    assert_eq!(
        report.reversed,
        PcurveOrientationOutcome::Compared(PcurveEdgeAgreement::Equal)
    );
    assert!(report.work_used <= WORK);
}

#[test]
fn retains_both_valid_orientations_for_a_constant_trace() {
    let patch = face();
    let left = trace([0., 1.], [0.5, 0.5], vec![1.; 2]);
    let right = trace([-2., 6.], [0.5, 0.5], vec![2., 1.]);
    let report = compare_pcurve_seam_orientations((&patch, &left), (&patch, &right), WORK).unwrap();
    assert_eq!(
        report.forward,
        PcurveOrientationOutcome::Compared(PcurveEdgeAgreement::Equal)
    );
    assert_eq!(report.forward, report.reversed);
}

#[test]
fn distinguishes_nonrepresentable_clocks_and_exhaustion_from_a_mismatch() {
    let patch = face();
    let left = trace([0., 1.], [0., 1.], vec![1.; 2]);
    let mut right = trace([0., 3.], [0., 1.], vec![1.; 2]);
    right.knots.insert(2, 1.);
    right.control_points.insert(1, vec![0.25, 0.]);
    right.weights.insert(1, 1.);
    let report = compare_pcurve_seam_orientations((&patch, &left), (&patch, &right), WORK).unwrap();
    assert_eq!(
        report.forward,
        PcurveOrientationOutcome::ClockRefused(Refusal::NotRepresentable)
    );
    assert_eq!(report.forward, report.reversed);
    let limited = compare_pcurve_seam_orientations((&patch, &left), (&patch, &right), 1).unwrap();
    assert_eq!(
        limited.forward,
        PcurveOrientationOutcome::ClockRefused(Refusal::ResourceLimit)
    );
    assert_eq!(limited.forward, limited.reversed);
    assert!(limited.work_used <= 1);
}
