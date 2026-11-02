//! Rootless complex fibers must not obstruct a complete REAL domain census.
use brep_core::profile_certificate::{certify_span_intersections, verify_span_intersections};
use nurbs_core::curve::Curve;
fn pair() -> (Curve, Curve) {
    let c = |control_points: Vec<Vec<f64>>, weights| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points,
        weights,
        periodic: false,
    };
    (
        c(
            vec![vec![4., 0.], vec![5., 4.], vec![4., 8.]],
            vec![1., 0.7, 1.],
        ),
        c(
            vec![
                vec![4., 0.],
                vec![4.623874036455391, 2.4954961458215634],
                vec![4.334003755927576, 5.858517992989387],
            ],
            vec![1., 0.7890327060493011, 0.8747800728778061],
        ),
    )
}
#[test]
fn rounded_retained_rational_support_has_complete_endpoint_census() {
    let (a, b) = pair();
    let certificate = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
    let verified = verify_span_intersections(&a, &b, &certificate).unwrap();
    assert_eq!(verified.events().len(), 1);
    assert!(verified.overlaps().is_empty());
    assert_eq!(verified.events()[0], [[0., 0.], [0., 0.]]);
    // Removing a map for an actual real event cannot invoke rootless exclusion.
    let mut forged = certificate.clone();
    let factor = forged
        .pairs
        .iter_mut()
        .flat_map(|p| p.factors.iter_mut())
        .find(|f| !f.roots.is_empty())
        .unwrap();
    factor.parameter_map = None;
    factor.roots.clear();
    assert!(verify_span_intersections(&a, &b, &forged).is_err());
}
