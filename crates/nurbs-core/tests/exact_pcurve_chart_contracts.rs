use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurvePullbackRefusal, certify_pcurve_chart_domain, pullback_mobius_pcurve,
        },
    },
};

const WORK: u64 = 1_000_000;
const DOMAIN: [[f64; 2]; 2] = [[0., 12.], [0., 1.]];

fn bezier(u: &[f64], weights: Vec<f64>) -> Curve {
    Curve {
        degree: u.len() - 1,
        knots: [vec![2.; u.len()], vec![5.; u.len()]].concat(),
        control_points: u.iter().map(|&x| vec![x, 0.5]).collect(),
        weights,
        periodic: false,
    }
}

#[test]
fn proves_rational_interior_tangency_and_allows_exact_pullback() {
    // Homogeneous U = (3t - 1)^2 and the positive weight is nonconstant.
    let source = bezier(&[1., -1., 4.], vec![1., 2., 1.]);
    let certificate = certify_pcurve_chart_domain(&source, DOMAIN, WORK).unwrap();
    assert_eq!(certificate.inside, Some(true));
    assert_eq!(certificate.refusal, None);
    let report = pullback_mobius_pcurve(&source, DOMAIN, [1.; 2], WORK).unwrap();
    let pieces = report
        .pieces
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].control_points, source.control_points);
    assert!(pieces[0].evaluate(3.).unwrap().point[0].abs() < 1e-14);
}

#[test]
fn endpoint_contacts_and_an_even_interior_root_do_not_imply_an_exit() {
    // U = 12 t (1 - t) (3t - 1)^2.
    let source = bezier(&[0., 3., -8., 12., 0.], vec![1.; 5]);
    let result = certify_pcurve_chart_domain(&source, DOMAIN, WORK).unwrap();
    assert_eq!(result.inside, Some(true));
}

#[test]
fn proves_two_interior_crossings_even_with_both_endpoints_inside() {
    // U = (3t - 1)^2 - 1/16.
    let source = bezier(&[0.9375, -2.0625, 3.9375], vec![1.; 3]);
    assert_eq!(
        certify_pcurve_chart_domain(&source, DOMAIN, WORK)
            .unwrap()
            .inside,
        Some(false)
    );
    let report = pullback_mobius_pcurve(&source, DOMAIN, [1.; 2], WORK).unwrap();
    assert_eq!(
        report.refusal,
        Some(PcurvePullbackRefusal::UnprovedChartDomain)
    );
    assert!(report.pieces.is_none());
}

#[test]
fn distinguishes_a_proved_exit_from_arithmetic_exhaustion() {
    let source = bezier(&[0., -0.5, 2.], vec![1.; 3]);
    assert_eq!(
        certify_pcurve_chart_domain(&source, DOMAIN, WORK)
            .unwrap()
            .inside,
        Some(false)
    );
    let exhausted = certify_pcurve_chart_domain(&source, DOMAIN, 1).unwrap();
    assert_eq!(exhausted.inside, None);
    assert_eq!(exhausted.refusal, Some(Refusal::ResourceLimit));
    assert!(exhausted.work_used <= 1);
}
