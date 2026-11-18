use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurveSeamIsolation, isolate_pcurve_line},
    },
};
const WORK: u64 = 1_000_000;
fn arc() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., 1., 2.],
        periodic: false,
    }
}

#[test]
fn isolates_an_oblique_rational_circle_tangency_exactly() {
    let report = isolate_pcurve_line(&arc(), [3., 4., -5.], 1e-8, WORK).unwrap();
    let spans = report
        .spans
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    let PcurveSeamIsolation::Discrete {
        at_start: false,
        at_end: false,
        roots,
    } = &spans[0].contact
    else {
        panic!("{:?}", spans[0])
    };
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].parameter, [4., 4.]);
    assert!(!roots[0].crosses);
}

#[test]
fn encloses_one_crossing_and_retains_both_endpoint_contacts() {
    let report = isolate_pcurve_line(&arc(), [1., -1., 0.], 1e-7, WORK).unwrap();
    let spans = report.spans.unwrap();
    let PcurveSeamIsolation::Discrete { roots, .. } = &spans[0].contact else {
        panic!()
    };
    assert_eq!(roots.len(), 1);
    let expected = 2. + 4. * (2_f64.sqrt() - 1.);
    let [lo, hi] = roots[0].parameter;
    assert!(lo < expected && expected < hi && hi - lo <= 1e-7 && roots[0].crosses);
    let report = isolate_pcurve_line(&arc(), [1., 1., -1.], 1e-7, WORK).unwrap();
    assert_eq!(
        report.spans.unwrap()[0].contact,
        PcurveSeamIsolation::Discrete {
            at_start: true,
            at_end: true,
            roots: vec![]
        }
    );
}

#[test]
fn reports_coincident_spans_and_budget_exhaustion() {
    let mut curve = arc();
    curve.control_points[1] = vec![0.5, 0.5];
    let report = isolate_pcurve_line(&curve, [1., 1., -1.], 1e-8, WORK).unwrap();
    assert_eq!(
        report.spans.unwrap()[0].contact,
        PcurveSeamIsolation::Coincident
    );
    let limited = isolate_pcurve_line(&arc(), [3., 4., -5.], 1e-8, 1).unwrap();
    assert!(limited.spans.is_none());
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
    assert!(limited.work_used <= 1);
}
